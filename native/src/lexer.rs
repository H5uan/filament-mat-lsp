use crate::token::{Token, TokenType};
use std::iter::Peekable;
use std::str::Chars;

// Shared state between both lexers
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
}

pub struct MaterialLexer<'a> {
  state: LexerState<'a>,
}

impl<'a> MaterialLexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      state: LexerState::new(input),
    }
  }

  pub fn tokenize(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    while let Some(&ch) = self.state.peek() {
      match ch {
        ' ' | '\t' => {
          self.state.advance();
        }
        '\n' | '\r' => {
          self.state.advance();
        }
        '/' => {
          self.skip_comment();
        }
        'a'..='z' | 'A'..='Z' | '_' => {
          if let Some(token) = self.read_keyword() {
            tokens.push(token);
          }
        }
        '{' => {
          let token = Token::new(TokenType::LCurly, "{", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        '}' => {
          let token = Token::new(TokenType::RCurly, "}", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        _ => {
          self.state.advance();
        }
      }
    }
    tokens
  }

  fn skip_comment(&mut self) {
    self.state.advance();
    if let Some(&'/') = self.state.peek() {
      self.state.advance();
      while let Some(&ch) = self.state.peek() {
        if ch == '\n' {
          break;
        }
        self.state.advance();
      }
    } else if let Some(&'*') = self.state.peek() {
      self.state.advance();
      while let Some(&ch) = self.state.peek() {
        if ch == '*' {
          self.state.advance();
          if let Some(&'/') = self.state.peek() {
            self.state.advance();
            break;
          }
        } else {
          self.state.advance();
        }
      }
    }
  }

  fn read_keyword(&mut self) -> Option<Token> {
    let start_column = self.state.column;
    let mut ident = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_alphanumeric() || ch == '_' {
        ident.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    match ident.as_str() {
      "material" => Some(Token::new(
        TokenType::Material,
        "material",
        self.state.line,
        start_column,
      )),
      "vertex" => Some(Token::new(
        TokenType::Vertex,
        "vertex",
        self.state.line,
        start_column,
      )),
      "fragment" => Some(Token::new(
        TokenType::Fragment,
        "fragment",
        self.state.line,
        start_column,
      )),
      "compute" => Some(Token::new(
        TokenType::Compute,
        "compute",
        self.state.line,
        start_column,
      )),
      _ => None,
    }
  }
}

pub struct JsonishLexer<'a> {
  state: LexerState<'a>,
}

impl<'a> JsonishLexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      state: LexerState::new(input),
    }
  }

  pub fn tokenize(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    while let Some(&ch) = self.state.peek() {
      match ch {
        ' ' | '\t' => {
          self.state.advance();
        }
        '\n' | '\r' => {
          self.state.advance();
        }
        '/' => {
          tokens.push(self.read_comment());
        }
        '"' => {
          tokens.push(self.read_string());
        }
        '0'..='9' | '-' | '.' => {
          tokens.push(self.read_number());
        }
        'a'..='z' | 'A'..='Z' | '_' => {
          tokens.push(self.read_identifier());
        }
        '{' => {
          let token = Token::new(TokenType::LCurly, "{", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        '}' => {
          let token = Token::new(TokenType::RCurly, "}", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        '[' => {
          let token = Token::new(TokenType::LBracket, "[", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        ']' => {
          let token = Token::new(TokenType::RBracket, "]", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        ':' => {
          let token = Token::new(TokenType::Colon, ":", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        ',' => {
          let token = Token::new(TokenType::Comma, ",", self.state.line, self.state.column);
          self.state.advance();
          tokens.push(token);
        }
        _ => {
          let token = Token::new(
            TokenType::Unknown,
            &ch.to_string(),
            self.state.line,
            self.state.column,
          );
          self.state.advance();
          tokens.push(token);
        }
      }
    }
    tokens
  }

  fn read_comment(&mut self) -> Token {
    let start_column = self.state.column;
    self.state.advance();
    if let Some(&'/') = self.state.peek() {
      let mut comment = String::from("//");
      self.state.advance();
      while let Some(&ch) = self.state.peek() {
        if ch == '\n' {
          break;
        }
        comment.push(ch);
        self.state.advance();
      }
      Token::new(TokenType::Comment, &comment, self.state.line, start_column)
    } else if let Some(&'*') = self.state.peek() {
      let mut comment = String::from("/*");
      self.state.advance();
      while let Some(&ch) = self.state.peek() {
        comment.push(ch);
        self.state.advance();
        if ch == '*' && self.state.peek() == Some(&'/') {
          comment.push('/');
          self.state.advance();
          break;
        }
      }
      Token::new(TokenType::Comment, &comment, self.state.line, start_column)
    } else {
      Token::new(TokenType::Unknown, "/", self.state.line, start_column)
    }
  }

  fn read_string(&mut self) -> Token {
    let start_column = self.state.column;
    self.state.advance();
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
    Token::new(TokenType::String, &s, self.state.line, start_column)
  }

  fn read_number(&mut self) -> Token {
    let start_column = self.state.column;
    let mut num = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+' {
        num.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    Token::new(TokenType::Number, &num, self.state.line, start_column)
  }

  fn read_identifier(&mut self) -> Token {
    let start_column = self.state.column;
    let mut ident = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_alphanumeric() || ch == '_' {
        ident.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    let token_type = match ident.as_str() {
      "name" => TokenType::Name,
      "shadingModel" => TokenType::ShadingModel,
      "requires" => TokenType::Requires,
      "parameters" => TokenType::Parameters,
      "type" => TokenType::Type,
      "true" => TokenType::True,
      "false" => TokenType::False,
      "lit" => TokenType::Lit,
      "unlit" => TokenType::Unlit,
      "float" => TokenType::Float,
      "sampler2d" => TokenType::Sampler2d,
      _ => TokenType::Identifier,
    };
    Token::new(token_type, &ident, self.state.line, start_column)
  }
}
