use futures_util::{FutureExt, StreamExt};
use warp::Filter;
use warp::ws::Ws;

mod protocol;

#[tokio::main]
async fn main() {

    let routes = warp::path("lobby_api")
        .and(warp::ws())
        .map(|ws: Ws| {
            ws.on_upgrade(|websocket| {
                // Echo all received messages back
                let (sink, stream) = websocket.split();
                stream.forward(sink).map(|result| {
                    if let Err(e) = result {
                        eprintln!("Encountered WebSocket error: {:?}", e);
                    }
                })
            })
        });

    // Start WebSocket server and await indenifitely
    warp::serve(routes).run(([127, 0, 0, 1], 9000)).await;
}
