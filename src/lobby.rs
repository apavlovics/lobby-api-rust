use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{Table, TableId, TableName};

/// Represent the lobby that contains ordered tables.
pub struct Lobby {
    pub tables: Vec<Table>,
}
impl Lobby {

    pub fn prepopulated() -> Self {
        Lobby {
            tables: vec![
                Table {
                    id: TableId(1),
                    name: TableName(String::from("table - James Bond")),
                    participants: 7,
                },
                Table {
                    id: TableId(2),
                    name: TableName(String::from("table - Mission Impossible")),
                    participants: 9,
                },
            ],
        }
    }
}

/// Represent the lobby that is shared among all the clients.
#[derive(Clone)]
pub struct SharedLobby {
    lobby: Arc<RwLock<Lobby>>,
}
impl SharedLobby {

    pub fn prepopulated() -> Self {
        SharedLobby {
            lobby: Arc::from(RwLock::from(Lobby::prepopulated())),
        }
    }

    pub async fn read_tables(&self) -> Vec<Table> {
        self.lobby.read().await.tables.clone()
    }
}
