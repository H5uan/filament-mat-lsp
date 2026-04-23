use crate::token::{Token, TokenExt, TokenType};
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
  pub name: Option<String>,
  pub shading_model: Option<String>,
  pub requires: Vec<String>,
  pub parameters: Vec<Parameter>,
  pub other_properties: Vec<(String, Value)>,
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
}

impl Parser {
  pub fn new(tokens: Vec<Token>) -> Self {
    Self {
      tokens: tokens.into_iter().peekable(),
    }
  }

  pub fn parse_material(&mut self) -> Option<Material> {
    self.skip_to(&TokenType::LCurly);
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LCurly)
    {
      self.tokens.next();
    }

    let mut material = Material {
      name: None,
      shading_model: None,
      requires: Vec::new(),
      parameters: Vec::new(),
      other_properties: Vec::new(),
    };

    while let Some(token) = self.tokens.peek() {
      if token.is_type(&TokenType::RCurly) {
        self.tokens.next();
        break;
      }

      match token.token_type.as_str() {
        "Name" => {
          self.tokens.next();
          self.expect(&TokenType::Colon);
          if let Some(Value::Identifier(s) | Value::String(s)) = self.parse_value() {
            material.name = Some(s);
          }
        }
        "ShadingModel" => {
          self.tokens.next();
          self.expect(&TokenType::Colon);
          if let Some(Value::Identifier(s)) = self.parse_value() {
            material.shading_model = Some(s);
          }
        }
        "Requires" => {
          self.tokens.next();
          self.expect(&TokenType::Colon);
          if let Some(Value::Array(arr)) = self.parse_value() {
            for item in arr {
              if let Value::Identifier(s) = item {
                material.requires.push(s);
              }
            }
          }
        }
        "Parameters" => {
          self.tokens.next();
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
              material
                .other_properties
                .push((key_token.value.clone(), value));
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
        let s = t.value;
        let s = s.strip_prefix('"')?.strip_suffix('"')?;
        Some(Value::String(s.to_string()))
      }
      "Number" => {
        let t = self.tokens.next()?;
        let n = t.value.parse().ok()?;
        Some(Value::Number(n))
      }
      "True" => {
        self.tokens.next();
        Some(Value::Bool(true))
      }
      "False" => {
        self.tokens.next();
        Some(Value::Bool(false))
      }
      "Null" => {
        self.tokens.next();
        Some(Value::Null)
      }
      "Identifier" | "Lit" | "Unlit" | "Float" | "Sampler2d" | "Back" | "None" | "Opaque"
      | "Object" | "Uv0" | "Color" | "Position" | "Normal" => {
        let t = self.tokens.next()?;
        Some(Value::Identifier(t.value))
      }
      "LBracket" => {
        self.tokens.next();
        let mut arr = Vec::new();
        while let Some(t) = self.tokens.peek() {
          if t.is_type(&TokenType::RBracket) {
            self.tokens.next();
            break;
          }
          if let Some(val) = self.parse_value() {
            arr.push(val);
          }
          if let Some(t) = self.tokens.peek()
            && t.is_type(&TokenType::Comma)
          {
            self.tokens.next();
          }
        }
        Some(Value::Array(arr))
      }
      "LCurly" => {
        self.tokens.next();
        let mut obj = Vec::new();
        while let Some(t) = self.tokens.peek() {
          if t.is_type(&TokenType::RCurly) {
            self.tokens.next();
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
            self.tokens.next();
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
