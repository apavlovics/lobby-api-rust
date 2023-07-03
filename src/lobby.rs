use std::sync::Arc;
use tokio::sync::RwLock;

use crate::protocol::{Table, TableId, TableName, TableToAdd};

/// Represents the lobby that contains ordered tables.
pub struct Lobby {
    pub tables: Vec<Table>,
}
impl Lobby {

    pub fn prepopulated() -> Self {
        Lobby {
            tables: vec![
                Table {
                    id: TableId::new(),
                    name: TableName::new(String::from("table - James Bond")),
                    participants: 7,
                },
                Table {
                    id: TableId::new(),
                    name: TableName::new(String::from("table - Mission Impossible")),
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
                    Err(format!("Cannot find table {:?}, after which another table should be added", after_id))
                }
            }
        }
    }

    pub fn update_table(&mut self, table_to_update: Table) -> Result<Table, String> {
        match self.tables.iter_mut().find(|table| table.id == table_to_update.id) {
            Some(table) => {
                table.update_with(table_to_update);
                Ok(table.clone())
            }
            None => {
                Err(format!("Cannot find table {:?}, which should be updated", table_to_update.id))
            },
        }
    }

    pub fn remove_table(&mut self, id: TableId) -> Result<TableId, String> {
        match self.tables.iter().position(|table| table.id == id) {
            Some(index) => {
                self.tables.remove(index);
                Ok(id)
            }
            None => {
                Err(format!("Cannot find table {:?}, which should be removed", id))
            }
        }
    }
}

/// Represents the lobby that is shared among all the clients.
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

    pub async fn update_table(&self, table_to_update: Table) -> Result<Table, String> {
        self.lobby.write().await.update_table(table_to_update)
    }

    pub async fn remove_table(&self, id: TableId) -> Result<TableId, String> {
        self.lobby.write().await.remove_table(id)
    }
}

#[cfg(test)]
mod tests {

    use crate::protocol::{TableId, test_data};

    use super::Lobby;

    #[test]
    fn add_table_in_front() {
        let mut lobby = Lobby::prepopulated();
        let result = lobby.add_table(TableId::ABSENT, test_data::table_to_add_foo_fighters());

        let added_table = result.expect("Success result expected");
        let first_table = lobby.tables.first().expect("First table must be present");
        assert_eq!(&added_table, first_table);
    }
}
