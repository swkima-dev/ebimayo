use std::collections::VecDeque;

use crate::channels::channel::{Channel, InboundEvent, IncomingMessage, StatusUpdate};
use crate::channels::cli::CliChannel;
use crate::tool::{glob::Glob, grep::Grep, read::Read};
use crate::{config, memory, util};
use anyhow::anyhow;
use rig::agent::Agent;
use rig::completion::CompletionModel;
use rig::{
    client::CompletionClient,
    completion::Completion,
    message::AssistantContent,
    providers::anthropic::{Client, completion::ANTHROPIC_VERSION_LATEST},
    tool::ToolSet,
};
use tokio::sync::mpsc;

pub async fn run() -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel(32);
    let tx_cli = tx.clone();

    println!("ebimayo!");

    let channel: Box<dyn Channel> = Box::new(CliChannel::new());
    channel.start(tx_cli).await;

    let api_key = config::load_anthropic_api_key();

    // magic number. refactor it later
    let mut main_memory = memory::ConversationMemory::new(700_000);
    let mut main_tools = ToolSet::default();
    main_tools.add_tool(Read);
    main_tools.add_tool(Grep);
    main_tools.add_tool(Glob);

    let client = Client::builder()
        .api_key(api_key)
        .anthropic_version(ANTHROPIC_VERSION_LATEST)
        .build()?;

    let agent = client
        .agent("claude-sonnet-4-6")
        .tool(Read)
        .tool(Grep)
        .tool(Glob)
        .build();
    while let Some(user_message) = rx.recv().await {
        match user_message {
            InboundEvent::UserMessage(message) => {
                agent_loop(
                    &agent,
                    &*channel,
                    &mut rx,
                    &mut main_memory,
                    &main_tools,
                    message,
                )
                .await?
            }
            InboundEvent::ApprovalResponse {
                request_id,
                approved,
            } => {
                if let Err(e) = channel
                    .send_status(StatusUpdate::InvalidApproval {
                        request_id,
                        approval: approved,
                    })
                    .await
                {
                    let _ = e;
                }
            }
        }
    }

    Ok(())
}

async fn agent_loop<M: CompletionModel>(
    agent: &Agent<M>,
    channel: &dyn Channel,
    rx: &mut mpsc::Receiver<InboundEvent>,
    memory: &mut memory::ConversationMemory,
    tools: &ToolSet,
    user_message: IncomingMessage,
) -> Result<(), anyhow::Error> {
    memory.push_user(user_message.content.as_str());

    const MAX_ITERATIONS: u16 = 1000;

    for _ in 0..MAX_ITERATIONS {
        let messages = memory.messages();
        let (prompt, history) = messages.split_last().expect("messages should not be empty");
        let response = agent
            .completion(prompt, history.to_vec())
            .await?
            .send()
            .await?;
        let response_text = util::extract_text(&response.choice);

        channel
            .respond(user_message.clone(), &response_text)
            .await
            .unwrap();

        memory.push_assistant(&response);

        let has_tool_calls = response
            .choice
            .iter()
            .any(|c| matches!(c, AssistantContent::ToolCall(_)));

        if !has_tool_calls {
            break;
        };

        let mut user_message_buf: VecDeque<String> = VecDeque::with_capacity(9);
        for content in response.choice.iter() {
            if let AssistantContent::ToolCall(tool_call) = content {
                let name = &tool_call.function.name;
                let args = &tool_call.function.arguments;

                channel
                    .send_status(StatusUpdate::ApprovalNeeded {
                        request_id: tool_call.id.clone(),
                        tool_name: name.to_string(),
                        args: args.to_string(),
                    })
                    .await
                    .unwrap();

                let (request_id, approved) = loop {
                    let event = rx.recv().await.ok_or_else(|| anyhow!("user judge error"))?;
                    match event {
                        InboundEvent::ApprovalResponse {
                            request_id,
                            approved,
                        } => break (request_id, approved),
                        InboundEvent::UserMessage(msg) => {
                            user_message_buf.push_back(msg.content);
                        }
                    }
                };

                if approved && request_id == tool_call.id {
                    let result = tools.call(name, args.to_string()).await?;
                    memory.push_tool_result(&tool_call.id, result);
                } else {
                    memory.push_tool_result(
                        &tool_call.id,
                        format!(
                            "Tool use was denied by user. Denied tool call: {}, {}",
                            name, args
                        ),
                    )
                }
            }
        }
        while let Some(message) = user_message_buf.pop_front() {
            memory.push_user(&message);
        }
    }

    Ok(())
}
