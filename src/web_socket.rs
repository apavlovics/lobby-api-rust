use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde_json::Error as SerdeError;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket, Ws};
use warp::Filter;

use crate::lobby::SharedLobby;
use crate::protocol::{Input, Output};
use crate::service::ClientSessionAction::*;
use crate::service::{self, ClientId, ClientSessionAction};
use crate::session::SharedSessions;

/// Starts WebSocket server at the given address and awaits indenifitely.
pub async fn run(address: impl Into<SocketAddr>) {
    // Keep track of all connected clients
    let sessions = SharedSessions::new();
    let sessions = warp::any().map(move || sessions.clone());

    // Keep track of the lobby
    let lobby = SharedLobby::prepopulated();
    let lobby = warp::any().map(move || lobby.clone());

    let routes = warp::path("lobby_api").and(warp::ws()).and(sessions).and(lobby).map(
        |ws: Ws, sessions: SharedSessions, lobby: SharedLobby| {
            ws.on_upgrade(move |ws| handle_connect(ws, sessions, lobby))
        },
    );
    warp::serve(routes).run(address).await;
}

async fn handle_connect(ws: WebSocket, sessions: SharedSessions, lobby: SharedLobby) {
    let client_id = ClientId::new();
    debug!("Connected client {:?}", client_id);

    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (client_sender, client_receiver) = mpsc::unbounded_channel::<Output>();
    let mut client_receiver = UnboundedReceiverStream::new(client_receiver);

    // Spawn a task per client that serializes and sends outgoing messages
    tokio::task::spawn(async move {
        while let Some(output) = client_receiver.next().await {
            match serde_json::to_string(&output).map(Message::text) {
                Ok(message) => {
                    ws_sender
                        .send(message)
                        .unwrap_or_else(|e| {
                            error!("Failed to send WebSocket message to client {:?}: {}", client_id, e);
                        })
                        .await;
                }
                Err(e) => {
                    error!("Failed to serialize WebSocket message for client {:?}: {}", client_id, e);
                }
            };
        }
    });

    // Add the new client session
    sessions.add(client_id, client_sender).await;

    // Receive, deserialize and process incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(message) => match message.to_str() {
                Ok(string) => {
                    let input: Result<Input, SerdeError> = serde_json::from_str(string);
                    process_input(client_id, &sessions, &lobby, input).await;
                }
                Err(_) => {
                    debug!("Received non-text WebSocket message from client {:?}, ignoring", client_id);
                }
            },
            Err(e) => {
                error!("Failed to receive WebSocket message from client {:?}: {}", client_id, e);
                break;
            }
        };
    }
    handle_disconnect(client_id, &sessions).await;
}

async fn handle_disconnect(client_id: ClientId, sessions: &SharedSessions) {
    debug!("Client {:?} has disconnected", client_id);
    sessions.remove(client_id).await;
}

async fn process_input(
    client_id: ClientId,
    sessions: &SharedSessions,
    lobby: &SharedLobby,
    input: Result<Input, SerdeError>,
) {
    let action: ClientSessionAction = match input {
        Ok(input) => match sessions.read_user_type(client_id).await {
            Ok(user_type) => {
                let process_result = service::process(input, &user_type, lobby).await;
                if let Some(output) = process_result.output {
                    process_output(client_id, &sessions, output).await;
                }
                if let Some(subscription_output) = process_result.subscription_output {
                    broadcast(&sessions, subscription_output).await;
                }
                process_result.action
            }
            Err(e) => {
                error!("Failed to read user type for client {:?}: {}", client_id, e);
                DoNothing
            }
        },
        Err(e) => {
            error!("Failed to deserialize WebSocket message for client {:?}: {}", client_id, e);
            process_output(client_id, &sessions, Output::InvalidMessage).await;
            DoNothing
        }
    };

    match action {
        DoNothing => {}
        UpdateUserType { user_type } => {
            sessions
                .write_user_type(client_id, user_type)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to write user type for client {:?}: {}", client_id, e);
                });
        }
        UpdateSubscribed { subscribed } => {
            sessions
                .write_subscribed(client_id, subscribed)
                .await
                .unwrap_or_else(|e| {
                    error!("Failed to write subscribed for client {:?}: {}", client_id, e);
                });
        }
    }
}

async fn process_output(client_id: ClientId, sessions: &SharedSessions, output: Output) {
    sessions.send(client_id, output).await.unwrap_or_else(|e| {
        error!("Failed to send message for client {:?}: {}", client_id, e);
    });
}

async fn broadcast(sessions: &SharedSessions, output: Output) {
    let broadcast_result = sessions.broadcast(output).await;
    debug!("Broadcasted message: {:?}", broadcast_result);
}
