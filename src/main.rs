use ebimayo::channels::channel::{Channel, StatusUpdate};
use ebimayo::channels::cli::CliChannel;
use ebimayo::tool::{glob::Glob, grep::Grep, read::Read};
use ebimayo::{config, memory, util};
use rig::{
    client::CompletionClient,
    completion::Completion,
    message::AssistantContent,
    providers::anthropic::{Client, completion::ANTHROPIC_VERSION_LATEST},
    tool::ToolSet,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let (tx, mut rx) = mpsc::channel(32);
    let tx_cli = tx.clone();

    const MAX_ITERATIONS: u16 = 1000;
    println!("ebimayo!");

    let channel = CliChannel::new();
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

    while let Some(user_message) = rx.recv().await {
        main_memory.push_user(user_message.content.as_str());
        let agent = client
            .agent("claude-sonnet-4-6")
            .tool(Read)
            .tool(Grep)
            .tool(Glob)
            .build();

        for _ in 1..MAX_ITERATIONS {
            let messages = main_memory.messages();
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

            main_memory.push_assistant(&response);

            let has_tool_calls = response
                .choice
                .iter()
                .any(|c| matches!(c, AssistantContent::ToolCall(_)));

            if !has_tool_calls {
                break;
            };

            for content in response.choice.iter() {
                if let AssistantContent::ToolCall(tool_call) = content {
                    let name = &tool_call.function.name;
                    let args = &tool_call.function.arguments;

                    channel
                        .send_status(StatusUpdate::ApprovalNeeded {
                            tool_name: name.to_string(),
                            args: args.to_string(),
                        })
                        .await
                        .unwrap();

                    let user_judge = rx.recv().await.unwrap();
                    if user_judge.content.trim() == "y" {
                        let result = main_tools.call(name, args.to_string()).await?;
                        main_memory.push_tool_result(&tool_call.id, result);
                    } else {
                        main_memory.push_tool_result(
                            &tool_call.id,
                            format!(
                                "Tool use was denied by user. Denied tool call: {}, {}",
                                name, args
                            ),
                        )
                    }
                }
            }
        }
    }

    Ok(())
}
