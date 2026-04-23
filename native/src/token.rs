use napi_derive::napi;
use serde::Serialize;

#[napi(object)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Token {
    pub r#type: String,
    pub value: String,
    pub line: u32,
    pub column: u32,
}

impl Token {
    pub fn new(r#type: &str, value: &str, line: u32, column: u32) -> Self {
        Self {
            r#type: r#type.to_string(),
            value: value.to_string(),
            line,
            column,
        }
    }
}
