use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::{Input, Output, UserType};
use crate::protocol::Input::*;
use crate::protocol::Output::*;

pub struct ProcessResult {
    pub output: Option<Output>,
    pub push_outputs: Vec<Output>,
}

/// Represents the client id.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ClientId(pub usize);

/// Represents the sender, which can be used to output messages to the client.
pub type ClientSender = UnboundedSender<Output>;

/// Represents the client session.
pub struct ClientSession {
    pub id: ClientId,
    pub user_type: Option<UserType>,
    pub sender: ClientSender,
}

pub fn process(input: Input) -> ProcessResult {
    match input {
        Ping { seq } => ProcessResult {
            output: Some(Pong { seq }),
            push_outputs: Vec::new(),
        },
        Login { username, password } => todo!(),
        SubscribeTables => todo!(),
        UnsubscribeTables => todo!(),
        AddTable { after_id, table_to_add } => todo!(),
        UpdateTable { table } => todo!(),
        RemoveTable { id } => todo!(),
    }
}
