use serde::Serialize;

/// Minimal token types — all semantic meaning is carried by the token value string.
/// The lexer no longer assigns specific types to keywords, property names, or enum values.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TokenType {
  /// Top-level block keyword: material, vertex, fragment, compute, tool
  BlockKeyword,
  /// Generic identifier (property names, enum values, type names, etc.)
  Identifier,
  /// String literal "like this"
  StringLiteral,
  /// Number literal 42 or 3.14
  NumberLiteral,
  /// Boolean literal true / false
  BoolLiteral,
  /// Null literal
  NullLiteral,
  /// {
  LCurly,
  /// }
  RCurly,
  /// [
  LBracket,
  /// ]
  RBracket,
  /// :
  Colon,
  /// ,
  Comma,
  /// Comment (// or /* */)
  Comment,
  /// Raw GLSL code inside a shader block
  GlslCode,
  /// Unknown/unrecognized token
  Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Token {
  pub token_type: TokenType,
  pub value: String,
  pub line: u32,
  pub column: u32,
}

impl Token {
  pub fn new(token_type: TokenType, value: &str, line: u32, column: u32) -> Self {
    Self {
      token_type,
      value: value.to_string(),
      line,
      column,
    }
  }
}

// Helper trait to make working with tokens easier
pub trait TokenExt {
  fn is_type(&self, token_type: &TokenType) -> bool;
}

impl TokenExt for Token {
  fn is_type(&self, token_type: &TokenType) -> bool {
    self.token_type == *token_type
  }
}
