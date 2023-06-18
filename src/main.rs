use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures_util::{StreamExt, SinkExt, TryFutureExt};
use lobby_session::{ClientId, ClientSession};
use protocol::{Input, Output};
use tokio::sync::{RwLock, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;
use serde_json::Error as SerdeError;
use warp::Filter;
use warp::ws::{Ws, Message, WebSocket};

mod protocol;
mod lobby_session;

#[macro_use] extern crate log;

/// The global unique client id counter.
static NEXT_CLIENT_ID: AtomicUsize = AtomicUsize::new(1);

/// Represents the currently connected clients.
type Clients = Arc<RwLock<HashMap<ClientId, ClientSession>>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Keep track of all connected clients
    let clients = Clients::default();
    let clients = warp::any().map(move || clients.clone());

    let routes = warp::path("lobby_api")
        .and(warp::ws())
        .and(clients)
        .map(|ws: Ws, clients| {
            ws.on_upgrade(move |ws| handle_connect(ws, clients))
        });

    // Start WebSocket server and await indenifitely
    // TODO Extract host and port into configuration parameters
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}

async fn handle_connect(ws: WebSocket, clients: Clients) {
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
                        match input {
                            Ok(input) => {
                                process_input(client_id, &clients, input).await;
                            }
                            Err(e) => {
                                error!("Failed to deserialize WebSocket message for client {:?}: {}", client_id, e);
                            }
                        }
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

async fn handle_disconnect(client_id: ClientId, clients: &Clients) {
    debug!("Client {:?} has disconnected", client_id);
    clients.write().await.remove(&client_id);
}

async fn process_input(client_id: ClientId, clients: &Clients, input: Input) {
    let process_result = lobby_session::process(input);
    if let Some(output) = process_result.output {
        if let Some(client_session) = clients.read().await.get(&client_id) {
            client_session.sender.send(output).unwrap_or_else(|e| {
                error!("Failed to send message for client {:?}: {}", client_id, e);
            });
        }
    }
}
