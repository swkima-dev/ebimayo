use anyhow::Result;
use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::mpsc::Sender;

#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;

    async fn start(&self, tx: Sender<InboundEvent>);

    async fn respond(&self, msg: IncomingMessage, response: &str) -> Result<(), ChannelError>;

    async fn send_status(&self, status: StatusUpdate) -> Result<(), ChannelError>;

    async fn shutdown(&self) -> Result<(), ChannelError> {
        Ok(())
    }

    async fn turn_complete(&self) -> Result<(), ChannelError> {
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ChannelError {
    #[error("invalid approval")]
    InvalidApproval,
}

#[derive(Debug)]
pub enum StatusUpdate {
    Thinking,
    ApprovalNeeded {
        request_id: String,
        tool_name: String,
        args: String,
    },
    InvalidApproval {
        request_id: String,
        approval: bool,
    },
}

pub enum InboundEvent {
    UserMessage(IncomingMessage),
    ApprovalResponse { request_id: String, approved: bool },
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
