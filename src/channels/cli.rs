use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{io, sync::Arc};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::channels::channel::{
    Channel, ChannelError, ChannelType, InboundEvent, IncomingMessage, StatusUpdate,
};

pub struct CliChannel {
    stdin_locked: Arc<AtomicBool>,
    wating_approval: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn start(&self, tx: mpsc::Sender<InboundEvent>) {
        let stdin_locked = Arc::clone(&self.stdin_locked);
        let waiting_approval = self.wating_approval.clone();

        std::thread::spawn(move || {
            loop {
                let mut user_input = String::new();
                while stdin_locked.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                }

                let msg: InboundEvent;

                {
                    let approval_needed = waiting_approval.lock().unwrap().clone();
                    match approval_needed {
                        Some(id) => {
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
                            stdin_locked.store(true, Ordering::Relaxed);

                            let approved: bool;

                            if user_input.trim() == "y" {
                                approved = true;
                            } else {
                                approved = false;
                            }

                            msg = InboundEvent::ApprovalResponse {
                                request_id: id,
                                approved: approved,
                            };
                        }
                        None => {
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
                            stdin_locked.store(true, Ordering::Relaxed);

                            msg = InboundEvent::UserMessage(IncomingMessage {
                                content: user_input,
                                channel_type: ChannelType::Cli,
                            });
                        }
                    }
                }

                let mut approval_needed = waiting_approval.lock().unwrap();
                *approval_needed = None;

                tx.blocking_send(msg).unwrap();
            }
        });
    }

    async fn respond(&self, _msg: IncomingMessage, response: &str) -> Result<(), ChannelError> {
        println!("{}", response);
        let stdin_locked = Arc::clone(&self.stdin_locked);
        stdin_locked.store(false, Ordering::Relaxed);
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
                let mut requested_id = self.wating_approval.lock().unwrap();
                *requested_id = Some(request_id);
                Ok(())
            }
            InvalidApproval {
                request_id,
                approval,
            } => {
                eprintln!("Invalid Approval: {} {}", request_id, approval);
                Err(ChannelError::InvalidApproval)
            }
            ApprovalExpected { message } => {
                eprintln!("tool use approval expected but found message {}", message);
                Err(ChannelError::InvalidApproval)
            }
        }
    }
}

impl CliChannel {
    pub fn new() -> Self {
        Self {
            stdin_locked: Arc::new(AtomicBool::new(false)),
            wating_approval: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}
