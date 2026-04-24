use crate::diagnostics::{TextPosition, TextRange};
use crate::token::{Token, TokenExt, TokenType};
use std::iter::Peekable;
use std::vec::IntoIter;

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

  pub fn parse_material(&mut self) -> Option<Material> {
    let start_token = self.tokens.peek()?.clone();
    self.skip_to(&TokenType::LCurly);
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LCurly)
    {
      self.tokens.next();
    }

    let mut material = Material {
      range: TextRange {
        start: Self::token_pos(&start_token),
        end: TextPosition { line: 0, character: 0 },
      },
      name: None,
      shading_model: None,
      requires: Located::new(Vec::new(), TextRange {
        start: TextPosition { line: 0, character: 0 },
        end: TextPosition { line: 0, character: 0 },
      }),
      parameters: Vec::new(),
      other_properties: Vec::new(),
    };

    while let Some(token) = self.tokens.peek() {
      if token.is_type(&TokenType::RCurly) {
        let end_token = self.tokens.next()?;
        material.range.end = TextPosition {
          line: end_token.line,
          character: end_token.column + 1,
        };
        break;
      }

      match token.token_type.as_str() {
        "Name" => {
          let name_token = self.tokens.next()?;
          self.expect(&TokenType::Colon);
          if let Some(value) = self.parse_value() {
            let end_token = self.last_token.as_ref().unwrap_or(&name_token);
            let range = Self::make_range(&name_token, end_token);
            if let Value::Identifier(s) | Value::String(s) = value {
              material.name = Some(Located::new(s, range));
            }
          }
        }
        "ShadingModel" => {
          let sm_token = self.tokens.next()?;
          self.expect(&TokenType::Colon);
          if let Some(value) = self.parse_value() {
            let end_token = self.last_token.as_ref().unwrap_or(&sm_token);
            let range = Self::make_range(&sm_token, end_token);
            if let Value::Identifier(s) = value {
              material.shading_model = Some(Located::new(s, range));
            }
          }
        }
        "Requires" => {
          let req_token = self.tokens.next()?;
          self.expect(&TokenType::Colon);
          if let Some(Value::Array(arr)) = self.parse_value() {
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
        "Parameters" => {
          let param_token = self.tokens.next()?;
          self.expect(&TokenType::Colon);
          if let Some(Value::Array(arr)) = self.parse_value() {
            for item in arr {
              if let Value::Object(props) = item
                && let Some(param) = Self::parse_parameter(props)
              {
                material.parameters.push(param);
              }
            }
          }
        }
        _ => {
          let key_token = self.tokens.next()?;
          if key_token.is_type(&TokenType::Identifier) {
            self.expect(&TokenType::Colon);
            if let Some(value) = self.parse_value() {
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

    Some(material)
  }

  fn parse_parameter(mut props: Vec<(String, Value)>) -> Option<Parameter> {
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

    Some(Parameter {
      param_type: param_type?,
      name: name?,
      other_fields,
    })
  }

  fn parse_value(&mut self) -> Option<Value> {
    let token = self.tokens.peek()?;
    match token.token_type.as_str() {
      "String" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t.clone());
        let s = t.value;
        let s = s.strip_prefix('"')?.strip_suffix('"')?;
        Some(Value::String(s.to_string()))
      }
      "Number" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t.clone());
        let n = t.value.parse().ok()?;
        Some(Value::Number(n))
      }
      "True" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t);
        Some(Value::Bool(true))
      }
      "False" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t);
        Some(Value::Bool(false))
      }
      "Null" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t);
        Some(Value::Null)
      }
      "Identifier" | "Lit" | "Unlit" | "Float" | "Sampler2d" | "Back" | "None" | "Opaque"
      | "Object" | "Uv0" | "Color" | "Position" | "Normal" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t.clone());
        Some(Value::Identifier(t.value))
      }
      "LBracket" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t);
        let mut arr = Vec::new();
        while let Some(t) = self.tokens.peek() {
          if t.is_type(&TokenType::RBracket) {
            let t = self.tokens.next()?;
            self.last_token = Some(t);
            break;
          }
          if let Some(val) = self.parse_value() {
            arr.push(val);
          }
          if let Some(t) = self.tokens.peek()
            && t.is_type(&TokenType::Comma)
          {
            let t = self.tokens.next()?;
            self.last_token = Some(t);
          }
        }
        Some(Value::Array(arr))
      }
      "LCurly" => {
        let t = self.tokens.next()?;
        self.last_token = Some(t);
        let mut obj = Vec::new();
        while let Some(t) = self.tokens.peek() {
          if t.is_type(&TokenType::RCurly) {
            let t = self.tokens.next()?;
            self.last_token = Some(t);
            break;
          }
          if let Some(key) = self.parse_object_key() {
            self.expect(&TokenType::Colon);
            if let Some(value) = self.parse_value() {
              obj.push((key, value));
            }
          }
          if let Some(t) = self.tokens.peek()
            && t.is_type(&TokenType::Comma)
          {
            let t = self.tokens.next()?;
            self.last_token = Some(t);
          }
        }
        Some(Value::Object(obj))
      }
      _ => None,
    }
  }

  fn parse_object_key(&mut self) -> Option<String> {
    let token = self.tokens.peek()?;
    if token.is_type(&TokenType::Identifier)
      || token.is_type(&TokenType::Type)
      || token.is_type(&TokenType::Parameters)
      || token.is_type(&TokenType::Name)
      || token.is_type(&TokenType::Requires)
      || token.is_type(&TokenType::ShadingModel)
    {
      return Some(self.tokens.next()?.value);
    }
    None
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
    assert!(result.is_some());
  }
}
