use std::sync::atomic::{AtomicUsize, Ordering};

use crate::lobby::SharedLobby;
use crate::service::ClientSessionAction::*;
use crate::protocol::{Input, Output, UserType, Username, Password, Seq, TableId, TableToAdd, Table};
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

/// The global unique client id generator.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// Represents the client id.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
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
        AddTable { after_id, table } => add_table(after_id, table, lobby).await,
        UpdateTable { table } => update_table(table, lobby).await,
        RemoveTable { id } => remove_table(id, lobby).await,
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
    let user_type = match (username.as_str(), password.as_str()) {
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

async fn add_table(after_id: TableId, table_to_add: TableToAdd, lobby: &SharedLobby) -> ProcessResult {
    match lobby.add_table(after_id, table_to_add).await {
        Ok(table) => ProcessResult {
            output: None,
            subscription_output: Some(TableAdded { after_id, table }),
            action: DoNothing,
        },
        Err(e) => {
            debug!("Failed to add table: {}", e);
            ProcessResult {
                output: Some(TableAddFailed),
                subscription_output: None,
                action: DoNothing,
            }
        }
    }
}

async fn update_table(table_to_update: Table, lobby: &SharedLobby) -> ProcessResult {
    let id = table_to_update.id;
    match lobby.update_table(table_to_update).await {
        Ok(table) => ProcessResult {
            output: None,
            subscription_output: Some(TableUpdated { table }),
            action: DoNothing,
        },
        Err(e) => {
            debug!("Failed to update table: {}", e);
            ProcessResult {
                output: Some(TableUpdateFailed { id }),
                subscription_output: None,
                action: DoNothing,
            }
        }
    }
}

async fn remove_table(id: TableId, lobby: &SharedLobby) -> ProcessResult {
    match lobby.remove_table(id).await {
        Ok(id) => ProcessResult {
            output: None,
            subscription_output: Some(TableRemoved { id }),
            action: DoNothing,
        },
        Err(e) => {
            debug!("Failed to remove table: {}", e);
            ProcessResult {
                output: Some(TableRemoveFailed { id }),
                subscription_output: None,
                action: DoNothing,
            }
        }
    }
}
