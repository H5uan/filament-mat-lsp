use crate::token::{Token, TokenType};
use std::iter::Peekable;
use std::str::Chars;

// ---------------------------------------------------------------------------
// Lexer state
// ---------------------------------------------------------------------------
struct LexerState<'a> {
  chars: Peekable<Chars<'a>>,
  line: u32,
  column: u32,
}

impl<'a> LexerState<'a> {
  fn new(input: &'a str) -> Self {
    Self {
      chars: input.chars().peekable(),
      line: 1,
      column: 1,
    }
  }

  fn advance(&mut self) -> Option<char> {
    let ch = self.chars.next()?;
    match ch {
      '\n' => {
        self.line += 1;
        self.column = 1;
      }
      '\r' => {
        if let Some('\n') = self.chars.peek() {
          self.chars.next();
        }
        self.line += 1;
        self.column = 1;
      }
      _ => self.column += 1,
    }
    Some(ch)
  }

  fn peek(&mut self) -> Option<&char> {
    self.chars.peek()
  }

  fn current_pos(&self) -> (u32, u32) {
    (self.line, self.column)
  }
}

// ---------------------------------------------------------------------------
// Lexer modes
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
enum LexerMode {
  TopLevel,
  MaterialBlock { brace_depth: usize },
  ShaderBlock { brace_depth: usize },
}

// ---------------------------------------------------------------------------
// Unified Lexer
// ---------------------------------------------------------------------------
pub struct Lexer<'a> {
  state: LexerState<'a>,
  mode: LexerMode,
  pending_block_name: Option<String>,
}

