use async_trait::async_trait;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;

    async fn start(&self, tx: Sender<IncomingMessage>);

    async fn respond(&self, msg: IncomingMessage, response: &str) -> Result<(), ChannelError>;

    async fn send_status(&self, status: StatusUpdate) -> Result<(), ChannelError>;

    async fn shutdown(&self) -> Result<(), ChannelError> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum ChannelError {}

#[derive(Debug)]
pub enum StatusUpdate {
    Thinking,
    ApprovalNeeded { tool_name: String, args: String },
}

#[derive(Debug, Clone)]
pub struct IncomingMessage {
    pub content: String,
    pub channel_type: ChannelType,
    // When we support messaging platforms such as Slack and Discord in the future,
    // there is a possibility that additional fields will be added.
}

impl IncomingMessage {
    pub fn new(content: impl Into<String>, channel_type: ChannelType) -> Self {
        Self {
            content: content.into(),
            channel_type,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChannelType {
    Cli,
    Discord,
}
