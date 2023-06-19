use warp::Filter;
use warp::ws::Ws;

use crate::lobby::SharedLobby;
use crate::session::SharedSessions;

mod lobby;
mod protocol;
mod service;
mod session;
mod web_socket;

#[macro_use] extern crate log;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Keep track of all connected clients
    let sessions = SharedSessions::new();
    let sessions = warp::any().map(move || sessions.clone());

    // Keep track of the lobby
    let lobby = SharedLobby::prepopulated();
    let lobby = warp::any().map(move || lobby.clone());

    let routes = warp::path("lobby_api")
        .and(warp::ws())
        .and(sessions)
        .and(lobby)
        .map(|ws: Ws, sessions: SharedSessions, lobby: SharedLobby| {
            ws.on_upgrade(move |ws| web_socket::handle_connect(ws, sessions, lobby))
        });

    // Start WebSocket server and await indenifitely
    // TODO Extract host and port into configuration parameters
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}
