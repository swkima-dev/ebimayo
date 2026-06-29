use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::{io, sync::Arc};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::channels::channel::{Channel, ChannelError, ChannelType, IncomingMessage, StatusUpdate};

pub struct CliChannel {
    stdin_locked: Arc<AtomicBool>,
}

#[async_trait]
impl Channel for CliChannel {
    fn name(&self) -> &str {
        "cli"
    }

    async fn start(&self, tx: mpsc::Sender<IncomingMessage>) {
        let stdin_locked = Arc::clone(&self.stdin_locked);

        std::thread::spawn(move || {
            loop {
                let mut user_input = String::new();
                while stdin_locked.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(50));
                }

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

                let msg = IncomingMessage {
                    content: user_input,
                    channel_type: ChannelType::Cli,
                };

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
        Self {
            stdin_locked: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Default for CliChannel {
    fn default() -> Self {
        Self::new()
    }
}
