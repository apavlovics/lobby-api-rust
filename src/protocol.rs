use std::sync::atomic::{Ordering, AtomicIsize};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seq(u64);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Username(pub String);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Password(pub String);

/// The global unique table id generator.
static NEXT_TABLE_ID: AtomicIsize = AtomicIsize::new(1);

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TableId(pub isize);
impl TableId {

    pub fn new() -> Self {
        TableId(NEXT_TABLE_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Table id to use as an absent (special, nonexistent) value.
    pub const ABSENT: TableId = TableId(-1);
}

#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TableName(pub String);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct TableToAdd {
    pub name: TableName,
    pub participants: u64,
}
impl TableToAdd {

    pub fn into_table(self, id: TableId) -> Table {
        Table {
            id,
            name: self.name,
            participants: self.participants,
        }
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: TableId,
    pub name: TableName,
    pub participants: u64,
}
impl Table {

    /// Updates all fields of this table with fields of the other table, excluding id.
    pub fn update_with(&mut self, other: Table) {
        self.name = other.name;
        self.participants = other.participants;
    }
}

#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserType {
    User,
    Admin,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
pub enum Input {
    Ping { seq: Seq },
    Login { username: Username, password: Password },
    SubscribeTables,
    UnsubscribeTables,
    AddTable { after_id: TableId, table: TableToAdd },
    UpdateTable { table: Table },
    RemoveTable { id: TableId },
}

#[derive(Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
pub enum Output {
    LoginSuccessful { user_type: UserType },
    LoginFailed,
    Pong { seq : Seq },
    TableList { tables: Vec<Table> },
    TableAdded { after_id: TableId, table: Table },
    TableUpdated { table: Table },
    TableRemoved { id: TableId },
    TableAddFailed,
    TableUpdateFailed { id: TableId },
    TableRemoveFailed { id: TableId },
    NotAuthorized,
    NotAuthenticated,
    InvalidMessage,
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;
    use serde_json::{Value, json};

    #[test]
    fn provide_correct_out_encoders() {

        // TODO Add similar tests for all other messages
        let test_data = HashMap::from([
            (
                test_data::login_successful_user(),
                json!({
                    "$type": "login_successful",
                    "user_type": "user"
                }),
            ),
            (
                test_data::login_successful_admin(),
                json!({
                    "$type": "login_successful",
                    "user_type": "admin"
                }),
            ),
            (
                test_data::login_successful_admin(),
                json!({
                    "$type": "login_successful",
                    "user_type": "admin"
                }),
            ),
            (
                test_data::login_failed(),
                json!({
                    "$type": "login_failed"
                }),
            ),
            (
                test_data::pong(),
                json!({
                    "$type": "pong",
                    "seq": 12345
                }),
            ),
            (
                test_data::table_list(),
                json!({
                    "$type": "table_list",
                    "tables": [
                      {
                        "id": 1,
                        "name": "table - James Bond",
                        "participants": 7
                      }, {
                        "id": 2,
                        "name": "table - Mission Impossible",
                        "participants": 9
                      }
                    ]
                  }),
            ),
            (
                test_data::table_added(),
                json!({
                    "$type": "table_added",
                    "after_id": -1,
                    "table": {
                        "id": 3,
                        "name": "table - Foo Fighters",
                        "participants": 4
                    }
                }),
            ),
            (
                test_data::table_updated(),
                json!({
                    "$type": "table_updated",
                    "table": {
                        "id": 3,
                        "name": "table - Foo Fighters",
                        "participants": 4
                    }
                }),
            ),
        ]);

        for (out, expected_value) in test_data {
            let actual = serde_json::to_string(&out).expect("Failed to serialize to string");
            let actual_value: Value = serde_json::from_str(&actual).expect("Failed to deserialize to JSON");
            assert_eq!(actual_value, expected_value);
        }
    }

    mod test_data {

        use crate::protocol::{Output, UserType, TableId, Table, TableName, Seq};
        use crate::protocol::Output::*;

        // Common

        fn table_james_bond() -> Table {
            Table {
                id: TableId(1),
                name: TableName(String::from("table - James Bond")),
                participants: 7,
            }
        }

        fn table_mission_impossible() -> Table {
            Table {
                id: TableId(2),
                name: TableName(String::from("table - Mission Impossible")),
                participants: 9,
            }
        }

        fn table_foo_fighters() -> Table {
            Table {
                id: TableId(3),
                name: TableName(String::from("table - Foo Fighters")),
                participants: 4,
            }
        }

        // Output

        pub fn login_successful_user() -> Output {
            LoginSuccessful {
                user_type: UserType::User,
            }
        }

        pub fn login_successful_admin() -> Output {
            LoginSuccessful {
                user_type: UserType::Admin,
            }
        }

        pub fn login_failed() -> Output {
            LoginFailed
        }

        pub fn pong() -> Output {
            Pong {
                seq: Seq(12345),
            }
        }

        pub fn table_list() -> Output {
            TableList {
                tables: vec![
                    table_james_bond(),
                    table_mission_impossible(),
                ],
            }
        }

        pub fn table_added() -> Output {
            TableAdded {
                after_id: TableId::ABSENT,
                table: table_foo_fighters(),
            }
        }

        pub fn table_updated() -> Output {
            TableUpdated {
                table: table_foo_fighters(),
            }
        }
    }
}
