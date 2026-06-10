use std::io;
use std::io::Write;

use async_trait::async_trait;

use crate::channels::channel::{Channel, ChannelError, IncomingMessage, StatusUpdate};

pub struct CliChannel;

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn receive(&self) -> Result<IncomingMessage, ChannelError> {
        let mut user_input = String::new();

        print!("User: ");
        io::stdout().flush().unwrap();
        io::stdin()
            .read_line(&mut user_input)
            .expect("std input error");

        Ok(IncomingMessage::new(user_input))
    }

    async fn respond(&self, _msg: IncomingMessage, response: &str) -> Result<(), ChannelError> {
        println!("{}", response);
        Ok(())
    }

    async fn send_status(&self, status: StatusUpdate) -> Result<(), ChannelError> {
        match status {
            StatusUpdate::Thinking => {
                println!("Thinking...");
                Ok(())
            }
            StatusUpdate::ApprovalNeeded { tool_name, args } => {
                println!("Tool call {}, {}, Approve?(y/n): ", tool_name, args);
                Ok(())
            }
        }
    }
}

impl CliChannel {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}
