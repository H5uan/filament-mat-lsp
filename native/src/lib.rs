#[macro_use]
extern crate napi_derive;

pub mod lexer;
pub mod token;

use napi::Result;
use token::Token;

#[napi]
pub fn tokenize(input: String) -> Result<Vec<Token>> {
    Ok(lexer::tokenize(&input))
}

#[napi]
pub fn hello() -> String {
    "Hello from filament-mat-core!".to_string()
}