impl<'a> Lexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      state: LexerState::new(input),
      mode: LexerMode::TopLevel,
      pending_block_name: None,
    }
  }

  pub fn tokenize(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    while let Some(&ch) = self.state.peek() {
      match &self.mode {
        LexerMode::TopLevel => {
          if ch.is_whitespace() {
            self.state.advance();
            continue;
          }
          if ch == '/' {
            let comment = self.read_comment();
            tokens.push(comment);
            continue;
          }

          // Try to read an identifier (top-level block keyword)
          if ch.is_ascii_alphabetic() || ch == '_' {
            let (line, col) = self.state.current_pos();
            let ident = self.read_identifier_raw();
            let tt = if is_toplevel_block_keyword(&ident) {
              self.pending_block_name = Some(ident.clone());
              TokenType::BlockKeyword
            } else {
              TokenType::Identifier
            };
            tokens.push(Token::new(tt, &ident, line, col));
            continue;
          }

          // Expect '{' after a top-level block keyword
          if ch == '{' {
            let (line, col) = self.state.current_pos();
            self.state.advance();
            tokens.push(Token::new(TokenType::LCurly, "{", line, col));

            // Switch mode based on pending block name
            match self.pending_block_name.take().as_deref() {
              Some("material") => {
                self.mode = LexerMode::MaterialBlock { brace_depth: 1 };
              }
              Some(_) => {
                self.mode = LexerMode::ShaderBlock { brace_depth: 1 };
              }
              None => {
                // Stray '{', stay in TopLevel
              }
            }
            continue;
          }

          // Unknown character at top level - skip
          let (line, col) = self.state.current_pos();
          let s = ch.to_string();
          self.state.advance();
          tokens.push(Token::new(TokenType::Unknown, &s, line, col));
        }

        LexerMode::MaterialBlock { brace_depth } => {
          let depth = *brace_depth;
          if let Some(tok) = self.tokenize_material_token(depth) {
            match tok.token_type {
              TokenType::LCurly => {
                self.mode = LexerMode::MaterialBlock {
                  brace_depth: depth + 1,
                };
              }
              TokenType::RCurly => {
                if depth == 1 {
                  self.mode = LexerMode::TopLevel;
                } else {
                  self.mode = LexerMode::MaterialBlock {
                    brace_depth: depth - 1,
                  };
                }
              }
              _ => {}
            }
            tokens.push(tok);
          }
        }

        LexerMode::ShaderBlock { brace_depth } => {
          let depth = *brace_depth;
          if let Some((tok, new_depth)) = self.tokenize_shader_token(depth) {
            if tok.token_type == TokenType::RCurly && new_depth == 0 {
              self.mode = LexerMode::TopLevel;
            } else {
              self.mode = LexerMode::ShaderBlock {
                brace_depth: new_depth,
              };
            }
            tokens.push(tok);
          }
        }
      }
    }
    tokens
  }

  // -------------------------------------------------------------------------
  // Material block tokenization (Jsonish)
  // -------------------------------------------------------------------------
  fn tokenize_material_token(&mut self, _brace_depth: usize) -> Option<Token> {
    let ch = loop {
      let &c = self.state.peek()?;

      if c.is_whitespace() {
        self.state.advance();
        continue;
      }

      if c == '/' {
        return Some(self.read_comment());
      }

      break c;
    };

    let (line, col) = self.state.current_pos();

    match ch {
      '{' => {
        self.state.advance();
        Some(Token::new(TokenType::LCurly, "{", line, col))
      }
      '}' => {
        self.state.advance();
        Some(Token::new(TokenType::RCurly, "}", line, col))
      }
      '[' => {
        self.state.advance();
        Some(Token::new(TokenType::LBracket, "[", line, col))
      }
      ']' => {
        self.state.advance();
        Some(Token::new(TokenType::RBracket, "]", line, col))
      }
      ':' => {
        self.state.advance();
        Some(Token::new(TokenType::Colon, ":", line, col))
      }
      ',' => {
        self.state.advance();
        Some(Token::new(TokenType::Comma, ",", line, col))
      }
      '"' => Some(self.read_string()),
      '0'..='9' | '-' => Some(self.read_number()),
      'a'..='z' | 'A'..='Z' | '_' => {
        let ident = self.read_identifier_raw();
        let tt = match ident.as_str() {
          "true" | "TRUE" => TokenType::BoolLiteral,
          "false" | "FALSE" => TokenType::BoolLiteral,
          "null" | "NULL" => TokenType::NullLiteral,
          _ => TokenType::Identifier,
        };
        Some(Token::new(tt, &ident, line, col))
      }
      _ => {
        let s = ch.to_string();
        self.state.advance();
        Some(Token::new(TokenType::Unknown, &s, line, col))
      }
    }
  }

  // -------------------------------------------------------------------------
  // Shader block tokenization (raw GLSL)
  // -------------------------------------------------------------------------
  fn tokenize_shader_token(&mut self, mut brace_depth: usize) -> Option<(Token, usize)> {
    let start_line = self.state.line;
    let start_col = self.state.column;
    let mut code = String::new();

    while let Some(&ch) = self.state.peek() {
      if ch == '{' {
        self.state.advance();
        brace_depth += 1;
        code.push('{');
      } else if ch == '}' {
        if brace_depth == 1 {
          // This is the closing brace of the shader block.
          // Return accumulated GLSL code WITHOUT consuming '}'.
          // Next call will see '}' and emit RCurly.
          if !code.is_empty() {
            return Some((
              Token::new(TokenType::GlslCode, &code, start_line, start_col),
              brace_depth,
            ));
          } else {
            // No code accumulated, consume '}' and return it
            self.state.advance();
            return Some((
              Token::new(TokenType::RCurly, "}", self.state.line, self.state.column),
              0,
            ));
          }
        } else {
          self.state.advance();
          brace_depth -= 1;
          code.push('}');
        }
      } else {
        code.push(ch);
        self.state.advance();
      }
    }

    // EOF while in shader block
    if !code.is_empty() {
      Some((
        Token::new(TokenType::GlslCode, &code, start_line, start_col),
        brace_depth,
      ))
    } else {
      None
    }
  }

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------
  fn read_comment(&mut self) -> Token {
    let (line, col) = self.state.current_pos();
    self.state.advance(); // '/'
    if let Some(&'/') = self.state.peek() {
      self.state.advance();
      let mut comment = String::from("//");
      while let Some(&ch) = self.state.peek() {
        if ch == '\n' {
          break;
        }
        comment.push(ch);
        self.state.advance();
      }
      Token::new(TokenType::Comment, &comment, line, col)
    } else if let Some(&'*') = self.state.peek() {
      self.state.advance();
      let mut comment = String::from("/*");
      while let Some(&ch) = self.state.peek() {
        comment.push(ch);
        self.state.advance();
        if ch == '*' && self.state.peek() == Some(&'/') {
          comment.push('/');
          self.state.advance();
          break;
        }
      }
      Token::new(TokenType::Comment, &comment, line, col)
    } else {
      Token::new(TokenType::Unknown, "/", line, col)
    }
  }

  fn read_string(&mut self) -> Token {
    let (line, col) = self.state.current_pos();
    self.state.advance(); // '"'
    let mut s = String::from("\"");
    while let Some(&ch) = self.state.peek() {
      if ch == '"' {
        s.push('"');
        self.state.advance();
        break;
      } else if ch == '\\' {
        s.push('\\');
        self.state.advance();
        if let Some(&escaped) = self.state.peek() {
          s.push(escaped);
          self.state.advance();
        }
      } else {
        s.push(ch);
        self.state.advance();
      }
    }
    Token::new(TokenType::StringLiteral, &s, line, col)
  }

  fn read_number(&mut self) -> Token {
    let (line, col) = self.state.current_pos();
    let mut num = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+' {
        num.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    Token::new(TokenType::NumberLiteral, &num, line, col)
  }

  fn read_identifier_raw(&mut self) -> String {
    let mut ident = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_alphanumeric() || ch == '_' {
        ident.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    ident
  }
}

