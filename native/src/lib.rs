#[macro_use]
extern crate napi_derive;

pub mod lexer;
pub mod token;

use lexer::{JsonishLexer, MaterialLexer};
use token::Token;

#[napi]
pub fn tokenize_material(input: String) -> Vec<Token> {
    let mut lexer = MaterialLexer::new(&input);
    lexer.tokenize()
}

#[napi]
pub fn tokenize_jsonish(input: String) -> Vec<Token> {
    let mut lexer = JsonishLexer::new(&input);
    lexer.tokenize()
}

#[napi]
pub fn hello() -> String {
    "Hello from filament-mat-core!".to_string()
}

#[cfg(test)]
mod tests {
    use crate::lexer::{JsonishLexer, MaterialLexer};

    #[test]
    fn test_material_lexer_basic() {
        let input = "material { } vertex { } fragment { }";
        let mut lexer = MaterialLexer::new(input);
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_jsonish_lexer_basic() {
        let input = r#"{ name: "Test", shadingModel: lit }"#;
        let mut lexer = JsonishLexer::new(input);
        let tokens = lexer.tokenize();
        assert!(!tokens.is_empty());
    }
}
