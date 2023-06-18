use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures_util::{StreamExt, SinkExt, TryFutureExt};
use protocol::{Input, Output};
use tokio::sync::mpsc::UnboundedSender;
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

/// Represents the client id.
#[derive(Debug, Clone, Copy)]
struct ClientId(usize);

/// Represents the sender, which can be used to output messages to the client.
type ClientSender = UnboundedSender<Output>;

/// Represents the currently connected clients.
type Clients = Arc<RwLock<HashMap<ClientId, ClientSender>>>;

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
            ws.on_upgrade(move |ws| handle_connection(ws, clients))
        });

    // Start WebSocket server and await indenifitely
    // TODO Extract host and port into configuration parameters
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}

async fn handle_connection(ws: WebSocket, clients: Clients) {
    let client_id = ClientId(NEXT_CLIENT_ID.fetch_add(1, Ordering::Relaxed));
    info!("Connected client {:?}", client_id);

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

    // Receive, deserialize and process incoming messages
    while let Some(result) = ws_receiver.next().await {
        match result {
            Ok(message) => {
                match message.to_str() {
                    Ok(string) => {
                        let input: Result<Input, SerdeError> = serde_json::from_str(string);
                        match input {
                            Ok(input) => {
                                process(client_id, &client_sender, input).await;
                            }
                            Err(e) => {
                                error!("Failed to deserialize WebSocket message for client {:?}: {}", client_id, e);
                            }
                        }
                    }
                    Err(_) => {
                        error!("Received non-text WebSocket message from client {:?}, ignoring", client_id);
                    }
                }
            }
            Err(e) => {
                error!("Failed to receive WebSocket message from client {:?}: {}", client_id, e);
                break;
            }
        };
    }

    // TODO Handle client disconnection
}

async fn process(client_id: ClientId, client_sender: &ClientSender, input: Input) {
    lobby_session::process(input).output.into_iter().for_each(|output| {
        client_sender.send(output).unwrap_or_else(|e| {
            error!("Failed to send message for client {:?}: {}", client_id, e);
        });
    });
}