/// Check if an identifier is a top-level block keyword.
fn is_toplevel_block_keyword(ident: &str) -> bool {
  matches!(
    ident,
    "material" | "vertex" | "fragment" | "compute" | "tool"
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::token::TokenExt;

  #[test]
  fn test_lex_toplevel_blocks() {
    let input = "material {} vertex {} fragment {} compute {}";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    assert_eq!(tokens.len(), 12);
    assert!(tokens[0].is_type(&TokenType::BlockKeyword));
    assert_eq!(tokens[0].value, "material");
    assert!(tokens[1].is_type(&TokenType::LCurly));
    assert!(tokens[2].is_type(&TokenType::RCurly));
    assert!(tokens[3].is_type(&TokenType::BlockKeyword));
    assert_eq!(tokens[3].value, "vertex");
    assert!(tokens[4].is_type(&TokenType::LCurly));
    assert!(tokens[5].is_type(&TokenType::RCurly));
    assert!(tokens[6].is_type(&TokenType::BlockKeyword));
    assert_eq!(tokens[6].value, "fragment");
    assert!(tokens[7].is_type(&TokenType::LCurly));
    assert!(tokens[8].is_type(&TokenType::RCurly));
    assert!(tokens[9].is_type(&TokenType::BlockKeyword));
    assert_eq!(tokens[9].value, "compute");
    assert!(tokens[10].is_type(&TokenType::LCurly));
    assert!(tokens[11].is_type(&TokenType::RCurly));
  }

  #[test]
  fn test_lex_material_properties() {
    let input = "material { name : Test, shadingModel : lit, blending : opaque }";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    // tokens[0] = BlockKeyword "material", tokens[1] = LCurly "{"
    // Then: name : Test, shadingModel : lit, blending : opaque
    assert_eq!(tokens[2].value, "name");
    assert!(tokens[2].is_type(&TokenType::Identifier));
    assert_eq!(tokens[4].value, "Test");
    assert!(tokens[4].is_type(&TokenType::Identifier));
    assert_eq!(tokens[6].value, "shadingModel");
    assert!(tokens[6].is_type(&TokenType::Identifier));
    assert_eq!(tokens[8].value, "lit");
    assert!(tokens[8].is_type(&TokenType::Identifier));
    assert_eq!(tokens[10].value, "blending");
    assert!(tokens[10].is_type(&TokenType::Identifier));
    assert_eq!(tokens[12].value, "opaque");
    assert!(tokens[12].is_type(&TokenType::Identifier));
  }

  #[test]
  fn test_lex_shader_block() {
    let input = r#"fragment {
      void material(inout MaterialInputs material) {
        prepareMaterial(material);
      }
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    // Should have BlockKeyword, LCurly, GlslCode, RCurly
    assert!(tokens[0].is_type(&TokenType::BlockKeyword));
    assert_eq!(tokens[0].value, "fragment");
    assert!(tokens[1].is_type(&TokenType::LCurly));
    // Find GlslCode token
    let has_glsl = tokens.iter().any(|t| t.is_type(&TokenType::GlslCode));
    assert!(has_glsl, "Expected GlslCode token in shader block");
    assert!(tokens.last().unwrap().is_type(&TokenType::RCurly));
  }

  #[test]
  fn test_lex_all_properties() {
    let input = r#"material {
      name : Test,
      apiLevel : 1,
      featureLevel : 2,
      shadingModel : lit,
      domain : surface,
      interpolation : smooth,
      quality : high,
      requires : [position, uv0],
      parameters : [{type : float4, name : color}],
      culling : back,
      blending : transparent,
      postLightingBlending : add,
      transparency : twoPassesOneSide,
      maskThreshold : 0.5,
      alphaToCoverage : true,
      vertexDomain : world,
      vertexDomainDeviceJittered : false,
      depthCulling : true,
      reflections : screenspace,
      specularAmbientOcclusion : bentNormals,
      variantFilter : [directionalLighting, shadowReceiver],
      stereoscopicType : multiview
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    // Should not contain any Unknown tokens
    let has_unknown = tokens.iter().any(|t| t.is_type(&TokenType::Unknown));
    assert!(
      !has_unknown,
      "Lexer produced Unknown tokens for known properties"
    );
  }
}
