pub mod config;
pub mod memory;
pub mod tool;
pub mod util;

use crate::tool::{glob::Glob, grep::Grep, read::Read};
use rig::{
    client::CompletionClient,
    completion::Completion,
    message::AssistantContent,
    providers::anthropic::{Client, completion::ANTHROPIC_VERSION_LATEST},
    tool::ToolSet,
};
use std::io;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    const MAX_ITERATIONS: u16 = 1000;
    println!("ebimayo!");

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

    for _ in 1..MAX_ITERATIONS {
        let mut user_input = String::new();
        print!("User: ");
        io::stdout().flush().unwrap();
        io::stdin()
            .read_line(&mut user_input)
            .expect("std input error");
        main_memory.push_user(user_input.as_str());
        let agent = client
            .agent("claude-sonnet-4-6")
            .tool(Read)
            .tool(Grep)
            .tool(Glob)
            .build();

        loop {
            let messages = main_memory.messages();
            let (prompt, history) = messages.split_last().expect("messages should not be empty");
            let response = agent
                .completion(prompt, history.to_vec())
                .await?
                .send()
                .await?;
            let response_text = util::extract_text(&response.choice);

            println!("Response: {response_text}");

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

                    let mut user_judge = String::new();

                    print!("Tool call {}, {}, Approve?(y/n): ", name, args);
                    io::stdout().flush().unwrap();

                    user_judge.clear();
                    io::stdin()
                        .read_line(&mut user_judge)
                        .expect("std input error");
                    if user_judge.trim() == "y" {
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
