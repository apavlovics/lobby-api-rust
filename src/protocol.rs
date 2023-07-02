use std::sync::atomic::{Ordering, AtomicIsize};
use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, EnumIter};

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Seq(u64);

#[derive(Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Username(pub String);

#[derive(Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TableName(pub String);

#[derive(Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Hash, Eq, EnumDiscriminants, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
#[strum_discriminants(derive(EnumIter))]
pub enum Input {
    Ping { seq: Seq },
    Login { username: Username, password: Password },
    SubscribeTables,
    UnsubscribeTables,
    AddTable { after_id: TableId, table: TableToAdd },
    UpdateTable { table: Table },
    RemoveTable { id: TableId },
}

#[derive(Clone, Hash, Eq, EnumDiscriminants, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename_all = "snake_case")]
#[strum_discriminants(derive(EnumIter))]
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

    use serde_json::{Value, json};
    use strum::IntoEnumIterator;

    use super::{Input, InputDiscriminants, Output, OutputDiscriminants, test_data};

    #[test]
    fn provide_correct_input_decoders() {

        fn verify(str: &str, expected_input: Input) {
            let actual_input: Input = serde_json::from_str(str).expect("Failed to deserialize input");
            assert_eq!(actual_input, expected_input);
        }

        // Ensure at compile time that all input variants are verified
        for input_discriminant in InputDiscriminants::iter() {
            match input_discriminant {
                InputDiscriminants::Ping => verify(
                    r#"{
                        "$type": "ping",
                        "seq": 12345
                    }"#,
                    test_data::ping(),
                ),
                InputDiscriminants::Login => verify(
                    r#"{
                        "$type": "login",
                        "username": "user",
                        "password": "pass"
                    }"#,
                    test_data::login(),
                ),
                InputDiscriminants::SubscribeTables => verify(
                    r#"{
                        "$type": "subscribe_tables"
                    }"#,
                    test_data::subscribe_tables(),
                ),
                InputDiscriminants::UnsubscribeTables =>verify(
                    r#"{
                        "$type": "unsubscribe_tables"
                    }"#,
                    test_data::unsubscribe_tables(),
                ),
                InputDiscriminants::AddTable => verify(
                    r#"{
                        "$type": "add_table",
                        "after_id": -1,
                        "table": {
                          "name": "table - Foo Fighters",
                          "participants": 4
                        }
                    }"#,
                    test_data::add_table(),
                ),
                InputDiscriminants::UpdateTable => verify(
                    r#"{
                        "$type": "update_table",
                        "table": {
                          "id": 3,
                          "name": "table - Foo Fighters",
                          "participants": 4
                        }
                    }"#,
                    test_data::update_table(),
                ),
                InputDiscriminants::RemoveTable => verify(
                    r#"{
                        "$type": "remove_table",
                        "id": 3
                    }"#,
                    test_data::remove_table(),
                ),
            }
        }
    }

    #[test]
    fn provide_correct_output_encoders() {

        fn verify(output: Output, expected_value: Value) {
            let actual_string = serde_json::to_string(&output).expect("Failed to serialize output");
            let actual_value: Value = serde_json::from_str(&actual_string).expect("Failed to deserialize to JSON");
            assert_eq!(actual_value, expected_value);
        }

        // Ensure at compile time that all output variants are verified
        for output_discriminant in OutputDiscriminants::iter() {
            match output_discriminant {
                OutputDiscriminants::LoginSuccessful => {
                    verify(
                        test_data::login_successful_user(),
                        json!({
                            "$type": "login_successful",
                            "user_type": "user"
                        }),
                    );
                    verify(
                        test_data::login_successful_admin(),
                        json!({
                            "$type": "login_successful",
                            "user_type": "admin"
                        }),
                    );
                },
                OutputDiscriminants::LoginFailed => verify(
                    test_data::login_failed(),
                    json!({
                        "$type": "login_failed"
                    }),
                ),
                OutputDiscriminants::Pong => verify(
                    test_data::pong(),
                    json!({
                        "$type": "pong",
                        "seq": 12345
                    }),
                ),
                OutputDiscriminants::TableList => verify(
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
                OutputDiscriminants::TableAdded => verify(
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
                OutputDiscriminants::TableUpdated => verify(
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
                OutputDiscriminants::TableRemoved => verify(
                    test_data::table_removed(),
                    json!({
                        "$type": "table_removed",
                        "id": 3
                    }),
                ),
                OutputDiscriminants::TableAddFailed => verify(
                    test_data::table_add_failed(),
                    json!({
                        "$type": "table_add_failed"
                    }),
                ),
                OutputDiscriminants::TableUpdateFailed => verify(
                    test_data::table_update_failed(),
                    json!({
                        "$type": "table_update_failed",
                        "id": 99999
                    }),
                ),
                OutputDiscriminants::TableRemoveFailed => verify(
                    test_data::table_remove_failed(),
                    json!({
                        "$type": "table_remove_failed",
                        "id": 99999
                    }),
                ),
                OutputDiscriminants::NotAuthorized => verify(
                    test_data::not_authorized(),
                    json!({
                        "$type": "not_authorized"
                    }),
                ),
                OutputDiscriminants::NotAuthenticated => verify(
                    test_data::not_authenticated(),
                    json!({
                        "$type": "not_authenticated"
                    }),
                ),
                OutputDiscriminants::InvalidMessage => verify(
                    test_data::invalid_message(),
                    json!({
                        "$type": "invalid_message"
                    }),
                ),
            }
        }
    }
}

#[cfg(test)]
pub mod test_data {

    use super::*;
    use super::Input::*;
    use super::Output::*;

    // Common

    const TABLE_ID_INVALID: TableId = TableId(99999);

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

    pub fn table_to_add_foo_fighters() -> TableToAdd {
        TableToAdd {
            name: TableName(String::from("table - Foo Fighters")),
            participants: 4,
        }
    }

    // Input

    pub fn login() -> Input {
        Login {
            username: Username(String::from("user")),
            password: Password(String::from("pass")),
        }
    }

    pub fn ping() -> Input {
        Ping {
            seq: Seq(12345),
        }
    }

    pub fn subscribe_tables() -> Input {
        SubscribeTables
    }

    pub fn unsubscribe_tables() -> Input {
        UnsubscribeTables
    }

    pub fn add_table() -> Input {
        AddTable {
            after_id: TableId::ABSENT,
            table: table_to_add_foo_fighters(),
        }
    }

    pub fn update_table() -> Input {
        UpdateTable {
            table: table_foo_fighters(),
        }
    }

    pub fn remove_table() -> Input {
        RemoveTable {
            id: TableId(3),
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

    pub fn table_removed() -> Output {
        TableRemoved {
            id: TableId(3),
        }
    }

    pub fn table_add_failed() -> Output {
        TableAddFailed
    }

    pub fn table_update_failed() -> Output {
        TableUpdateFailed { id: TABLE_ID_INVALID }
    }

    pub fn table_remove_failed() -> Output {
        TableRemoveFailed { id: TABLE_ID_INVALID }
    }

    pub fn not_authorized() -> Output {
        NotAuthorized
    }

    pub fn not_authenticated() -> Output {
        NotAuthenticated
    }

    pub fn invalid_message() -> Output {
        InvalidMessage
    }
}
