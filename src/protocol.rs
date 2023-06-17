use serde::{Deserialize, Serialize};

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct Seq(u64);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct Username(String);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct Password(String);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct TableId(i64);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
struct TableName(String);

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
struct TableToAdd {
    name: TableName,
    participants: u64,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
struct Table {
    id: TableId,
    name: TableName,
    participants: u64,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
enum UserType {
    User,
    Admin,
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
enum In {
    Ping { seq: Seq },
    Login { username: Username, password: Password },
    SubscribeTables,
    UnsubscribeTables,
    AddTable { after_id: TableId, table_to_add: TableToAdd },
    UpdateTable { table: Table },
    RemoveTable { id: TableId },
}

#[derive(Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
enum Out {
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
    use crate::protocol::*;
    use crate::protocol::Out::*;

    #[test]
    fn provide_correct_out_encoders() {
        let test_data = HashMap::from([
            (
                TableAdded {
                    after_id: TableId(-1),
                    table: Table {
                        id: TableId(3),
                        name: TableName(String::from("table - Foo Fighters")),
                        participants: 4,
                    },
                },
                json!({
                    "$type": "table_added",
                    "after_id": -1,
                    "table": {
                        "id": 3,
                        "name": "table - Foo Fighters",
                        "participants": 4
                    }
                })
            ),
        ]);

        for (out, expected_value) in test_data {
            let actual = serde_json::to_string(&out).expect("Failed to serialize to string");
            let actual_value: Value = serde_json::from_str(&actual).expect("Failed to deserialize to JSON");
            assert_eq!(actual_value, expected_value);
        }
    }
}
