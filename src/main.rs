pub mod config;
pub mod memory;
pub mod util;

use std::io;

use rig::{
    client::CompletionClient,
    completion::Completion,
    providers::anthropic::{Client, completion::ANTHROPIC_VERSION_LATEST},
};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    const MAX_ITERATIONS: u16 = 1000;
    println!("ebimayo!");

    let api_key = config::load_anthropic_api_key();

    // magic number. refactor it later
    let mut main_memory = memory::ConversationMemory::new(700_000);

    let client = Client::builder()
        .api_key(api_key)
        .anthropic_version(ANTHROPIC_VERSION_LATEST)
        .build()?;

    for _ in 1..MAX_ITERATIONS {
        let mut user_input = String::new();
        io::stdin()
            .read_line(&mut user_input)
            .expect("std input error");
        main_memory.push_user(user_input.as_str());
        let messages = main_memory.messages();
        let (prompt, history) = messages.split_last().expect("messages should not be empty");
        let agent = client.agent("claude-sonnet-4-6").build();

        let response = agent
            .completion(prompt, history.to_vec())
            .await?
            .send()
            .await?;
        let response_text = util::extract_text(&response.choice);

        println!("Response: {response_text}");

        main_memory.push_assistant(&response);
    }

    Ok(())
}
