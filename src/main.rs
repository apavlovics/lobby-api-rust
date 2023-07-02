use lobby_api_rust::web_socket;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // TODO Extract host and port into configuration parameters
    web_socket::run(([127, 0, 0, 1], 9000)).await;
}
