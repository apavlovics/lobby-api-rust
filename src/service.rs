use std::sync::atomic::{AtomicUsize, Ordering};

use crate::lobby::SharedLobby;
use crate::service::ClientSessionAction::*;
use crate::protocol::{Input, Output, UserType, Username, Password, Seq};
use crate::protocol::Input::*;
use crate::protocol::Output::*;

/// The action to perform to the client session upon processing the input message.
pub enum ClientSessionAction {
    DoNothing,
    UpdateUserType { user_type: Option<UserType> },
    UpdateSubscribed { subscribed: bool },
}

/// Represents the result of processing the input message.
pub struct ProcessResult {
    pub output: Option<Output>,
    pub subscription_output: Option<Output>,
    pub action: ClientSessionAction,
}

/// The global unique client id counter.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// Represents the client id.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ClientId(pub usize);
impl ClientId {

    pub fn new() -> Self {
        ClientId(NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

pub async fn process(input: Input, user_type: &Option<UserType>, lobby: &SharedLobby) -> ProcessResult {
    match user_type {
        None => process_unathenticated(input),
        Some(UserType::User) => process_user(input, lobby).await,
        Some(UserType::Admin) => process_admin(input, lobby).await,
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

async fn process_user(input: Input, lobby: &SharedLobby) -> ProcessResult {
    match input {
        Ping { seq } => ping(seq),
        Login { username, password } => login(username, password),
        SubscribeTables => subscribe(lobby).await,
        UnsubscribeTables => unsubscribe(),
        AddTable { .. } |
        UpdateTable { .. } |
        RemoveTable { .. } => ProcessResult {
            output: Some(NotAuthorized),
            subscription_output: None,
            action: DoNothing,
        },
    }
}

async fn process_admin(input: Input, lobby: &SharedLobby) -> ProcessResult {
    match input {
        Ping { seq } => ping(seq),
        Login { username, password } => login(username, password),
        SubscribeTables => subscribe(lobby).await,
        UnsubscribeTables => unsubscribe(),
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

async fn subscribe(lobby: &SharedLobby) -> ProcessResult {
    // TODO Complete implementation
    let tables = lobby.read_tables().await;
    ProcessResult {
        output: Some(TableList { tables }),
        subscription_output: None,
        action: UpdateSubscribed { subscribed: true },
    }
}

fn unsubscribe() -> ProcessResult {
    ProcessResult {
        output: None,
        subscription_output: None,
        action: UpdateSubscribed { subscribed: false },
    }
}