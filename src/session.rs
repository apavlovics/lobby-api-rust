use std::collections::{HashMap, HashSet};
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
    pub success_client_ids: HashSet<ClientId>,
    pub failure_client_ids: HashSet<ClientId>,
}
impl BroadcastResult {
    pub fn new() -> Self {
        BroadcastResult {
            success_client_ids: HashSet::new(),
            failure_client_ids: HashSet::new(),
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
                    acc.success_client_ids.insert(client_id);
                    acc
                }
                Err(client_id) => {
                    acc.failure_client_ids.insert(client_id);
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

    use std::collections::HashSet;

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

    #[tokio::test]
    async fn broadcast_output_to_subscribed_client_ids() {
        // given
        let shared_sessions = SharedSessions::new();

        let client_id_1 = ClientId::new();
        let client_id_2 = ClientId::new();
        let client_id_3 = ClientId::new();

        let (client_sender_1, mut client_receiver_1) = mpsc::unbounded_channel::<Output>();
        let (client_sender_2, mut client_receiver_2) = mpsc::unbounded_channel::<Output>();
        let (client_sender_3, mut client_receiver_3) = mpsc::unbounded_channel::<Output>();

        let broadcasted_output = test_data::pong();

        shared_sessions.add(client_id_1, client_sender_1).await;
        shared_sessions.add(client_id_2, client_sender_2).await;
        shared_sessions.add(client_id_3, client_sender_3).await;

        shared_sessions
            .write_subscribed(client_id_2, true)
            .await
            .expect("Client #2 should be subscribed");
        shared_sessions
            .write_subscribed(client_id_3, true)
            .await
            .expect("Client #3 should be subscribed");

        // when
        let broadcast_result = shared_sessions.broadcast(broadcasted_output.clone()).await;

        // then
        assert_eq!(broadcast_result.success_client_ids, HashSet::from([client_id_2, client_id_3]));
        assert!(broadcast_result.failure_client_ids.is_empty());

        let received_output_1 = client_receiver_1.try_recv();
        let received_output_2 = client_receiver_2
            .recv()
            .await
            .expect("Output for client #2 should be received");
        let received_output_3 = client_receiver_3
            .recv()
            .await
            .expect("Output for client #3 should be received");

        assert!(received_output_1.is_err(), "Output for client #1 should not be received");
        assert_eq!(received_output_2, broadcasted_output);
        assert_eq!(received_output_3, broadcasted_output);
    }

    #[tokio::test]
    async fn read_user_type_of_existing_client_id() {
        // given
        let shared_sessions = SharedSessions::new();
        let client_id = ClientId::new();
        let (client_sender, _) = mpsc::unbounded_channel::<Output>();
        shared_sessions.add(client_id, client_sender).await;

        // when
        let result = shared_sessions.read_user_type(client_id).await;

        // then
        let user_type = result.expect("User type should be read");
        assert!(user_type.is_none(), "User type should be none");
    }

    #[tokio::test]
    async fn not_read_user_type_of_missing_client_id() {
        // given
        let shared_sessions = SharedSessions::new();
        let client_id = ClientId::new();

        // when
        let result = shared_sessions.read_user_type(client_id).await;

        // then
        assert!(result.is_err(), "User type should not be read");
    }
}
