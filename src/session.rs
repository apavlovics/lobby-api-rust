use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::sync::mpsc::UnboundedSender;

use crate::{protocol::{Output, UserType}, service::ClientId};

/// Represents the sender, which can be used to output messages to the client.
pub type ClientSender = UnboundedSender<Output>;

/// Represents the client session.
pub struct Session {
    pub client_id: ClientId,
    pub client_sender: ClientSender,
    pub user_type: Option<UserType>,
    pub subscribed: bool,
}

/// Represents the currently connected client sessions.
pub type SharedSessions = Arc<RwLock<HashMap<ClientId, Session>>>;
