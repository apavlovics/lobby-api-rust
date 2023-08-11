use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::RwLock;

use crate::{
    protocol::{Output, UserType},
    service::ClientId,
};

/// Represents the sender, which can be used to output messages to the client.
type ClientSender = UnboundedSender<Output>;

/// Represents the client session.
struct Session {
    pub client_id: ClientId,
    pub client_sender: ClientSender,
    pub user_type: Option<UserType>,
    pub subscribed: bool,
}

#[derive(Debug)]
pub struct BroadcastResult {
    pub success_client_ids: Vec<ClientId>,
    pub failure_client_ids: Vec<ClientId>,
}
impl BroadcastResult {
    pub fn new() -> Self {
        BroadcastResult {
            success_client_ids: vec![],
            failure_client_ids: vec![],
        }
    }
}

/// Represents the currently connected client sessions.
#[derive(Clone)]
pub struct SharedSessions {
    sessions: Arc<RwLock<HashMap<ClientId, Session>>>,
}
impl SharedSessions {
    pub fn new() -> Self {
        SharedSessions {
            sessions: Arc::default(),
        }
    }

    pub async fn add(&self, client_id: ClientId, client_sender: ClientSender) {
        let session = Session {
            client_id,
            client_sender,
            user_type: None,
            subscribed: false,
        };
        self.sessions.write().await.insert(client_id, session);
    }

    pub async fn remove(&self, client_id: ClientId) {
        self.sessions.write().await.remove(&client_id);
    }

    /// Sends the output message to the given client.
    pub async fn send(&self, client_id: ClientId, output: Output) -> Result<(), String> {
        match self.sessions.read().await.get(&client_id) {
            Some(session) => session.client_sender.send(output).map_err(|e| e.to_string()),
            None => Self::no_session(client_id),
        }
    }

    /// Broadcasts the output message to all subscribed clients.
    pub async fn broadcast(&self, output: Output) -> BroadcastResult {
        self.sessions
            .read()
            .await
            .values()
            .filter_map(|session| {
                if session.subscribed {
                    Some(
                        session
                            .client_sender
                            .send(output.clone())
                            .map(|_| session.client_id)
                            .map_err(|_| session.client_id),
                    )
                } else {
                    None
                }
            })
            .fold(BroadcastResult::new(), |mut acc, result| match result {
                Ok(client_id) => {
                    acc.success_client_ids.push(client_id);
                    acc
                }
                Err(client_id) => {
                    acc.failure_client_ids.push(client_id);
                    acc
                }
            })
    }

    pub async fn read_user_type(&self, client_id: ClientId) -> Result<Option<UserType>, String> {
        if let Some(session) = self.sessions.read().await.get(&client_id) {
            Ok(session.user_type.clone())
        } else {
            Self::no_session(client_id)
        }
    }

    pub async fn write_user_type(&self, client_id: ClientId, user_type: Option<UserType>) -> Result<(), String> {
        self.write(client_id, |session| {
            session.user_type = user_type;
        })
        .await
    }

    pub async fn write_subscribed(&self, client_id: ClientId, subscribed: bool) -> Result<(), String> {
        self.write(client_id, |session| {
            session.subscribed = subscribed;
        })
        .await
    }

    async fn write<F>(&self, client_id: ClientId, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut Session) -> (),
    {
        if let Some(session) = self.sessions.write().await.get_mut(&client_id) {
            f(session);
            Ok(())
        } else {
            Self::no_session(client_id)
        }
    }

    fn no_session<T>(client_id: ClientId) -> Result<T, String> {
        Err(format!("Failed to retrieve session for client {:?}", client_id))
    }
}

#[cfg(test)]
mod tests {

    use tokio::sync::mpsc;

    use crate::{
        protocol::{test_data, Output},
        service::ClientId,
        session::SharedSessions,
    };

    #[tokio::test]
    async fn send_output_to_existing_client_id() {
        // given
        let shared_sessions = SharedSessions::new();
        let client_id = ClientId::new();
        let (client_sender, mut client_receiver) = mpsc::unbounded_channel::<Output>();
        let sent_output = test_data::pong();
        shared_sessions.add(client_id, client_sender).await;

        // when
        let result = shared_sessions.send(client_id, sent_output.clone()).await;

        // then
        result.expect("Output should be sent");
        let received_output = client_receiver.recv().await.expect("Output should be received");
        assert_eq!(received_output, sent_output);
    }

    #[tokio::test]
    async fn not_send_output_to_missing_client_id() {
        // given
        let shared_sessions = SharedSessions::new();
        let client_id = ClientId::new();
        let sent_output = test_data::pong();

        // when
        let result = shared_sessions.send(client_id, sent_output.clone()).await;

        // then
        assert!(result.is_err(), "Output should not be sent");
    }
}
