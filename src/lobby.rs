use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{Table, TableId, TableName, TableToAdd};

/// Represent the lobby that contains ordered tables.
pub struct Lobby {
    pub tables: Vec<Table>,
}
impl Lobby {

    pub fn prepopulated() -> Self {
        Lobby {
            tables: vec![
                Table {
                    id: TableId::new(),
                    name: TableName(String::from("table - James Bond")),
                    participants: 7,
                },
                Table {
                    id: TableId::new(),
                    name: TableName(String::from("table - Mission Impossible")),
                    participants: 9,
                },
            ],
        }
    }

    pub fn add_table(&mut self, after_id: TableId, table_to_add: TableToAdd) -> Result<Table, String> {
        let table = table_to_add.into_table(TableId::new());
        if after_id == TableId::ABSENT {
            self.tables.insert(0, table.clone());
            Ok(table)
        } else {
            match self.tables.iter().position(|table| table.id == after_id) {
                Some(index) => {
                    self.tables.insert(index + 1, table.clone());
                    Ok(table)
                }
                None => {
                    Err(format!("Cannot find existing table {:?}", after_id))
                }
            }
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

    pub async fn add_table(&self, after_id: TableId, table_to_add: TableToAdd) -> Result<Table, String> {
        self.lobby.write().await.add_table(after_id, table_to_add)
    }
}
