use tokio::sync::mpsc::UnboundedSender;

use crate::lobby_session::ClientSessionAction::*;
use crate::protocol::{Input, Output, UserType, Username, Password, Seq};
use crate::protocol::Input::*;
use crate::protocol::Output::*;

/// The action to perform to the client session upon processing the input message.
pub enum ClientSessionAction {
    DoNothing,
    UpdateUserType { user_type: Option<UserType> },
}

/// Represents the result of processing the input message.
pub struct ProcessResult {
    pub output: Option<Output>,
    pub subscription_output: Option<Output>,
    pub action: ClientSessionAction,
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
    pub subscribed: bool,
    pub sender: ClientSender,
}

pub fn process(user_type: &Option<UserType>, input: Input) -> ProcessResult {
    match user_type {
        None => process_unathenticated(input),
        Some(UserType::User) => process_user(input),
        Some(UserType::Admin) => process_admin(input),
    }
}

fn process_unathenticated(input: Input) -> ProcessResult {
    match input {
        Login { username, password } => login(username, password),
        _ => ProcessResult {
            output: Some(NotAuthenticated),
            subscription_output: None,
            action: DoNothing,
        }
    }
}

fn process_user(input: Input) -> ProcessResult {
    match input {
        Ping { seq } => ping(seq),
        Login { username, password } => login(username, password),
        SubscribeTables |
        UnsubscribeTables => todo!("Complete implementation"),
        AddTable { .. } |
        UpdateTable { .. } |
        RemoveTable { .. } => ProcessResult {
            output: Some(NotAuthorized),
            subscription_output: None,
            action: DoNothing,
        },
    }
}

fn process_admin(input: Input) -> ProcessResult {
    match input {
        Ping { seq } => ping(seq),
        Login { username, password } => login(username, password),
        SubscribeTables |
        UnsubscribeTables |
        AddTable { .. } |
        UpdateTable { .. } |
        RemoveTable { .. } => todo!("Complete implementation"),
    }
}

fn ping(seq: Seq) -> ProcessResult {
    ProcessResult {
        output: Some(Pong { seq }),
        subscription_output: None,
        action: DoNothing,
    }
}

fn login(username: Username, password: Password) -> ProcessResult {
    let user_type = match (username.0.as_str(), password.0.as_str()) {
        ("admin", "admin") => Some(UserType::Admin),
        ("user", "user") => Some(UserType::User),
        _ => None,
    };
    let output = match user_type.clone() {
        None => LoginFailed,
        Some(user_type) => LoginSuccessful { user_type },
    };
    ProcessResult {
        output: Some(output),
        subscription_output: None,
        action: UpdateUserType { user_type },
    }
}
