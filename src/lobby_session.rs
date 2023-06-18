use crate::protocol::{Input, Output};
use crate::protocol::Input::*;
use crate::protocol::Output::*;

pub struct ProcessResult {
    pub output: Option<Output>,
    pub push_outputs: Vec<Output>,
}

pub fn process(input: Input) -> ProcessResult {
    match input {
        Ping { seq } => ProcessResult {
            output: Some(Pong { seq }),
            push_outputs: Vec::new(),
        },
        Login { username, password } => todo!(),
        SubscribeTables => todo!(),
        UnsubscribeTables => todo!(),
        AddTable { after_id, table_to_add } => todo!(),
        UpdateTable { table } => todo!(),
        RemoveTable { id } => todo!(),
    }
}
