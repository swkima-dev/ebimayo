use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;
use std::{io, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::channels::channel::{
    Channel, ChannelError, ChannelType, InboundEvent, IncomingMessage, StatusUpdate,
};

pub struct CliChannel {
    state: Arc<Mutex<ReadState>>,
}

#[derive(Debug)]
enum ReadState {
    Locked,
    AwaitingMessage,
    AwaitingApproval(String),
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn start(&self, tx: mpsc::Sender<InboundEvent>) {
        let state = Arc::clone(&self.state);

        std::thread::spawn(move || {
            loop {
                let mut user_input = String::new();

                while matches!(*state.lock().unwrap(), ReadState::Locked) {
                    std::thread::sleep(Duration::from_millis(50));
                }

                let msg: InboundEvent;

                {
                    let mut state_guard = state.lock().unwrap();
                    let current = std::mem::replace(&mut *state_guard, ReadState::Locked);
                    drop(state_guard);
                    match current {
                        ReadState::AwaitingApproval(id) => {
                            println!("Approve?(y/n): ");
                            io::stdout().flush().unwrap();
                            match io::stdin().read_line(&mut user_input) {
                                Ok(0) => break,
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("stdin error: {e}");
                                    break;
                                }
                            }

                            let approved = user_input.trim() == "y";

                            msg = InboundEvent::ApprovalResponse {
                                request_id: id,
                                approved,
                            };
                        }
                        ReadState::AwaitingMessage => {
                            print!("User: ");
                            io::stdout().flush().unwrap();
                            match io::stdin().read_line(&mut user_input) {
                                Ok(0) => break,
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!("stdin error: {e}");
                                    break;
                                }
                            }

                            msg = InboundEvent::UserMessage(IncomingMessage {
                                content: user_input,
                                channel_type: ChannelType::Cli,
                            });
                        }
                        ReadState::Locked => unreachable!(),
                    }
                }

                tx.blocking_send(msg).unwrap();
            }
        });
    }

    async fn respond(&self, _msg: IncomingMessage, response: &str) -> Result<(), ChannelError> {
        println!("{}", response);
        Ok(())
    }

    async fn send_status(&self, status: StatusUpdate) -> Result<(), ChannelError> {
        use StatusUpdate::*;
        match status {
            Thinking => {
                println!("Thinking...");
                Ok(())
            }
            ApprovalNeeded {
                request_id,
                tool_name,
                args,
            } => {
                println!("Tool call {}, {}", tool_name, args);

                *self.state.lock().unwrap() = ReadState::AwaitingApproval(request_id);
                Ok(())
            }
            InvalidApproval {
                request_id,
                approval,
            } => {
                eprintln!("Invalid Approval: {} {}", request_id, approval);
                Err(ChannelError::InvalidApproval)
            }
        }
    }

    async fn turn_complete(&self) -> Result<(), ChannelError> {
        *self.state.lock().unwrap() = ReadState::AwaitingMessage;
        Ok(())
    }
}

impl CliChannel {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ReadState::AwaitingMessage)),
        }
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}
