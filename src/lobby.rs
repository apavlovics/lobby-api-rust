use crate::protocol::{Table, TableId, TableName};

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
