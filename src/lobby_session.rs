use tokio::sync::mpsc::UnboundedSender;

use crate::protocol::{Input, Output, UserType, Username, Password, Seq};
use crate::protocol::Input::*;
use crate::protocol::Output::*;

/// Represents the result of processing an input message.
pub struct ProcessResult {
    pub output: Option<Output>,
    pub subscription_output: Option<Output>,
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

pub fn process(client_session: &mut ClientSession, input: Input) -> ProcessResult {
    match client_session.user_type {
        None => process_unathenticated(client_session, input),
        Some(UserType::User) => process_user(client_session, input),
        Some(UserType::Admin) => process_admin(client_session, input),
    }
}

fn process_unathenticated(client_session: &mut ClientSession, input: Input) -> ProcessResult {
    match input {
        Login { username, password } => login(client_session, username, password),
        _ => ProcessResult {
            output: Some(NotAuthenticated),
            subscription_output: None,
        }
    }
}

fn process_user(client_session: &mut ClientSession, input: Input) -> ProcessResult {
    match input {
        Ping { seq } => ping(seq),
        Login { username, password } => login(client_session, username, password),
        SubscribeTables => todo!(),
        UnsubscribeTables => todo!(),
        AddTable { .. } |
        UpdateTable { .. } |
        RemoveTable { .. } => ProcessResult {
            output: Some(NotAuthorized),
            subscription_output: None,
        },
    }
}

fn process_admin(client_session: &mut ClientSession, input: Input) -> ProcessResult {
    match input {
        Ping { seq } => ping(seq),
        Login { username, password } => login(client_session, username, password),
        SubscribeTables => todo!(),
        UnsubscribeTables => todo!(),
        AddTable { .. } => todo!(),
        UpdateTable { .. } => todo!(),
        RemoveTable { .. } => todo!(),
    }
}

fn ping(seq: Seq) -> ProcessResult {
    ProcessResult {
        output: Some(Pong { seq }),
        subscription_output: None,
    }
}

fn login(client_session: &mut ClientSession, username: Username, password: Password) -> ProcessResult {
    let user_type = match (username.0.as_str(), password.0.as_str()) {
        ("admin", "admin") => Some(UserType::Admin),
        ("user", "user") => Some(UserType::User),
        _ => None,
    };
    client_session.user_type = user_type.clone();
    let output = match user_type {
        None => LoginFailed,
        Some(user_type) => LoginSuccessful { user_type },
    };
    ProcessResult {
        output: Some(output),
        subscription_output: None,
    }
}
