use crate::diagnostics::{TextPosition, TextRange};
use crate::token::{Token, TokenExt, TokenType};
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
  pub message: String,
  pub range: TextRange,
}

impl ParseError {
  fn new(message: impl Into<String>, range: TextRange) -> Self {
    Self {
      message: message.into(),
      range,
    }
  }

  fn at_token(message: impl Into<String>, token: &Token) -> Self {
    let range = TextRange {
      start: TextPosition {
        line: token.line,
        character: token.column,
      },
      end: TextPosition {
        line: token.line,
        character: token.column + token.value.len() as u32,
      },
    };
    Self::new(message, range)
  }

  fn at_eof(expected: &str) -> Self {
    Self::new(
      format!("Unexpected end of file, expected {}", expected),
      TextRange {
        start: TextPosition {
          line: 0,
          character: 0,
        },
        end: TextPosition {
          line: 0,
          character: 0,
        },
      },
    )
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Located<T> {
  pub value: T,
  pub range: TextRange,
}

impl<T> Located<T> {
  pub fn new(value: T, range: TextRange) -> Self {
    Self { value, range }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
  pub range: TextRange,
  pub name: Option<Located<String>>,
  pub shading_model: Option<Located<String>>,
  pub requires: Located<Vec<String>>,
  pub parameters: Vec<Parameter>,
  pub other_properties: Vec<(String, Located<Value>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
  pub param_type: String,
  pub name: String,
  pub other_fields: Vec<(String, Value)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
  String(String),
  Number(f64),
  Bool(bool),
  Array(Vec<Value>),
  Object(Vec<(String, Value)>),
  Identifier(String),
  Null,
}

pub struct Parser {
  tokens: Peekable<IntoIter<Token>>,
  last_token: Option<Token>,
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Self {
    Self {
      tokens: tokens.into_iter().peekable(),
      last_token: None,
    }
  }

  fn token_pos(token: &Token) -> TextPosition {
    TextPosition {
      line: token.line,
      character: token.column,
    }
  }

  fn make_range(start: &Token, end: &Token) -> TextRange {
    TextRange {
      start: Self::token_pos(start),
      end: TextPosition {
        line: end.line,
        character: end.column + end.value.len() as u32,
      },
    }
  }

  pub fn parse_material(&mut self) -> Result<Material, ParseError> {
    let start_token = match self.tokens.peek() {
      Some(t) => t.clone(),
      None => return Err(ParseError::at_eof("material block")),
    };
    self.skip_to(&TokenType::LCurly);
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LCurly)
    {
      self.tokens.next();
    }

    let mut material = Material {
      range: TextRange {
        start: Self::token_pos(&start_token),
        end: TextPosition {
          line: 0,
          character: 0,
        },
      },
      name: None,
      shading_model: None,
      requires: Located::new(
        Vec::new(),
        TextRange {
          start: TextPosition {
            line: 0,
            character: 0,
          },
          end: TextPosition {
            line: 0,
            character: 0,
          },
        },
      ),
      parameters: Vec::new(),
      other_properties: Vec::new(),
    };

    while let Some(token) = self.tokens.peek() {
      if token.is_type(&TokenType::RCurly) {
        let end_token = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("closing brace"))?;
        material.range.end = TextPosition {
          line: end_token.line,
          character: end_token.column + 1,
        };
        break;
      }

      match token.token_type {
        TokenType::Name => {
          let name_token = self
            .tokens
            .next()
            .ok_or_else(|| ParseError::at_eof("property name"))?;
          self.expect(&TokenType::Colon);
          if let Ok(value) = self.parse_value() {
            let end_token = self.last_token.as_ref().unwrap_or(&name_token);
            let range = Self::make_range(&name_token, end_token);
            if let Value::Identifier(s) | Value::String(s) = value {
              material.name = Some(Located::new(s, range));
            }
          }
        }
        TokenType::ShadingModel => {
          let sm_token = self
            .tokens
            .next()
            .ok_or_else(|| ParseError::at_eof("shadingModel property"))?;
          self.expect(&TokenType::Colon);
          if let Ok(value) = self.parse_value() {
            let end_token = self.last_token.as_ref().unwrap_or(&sm_token);
            let range = Self::make_range(&sm_token, end_token);
            if let Value::Identifier(s) = value {
              material.shading_model = Some(Located::new(s, range));
            }
          }
        }
        TokenType::Requires => {
          let req_token = self
            .tokens
            .next()
            .ok_or_else(|| ParseError::at_eof("requires property"))?;
          self.expect(&TokenType::Colon);
          if let Ok(Value::Array(arr)) = self.parse_value() {
            let end_token = self.last_token.as_ref().unwrap_or(&req_token);
            let range = Self::make_range(&req_token, end_token);
            let mut items = Vec::new();
            for item in arr {
              if let Value::Identifier(s) = item {
                items.push(s);
              }
            }
            material.requires = Located::new(items, range);
          }
        }
        TokenType::Parameters => {
          self
            .tokens
            .next()
            .ok_or_else(|| ParseError::at_eof("parameters property"))?;
          self.expect(&TokenType::Colon);
          if let Ok(Value::Array(arr)) = self.parse_value() {
            for item in arr {
              if let Value::Object(props) = item
                && let Ok(param) = Self::parse_parameter(props)
              {
                material.parameters.push(param);
              }
            }
          }
        }
        _ => {
          let key_token = self
            .tokens
            .next()
            .ok_or_else(|| ParseError::at_eof("property key"))?;
          if key_token.is_type(&TokenType::Identifier) {
            self.expect(&TokenType::Colon);
            if let Ok(value) = self.parse_value() {
              let end_token = self.last_token.as_ref().unwrap_or(&key_token);
              let range = Self::make_range(&key_token, end_token);
              material
                .other_properties
                .push((key_token.value.clone(), Located::new(value, range)));
            }
          }
        }
      }
    }

    Ok(material)
  }

  fn parse_parameter(mut props: Vec<(String, Value)>) -> Result<Parameter, ParseError> {
    let mut param_type = None;
    let mut name = None;
    let mut other_fields = Vec::new();

    for (k, v) in props.drain(..) {
      match k.to_lowercase().as_str() {
        "type" => {
          if let Value::Identifier(s) = v {
            param_type = Some(s);
          }
        }
        "name" => {
          if let Value::Identifier(s) = v {
            name = Some(s);
          }
        }
        _ => other_fields.push((k, v)),
      }
    }

    let param_type = param_type.ok_or_else(|| {
      ParseError::new(
        "Parameter is missing 'type' field",
        TextRange {
          start: TextPosition {
            line: 0,
            character: 0,
          },
          end: TextPosition {
            line: 0,
            character: 0,
          },
        },
      )
    })?;
    let name = name.ok_or_else(|| {
      ParseError::new(
        "Parameter is missing 'name' field",
        TextRange {
          start: TextPosition {
            line: 0,
            character: 0,
          },
          end: TextPosition {
            line: 0,
            character: 0,
          },
        },
      )
    })?;

    Ok(Parameter {
      param_type,
      name,
      other_fields,
    })
  }

  fn parse_value(&mut self) -> Result<Value, ParseError> {
    let token = match self.tokens.peek() {
      Some(t) => t.clone(),
      None => return Err(ParseError::at_eof("value")),
    };
    match token.token_type {
      TokenType::String => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("string value"))?;
        self.last_token = Some(t.clone());
        if !t.value.starts_with('"') || !t.value.ends_with('"') {
          return Err(ParseError::at_token("Invalid string literal", &t));
        }
        let s = t.value[1..t.value.len() - 1].to_string();
        Ok(Value::String(s))
      }
      TokenType::Number => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("number value"))?;
        self.last_token = Some(t.clone());
        let n = t
          .value
          .parse()
          .map_err(|_| ParseError::at_token("Invalid number", &t))?;
        Ok(Value::Number(n))
      }
      TokenType::True => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("boolean value"))?;
        self.last_token = Some(t);
        Ok(Value::Bool(true))
      }
      TokenType::False => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("boolean value"))?;
        self.last_token = Some(t);
        Ok(Value::Bool(false))
      }
      TokenType::Null => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("null value"))?;
        self.last_token = Some(t);
        Ok(Value::Null)
      }
      TokenType::Identifier
      | TokenType::Lit
      | TokenType::Unlit
      | TokenType::Float
      | TokenType::Sampler2d
      | TokenType::Back
      | TokenType::None
      | TokenType::Opaque
      | TokenType::Object
      | TokenType::Uv0
      | TokenType::Color
      | TokenType::Position
      | TokenType::Normal => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("identifier"))?;
        self.last_token = Some(t.clone());
        Ok(Value::Identifier(t.value))
      }
      TokenType::LBracket => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("opening bracket"))?;
        self.last_token = Some(t);
        let mut arr = Vec::new();
        while let Some(t) = self.tokens.peek() {
          if t.is_type(&TokenType::RBracket) {
            let t = self
              .tokens
              .next()
              .ok_or_else(|| ParseError::at_eof("closing bracket"))?;
            self.last_token = Some(t);
            break;
          }
          if let Ok(val) = self.parse_value() {
            arr.push(val);
          }
          if let Some(t) = self.tokens.peek()
            && t.is_type(&TokenType::Comma)
          {
            let t = self
              .tokens
              .next()
              .ok_or_else(|| ParseError::at_eof("comma"))?;
            self.last_token = Some(t);
          }
        }
        Ok(Value::Array(arr))
      }
      TokenType::LCurly => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("opening brace"))?;
        self.last_token = Some(t);
        let mut obj = Vec::new();
        while let Some(t) = self.tokens.peek() {
          if t.is_type(&TokenType::RCurly) {
            let t = self
              .tokens
              .next()
              .ok_or_else(|| ParseError::at_eof("closing brace"))?;
            self.last_token = Some(t);
            break;
          }
          if let Ok(key) = self.parse_object_key() {
            self.expect(&TokenType::Colon);
            if let Ok(value) = self.parse_value() {
              obj.push((key, value));
            }
          }
          if let Some(t) = self.tokens.peek()
            && t.is_type(&TokenType::Comma)
          {
            let t = self
              .tokens
              .next()
              .ok_or_else(|| ParseError::at_eof("comma"))?;
            self.last_token = Some(t);
          }
        }
        Ok(Value::Object(obj))
      }
      _ => Err(ParseError::at_token(
        format!("Unexpected token: {}", token.value),
        &token,
      )),
    }
  }

  fn parse_object_key(&mut self) -> Result<String, ParseError> {
    let token = match self.tokens.peek() {
      Some(t) => t.clone(),
      None => return Err(ParseError::at_eof("object key")),
    };
    if token.is_type(&TokenType::Identifier)
      || token.is_type(&TokenType::Type)
      || token.is_type(&TokenType::Parameters)
      || token.is_type(&TokenType::Name)
      || token.is_type(&TokenType::Requires)
      || token.is_type(&TokenType::ShadingModel)
    {
      let t = self
        .tokens
        .next()
        .ok_or_else(|| ParseError::at_eof("object key"))?;
      return Ok(t.value);
    }
    Err(ParseError::at_token("Expected object key", &token))
  }

  fn skip_to(&mut self, target: &TokenType) {
    while let Some(token) = self.tokens.peek() {
      if token.is_type(target) {
        break;
      }
      self.tokens.next();
    }
  }

  fn expect(&mut self, expected: &TokenType) {
    if let Some(token) = self.tokens.peek()
      && token.is_type(expected)
    {
      self.tokens.next();
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lexer::JsonishLexer;

  #[test]
  fn test_parse_simple_material() {
    let input = r#"{ name: TestMat, shadingModel: lit }"#;
    let mut lexer = JsonishLexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let result = parser.parse_material();
    assert!(result.is_ok());
  }
}
