use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures_util::{StreamExt, SinkExt, TryFutureExt};
use lobby::Lobby;
use lobby_session::{ClientId, ClientSession, ClientSessionAction};
use lobby_session::ClientSessionAction::*;
use protocol::{Input, Output};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use serde_json::Error as SerdeError;
use warp::Filter;
use warp::ws::{Ws, Message, WebSocket};

mod protocol;
mod lobby;
mod lobby_session;

#[macro_use] extern crate log;

/// The global unique client id counter.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// Represents the currently connected clients.
type SharedClients = Arc<RwLock<HashMap<ClientId, ClientSession>>>;

/// Represent the lobby that is shared among all the clients.
type SharedLobby = Arc<Mutex<Lobby>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Keep track of all connected clients
    let clients = SharedClients::default();
    let clients = warp::any().map(move || clients.clone());

    // Keep track of the lobby
    let lobby = Arc::from(Mutex::from(Lobby::prepopulated()));
    let lobby = warp::any().map(move || lobby.clone());

    let routes = warp::path("lobby_api")
        .and(warp::ws())
        .and(clients)
        .and(lobby)
        .map(|ws: Ws, clients: SharedClients, lobby: SharedLobby| {
            ws.on_upgrade(move |ws| handle_connect(ws, clients, lobby))
        });

    // Start WebSocket server and await indenifitely
    // TODO Extract host and port into configuration parameters
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}

async fn handle_connect(ws: WebSocket, clients: SharedClients, lobby: SharedLobby) {
    let client_id = ClientId(NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed));
    debug!("Connected client {:?}", client_id);

    let (mut ws_sender, mut ws_receiver) = ws.split();
    let (client_sender, client_receiver) = mpsc::unbounded_channel::<Output>();
    let mut client_receiver = UnboundedReceiverStream::new(client_receiver);

    // Spawn a task per client that serializes and sends outgoing messages
    tokio::task::spawn(async move {
        while let Some(output) = client_receiver.next().await {
            match serde_json::to_string(&output).map(Message::text) {
                Ok(message) => {
                    ws_sender.send(message).unwrap_or_else(|e| {
                        error!("Failed to send WebSocket message to client {:?}: {}", client_id, e);
                    }).await;
                }
                Err(e) => {
                    error!("Failed to serialize WebSocket message for client {:?}: {}", client_id, e);
                }
            };
        }
    });

    // Create and persist the client session
    let client_session = ClientSession {
        id: client_id,
        user_type: None,
        subscribed: false,
        sender: client_sender,
    };
    clients.write().await.insert(client_id, client_session);

    // Receive, deserialize and process incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(message) => {
                match message.to_str() {
                    Ok(string) => {
                        let input: Result<Input, SerdeError> = serde_json::from_str(string);
                        process_input(client_id, &clients, &lobby, input).await;
                    }
                    Err(_) => {
                        debug!("Received non-text WebSocket message from client {:?}, ignoring", client_id);
                    }
                }
            }
            Err(e) => {
                error!("Failed to receive WebSocket message from client {:?}: {}", client_id, e);
                break;
            }
        };
    }
    handle_disconnect(client_id, &clients).await;
}

async fn handle_disconnect(client_id: ClientId, clients: &SharedClients) {
    debug!("Client {:?} has disconnected", client_id);
    clients.write().await.remove(&client_id);
}

async fn process_input(
    client_id: ClientId,
    clients: &SharedClients,
    lobby: &SharedLobby,
    input: Result<Input, SerdeError>,
) {
    let user_type = match clients.read().await.get(&client_id) {
        Some(client_session) => client_session.user_type.clone(),
        None => {
            error!("Failed to retrieve session for client {:?}", client_id);
            None
        }
    };

    let action: ClientSessionAction =
        match input {
            Ok(input) => {
                let process_result = lobby_session::process(&user_type, input);
                if let Some(output) = process_result.output {
                    process_output(client_id, &clients, output).await;
                }
                process_result.action
            }
            Err(e) => {
                error!("Failed to deserialize WebSocket message for client {:?}: {}", client_id, e);
                process_output(client_id, &clients, Output::InvalidMessage).await;
                DoNothing
            }
        };

    match action {
        DoNothing => {}
        UpdateUserType { user_type } => {
            if let Some(mut client_session) = clients.write().await.get_mut(&client_id) {
                client_session.user_type = user_type;
            }
        }
    }
}

async fn process_output(client_id: ClientId, clients: &SharedClients, output: Output) {
    match clients.read().await.get(&client_id) {
        Some(client_session) => {
            client_session.sender.send(output).unwrap_or_else(|e| {
                error!("Failed to send message for client {:?}: {}", client_id, e);
            });
        }
        None => {
            error!("Failed to retrieve session for client {:?}", client_id);
        }
    }
}
