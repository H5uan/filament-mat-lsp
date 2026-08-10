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
  /// Convenience field — populated from `properties["name"]` after parsing.
  pub name: Option<Located<String>>,
  /// Convenience field — populated from `properties["shadingModel"]` after parsing.
  pub shading_model: Option<Located<String>>,
  /// All parsed properties (including name, shadingModel, requires, etc.).
  pub properties: Vec<(String, Located<Value>)>,
  /// Structured parameter definitions.
  pub parameters: Vec<Parameter>,
  /// Compile-time constants.
  pub constants: Vec<Constant>,
  /// Vertex-to-fragment varyings.
  pub variables: Vec<Variable>,
  /// SSBO bindings.
  pub buffers: Vec<Buffer>,
  /// Subpass inputs.
  pub subpasses: Vec<Subpass>,
  /// Fragment outputs.
  pub outputs: Vec<Output>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
  pub param_type: String,
  pub name: String,
  pub other_fields: Vec<(String, Value)>,
  pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Constant {
  pub name: String,
  pub const_type: String,
  pub default_value: Option<Value>,
  pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Variable {
  pub name: String,
  pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Buffer {
  pub name: String,
  pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subpass {
  pub name: String,
  pub subpass_type: String,
  pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
  pub name: String,
  pub output_type: String,
  pub range: TextRange,
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

/// Type of shader block in a .mat file.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderBlockType {
  Vertex,
  Fragment,
  Compute,
  Tool,
}

/// A shader block (vertex/fragment/compute/tool) containing raw GLSL code.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderBlock {
  pub block_type: ShaderBlockType,
  pub code: String,
  pub range: TextRange,
  /// Extracted GLSL symbols (functions, uniforms, varyings, references).
  pub symbols: Vec<crate::shader_symbols::ShaderSymbol>,
}

/// Top-level AST for a complete .mat file.
#[derive(Debug, Clone, PartialEq)]
pub struct MatFile {
  pub material: Material,
  pub shaders: Vec<ShaderBlock>,
  pub errors: Vec<ParseError>,
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
      properties: Vec::new(),
      parameters: Vec::new(),
      constants: Vec::new(),
      variables: Vec::new(),
      buffers: Vec::new(),
      subpasses: Vec::new(),
      outputs: Vec::new(),
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

      // All property keys are now TokenType::Identifier (or BoolLiteral/NullLiteral in edge cases)
      let key_token = match token.token_type {
        TokenType::Identifier | TokenType::BoolLiteral | TokenType::NullLiteral => self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("property key"))?,
        _ => {
          // Unknown token — skip and continue for error recovery
          self.tokens.next();
          continue;
        }
      };

      let key = key_token.value.clone();
      self.expect(&TokenType::Colon);

      // Dispatch to structured parsers for complex properties, otherwise generic value
      let structured_handled = match key.as_str() {
        "parameters" => {
          self.parse_parameters_array(&mut material.parameters);
          true
        }
        "constants" => {
          self.parse_constants_array(&mut material.constants);
          true
        }
        "variables" => {
          self.parse_variables_list(&mut material.variables);
          true
        }
        "buffers" => {
          self.parse_buffers_array(&mut material.buffers);
          true
        }
        "subpasses" => {
          self.parse_subpasses_array(&mut material.subpasses);
          true
        }
        "outputs" => {
          self.parse_outputs_array(&mut material.outputs);
          true
        }
        _ => false,
      };

      if !structured_handled {
        // Generic property: parse value, store in unified properties
        if let Ok(value) = self.parse_value() {
          let end_token = self.last_token.as_ref().unwrap_or(&key_token);
          let range = Self::make_range(&key_token, end_token);
          material.properties.push((key, Located::new(value, range)));
        }
      }
    }

    // Extract convenience fields from properties
    for (k, v) in &material.properties {
      match k.as_str() {
        "name" => {
          if let Value::Identifier(s) | Value::String(s) = &v.value {
            material.name = Some(Located::new(s.clone(), v.range.clone()));
          }
        }
        "shadingModel" => {
          if let Value::Identifier(s) = &v.value {
            material.shading_model = Some(Located::new(s.clone(), v.range.clone()));
          }
        }
        _ => {}
      }
    }

    Ok(material)
  }

  // ---------------------------------------------------------------------------
  // Structured property parsers
  // ---------------------------------------------------------------------------

  fn parse_parameters_array(&mut self, params: &mut Vec<Parameter>) {
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LBracket)
    {
      self.tokens.next(); // consume '['
      while let Some(token) = self.tokens.peek() {
        if token.is_type(&TokenType::RBracket) {
          self.tokens.next(); // consume ']'
          break;
        }
        if token.is_type(&TokenType::LCurly) {
          let start_token = self.tokens.next().unwrap(); // consume '{'
          let mut props = Vec::new();
          let mut end_token = start_token.clone();
          while let Some(t) = self.tokens.peek() {
            if t.is_type(&TokenType::RCurly) {
              end_token = self.tokens.next().unwrap();
              break;
            }
            if let Ok(key) = self.parse_object_key() {
              self.expect(&TokenType::Colon);
              if let Ok(value) = self.parse_value() {
                props.push((key, value));
              }
            } else {
              if let Some(t) = self.tokens.next() {
                self.last_token = Some(t);
              }
            }
            if let Some(t) = self.tokens.peek()
              && t.is_type(&TokenType::Comma)
            {
              self.tokens.next();
            }
          }
          let range = Self::make_range(&start_token, &end_token);
          if let Ok(param) = self.parse_parameter_from_props(props, range) {
            params.push(param);
          }
        }
        if let Some(t) = self.tokens.peek()
          && t.is_type(&TokenType::Comma)
        {
          self.tokens.next();
        }
      }
    }
  }

  fn parse_constants_array(&mut self, constants: &mut Vec<Constant>) {
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LBracket)
    {
      self.tokens.next(); // consume '['
      while let Some(token) = self.tokens.peek() {
        if token.is_type(&TokenType::RBracket) {
          self.tokens.next(); // consume ']'
          break;
        }
        if token.is_type(&TokenType::LCurly) {
          let start_token = self.tokens.next().unwrap(); // consume '{'
          let mut props = Vec::new();
          let mut end_token = start_token.clone();
          while let Some(t) = self.tokens.peek() {
            if t.is_type(&TokenType::RCurly) {
              end_token = self.tokens.next().unwrap();
              break;
            }
            if let Ok(key) = self.parse_object_key() {
              self.expect(&TokenType::Colon);
              if let Ok(value) = self.parse_value() {
                props.push((key, value));
              }
            } else {
              if let Some(t) = self.tokens.next() {
                self.last_token = Some(t);
              }
            }
            if let Some(t) = self.tokens.peek()
              && t.is_type(&TokenType::Comma)
            {
              self.tokens.next();
            }
          }
          let range = Self::make_range(&start_token, &end_token);
          if let Ok(constant) = self.parse_constant_from_props(props, range) {
            constants.push(constant);
          }
        }
        if let Some(t) = self.tokens.peek()
          && t.is_type(&TokenType::Comma)
        {
          self.tokens.next();
        }
      }
    }
  }

  fn parse_variables_list(&mut self, variables: &mut Vec<Variable>) {
    if let Ok(value) = self.parse_value()
      && let Value::Array(arr) = value
    {
      for item in arr {
        if let Value::Identifier(s) = item {
          let range = self.last_token.as_ref().map_or(
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
            |t| TextRange {
              start: TextPosition {
                line: t.line,
                character: t.column,
              },
              end: TextPosition {
                line: t.line,
                character: t.column + t.value.len() as u32,
              },
            },
          );
          variables.push(Variable { name: s, range });
        }
      }
    }
  }

  fn parse_buffers_array(&mut self, buffers: &mut Vec<Buffer>) {
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LBracket)
    {
      self.tokens.next(); // consume '['
      while let Some(token) = self.tokens.peek() {
        if token.is_type(&TokenType::RBracket) {
          self.tokens.next(); // consume ']'
          break;
        }
        if token.is_type(&TokenType::LCurly) {
          let start_token = self.tokens.next().unwrap();
          let mut props = Vec::new();
          let mut end_token = start_token.clone();
          while let Some(t) = self.tokens.peek() {
            if t.is_type(&TokenType::RCurly) {
              end_token = self.tokens.next().unwrap();
              break;
            }
            if let Ok(key) = self.parse_object_key() {
              self.expect(&TokenType::Colon);
              if let Ok(value) = self.parse_value() {
                props.push((key, value));
              }
            } else {
              if let Some(t) = self.tokens.next() {
                self.last_token = Some(t);
              }
            }
            if let Some(t) = self.tokens.peek()
              && t.is_type(&TokenType::Comma)
            {
              self.tokens.next();
            }
          }
          let range = Self::make_range(&start_token, &end_token);
          if let Ok(buffer) = self.parse_buffer_from_props(props, range) {
            buffers.push(buffer);
          }
        }
        if let Some(t) = self.tokens.peek()
          && t.is_type(&TokenType::Comma)
        {
          self.tokens.next();
        }
      }
    }
  }

  fn parse_subpasses_array(&mut self, subpasses: &mut Vec<Subpass>) {
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LBracket)
    {
      self.tokens.next(); // consume '['
      while let Some(token) = self.tokens.peek() {
        if token.is_type(&TokenType::RBracket) {
          self.tokens.next(); // consume ']'
          break;
        }
        if token.is_type(&TokenType::LCurly) {
          let start_token = self.tokens.next().unwrap();
          let mut props = Vec::new();
          let mut end_token = start_token.clone();
          while let Some(t) = self.tokens.peek() {
            if t.is_type(&TokenType::RCurly) {
              end_token = self.tokens.next().unwrap();
              break;
            }
            if let Ok(key) = self.parse_object_key() {
              self.expect(&TokenType::Colon);
              if let Ok(value) = self.parse_value() {
                props.push((key, value));
              }
            } else {
              if let Some(t) = self.tokens.next() {
                self.last_token = Some(t);
              }
            }
            if let Some(t) = self.tokens.peek()
              && t.is_type(&TokenType::Comma)
            {
              self.tokens.next();
            }
          }
          let range = Self::make_range(&start_token, &end_token);
          if let Ok(subpass) = self.parse_subpass_from_props(props, range) {
            subpasses.push(subpass);
          }
        }
        if let Some(t) = self.tokens.peek()
          && t.is_type(&TokenType::Comma)
        {
          self.tokens.next();
        }
      }
    }
  }

  fn parse_outputs_array(&mut self, outputs: &mut Vec<Output>) {
    if let Some(token) = self.tokens.peek()
      && token.is_type(&TokenType::LBracket)
    {
      self.tokens.next(); // consume '['
      while let Some(token) = self.tokens.peek() {
        if token.is_type(&TokenType::RBracket) {
          self.tokens.next(); // consume ']'
          break;
        }
        if token.is_type(&TokenType::LCurly) {
          let start_token = self.tokens.next().unwrap();
          let mut props = Vec::new();
          let mut end_token = start_token.clone();
          while let Some(t) = self.tokens.peek() {
            if t.is_type(&TokenType::RCurly) {
              end_token = self.tokens.next().unwrap();
              break;
            }
            if let Ok(key) = self.parse_object_key() {
              self.expect(&TokenType::Colon);
              if let Ok(value) = self.parse_value() {
                props.push((key, value));
              }
            } else {
              if let Some(t) = self.tokens.next() {
                self.last_token = Some(t);
              }
            }
            if let Some(t) = self.tokens.peek()
              && t.is_type(&TokenType::Comma)
            {
              self.tokens.next();
            }
          }
          let range = Self::make_range(&start_token, &end_token);
          if let Ok(output) = self.parse_output_from_props(props, range) {
            outputs.push(output);
          }
        }
        if let Some(t) = self.tokens.peek()
          && t.is_type(&TokenType::Comma)
        {
          self.tokens.next();
        }
      }
    }
  }

  /// Parse a complete .mat file, returning the full AST with error recovery.
  pub fn parse(&mut self) -> MatFile {
    let mut material = None;
    let mut shaders = Vec::new();
    let mut errors = Vec::new();

    while let Some(token) = self.tokens.peek() {
      match token.token_type {
        TokenType::BlockKeyword => {
          match token.value.as_str() {
            "material" => {
              if material.is_some() {
                errors.push(ParseError::at_token(
                  "Multiple material blocks are not allowed",
                  token,
                ));
                self.skip_block();
              } else {
                match self.parse_material() {
                  Ok(m) => material = Some(m),
                  Err(e) => {
                    errors.push(e);
                    self.skip_block();
                  }
                }
              }
            }
            // vertex, fragment, compute, tool
            _ => match self.parse_shader_block() {
              Ok(block) => shaders.push(block),
              Err(e) => {
                errors.push(e);
                self.skip_block();
              }
            },
          }
        }
        TokenType::Comment => {
          // Skip top-level comments
          self.tokens.next();
        }
        _ => {
          let bad = self.tokens.next().unwrap();
          errors.push(ParseError::at_token(
            format!("Unexpected token at top level: {}", bad.value),
            &bad,
          ));
          // Try to recover by skipping to next block start
          self.skip_to_next_block();
        }
      }
    }

    let material = material.unwrap_or_else(|| Material {
      range: TextRange {
        start: TextPosition {
          line: 0,
          character: 0,
        },
        end: TextPosition {
          line: 0,
          character: 0,
        },
      },
      name: None,
      shading_model: None,
      properties: Vec::new(),
      parameters: Vec::new(),
      constants: Vec::new(),
      variables: Vec::new(),
      buffers: Vec::new(),
      subpasses: Vec::new(),
      outputs: Vec::new(),
    });

    MatFile {
      material,
      shaders,
      errors,
    }
  }

  /// Parse a shader block (vertex/fragment/compute/tool).
  fn parse_shader_block(&mut self) -> Result<ShaderBlock, ParseError> {
    let start_token = self
      .tokens
      .next()
      .ok_or_else(|| ParseError::at_eof("shader block keyword"))?;

    let block_type = match start_token.value.as_str() {
      "vertex" => ShaderBlockType::Vertex,
      "fragment" => ShaderBlockType::Fragment,
      "compute" => ShaderBlockType::Compute,
      "tool" => ShaderBlockType::Tool,
      _ => {
        return Err(ParseError::at_token(
          "Expected shader block keyword (vertex/fragment/compute/tool)",
          &start_token,
        ));
      }
    };

    // Expect '{'
    if let Some(token) = self.tokens.peek()
      && !token.is_type(&TokenType::LCurly)
    {
      return Err(ParseError::at_token(
        "Expected '{' after shader block keyword",
        &start_token,
      ));
    }
    self.tokens.next(); // consume '{'

    // Collect GLSL code tokens until matching '}'
    let mut code = String::new();
    let mut brace_depth = 1usize;
    let _code_start_line = start_token.line;
    let mut last_line = start_token.line;

    while let Some(token) = self.tokens.peek() {
      match token.token_type {
        TokenType::LCurly => {
          brace_depth += 1;
          code.push('{');
          last_line = token.line;
          self.tokens.next();
        }
        TokenType::RCurly => {
          if brace_depth == 1 {
            let end_token = self
              .tokens
              .next()
              .ok_or_else(|| ParseError::at_eof("closing brace"))?;
            let range = TextRange {
              start: Self::token_pos(&start_token),
              end: TextPosition {
                line: end_token.line,
                character: end_token.column + 1,
              },
            };
            let trimmed_code = code.trim().to_string();
            let symbols = crate::shader_symbols::extract_symbols(&ShaderBlock {
              block_type: block_type.clone(),
              code: trimmed_code.clone(),
              range: range.clone(),
              symbols: vec![],
            });
            return Ok(ShaderBlock {
              block_type,
              code: trimmed_code,
              range,
              symbols,
            });
          } else {
            brace_depth -= 1;
            code.push('}');
            last_line = token.line;
            self.tokens.next();
          }
        }
        TokenType::GlslCode => {
          let t = self.tokens.next().unwrap();
          if !code.is_empty() {
            code.push('\n');
          }
          code.push_str(&t.value);
          last_line = t.line;
        }
        TokenType::Comment => {
          let t = self.tokens.next().unwrap();
          if !code.is_empty() {
            code.push('\n');
          }
          code.push_str(&t.value);
          last_line = t.line;
        }
        _ => {
          let t = self.tokens.next().unwrap();
          if !code.is_empty() && last_line != t.line {
            code.push('\n');
          }
          code.push_str(&t.value);
          last_line = t.line;
        }
      }
    }

    Err(ParseError::at_eof("closing brace for shader block"))
  }

  /// Skip tokens until the matching '}' for the current block.
  fn skip_block(&mut self) {
    let mut depth = 0usize;
    while let Some(token) = self.tokens.peek() {
      if token.is_type(&TokenType::LCurly) {
        depth += 1;
        self.tokens.next();
      } else if token.is_type(&TokenType::RCurly) {
        if depth <= 1 {
          self.tokens.next();
          break;
        }
        depth -= 1;
        self.tokens.next();
      } else {
        self.tokens.next();
      }
    }
  }

  /// Skip tokens until the next top-level block keyword or EOF.
  fn skip_to_next_block(&mut self) {
    while let Some(token) = self.tokens.peek() {
      if token.is_type(&TokenType::BlockKeyword) {
        break;
      }
      self.tokens.next();
    }
  }

  fn parse_parameter_from_props(
    &mut self,
    mut props: Vec<(String, Value)>,
    range: TextRange,
  ) -> Result<Parameter, ParseError> {
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
      range,
    })
  }

  fn parse_constant_from_props(
    &mut self,
    mut props: Vec<(String, Value)>,
    range: TextRange,
  ) -> Result<Constant, ParseError> {
    let mut name = None;
    let mut const_type = None;
    let mut default_value = None;
    for (k, v) in props.drain(..) {
      match k.to_lowercase().as_str() {
        "name" => {
          if let Value::Identifier(s) | Value::String(s) = v {
            name = Some(s);
          }
        }
        "type" => {
          if let Value::Identifier(s) = v {
            const_type = Some(s);
          }
        }
        "default" => {
          default_value = Some(v);
        }
        _ => {}
      }
    }
    let name = name.ok_or_else(|| ParseError::new("constant missing 'name'", range.clone()))?;
    let const_type =
      const_type.ok_or_else(|| ParseError::new("constant missing 'type'", range.clone()))?;
    Ok(Constant {
      name,
      const_type,
      default_value,
      range,
    })
  }

  fn parse_buffer_from_props(
    &mut self,
    mut props: Vec<(String, Value)>,
    range: TextRange,
  ) -> Result<Buffer, ParseError> {
    let mut name = None;
    for (k, v) in props.drain(..) {
      if k.to_lowercase().as_str() == "name"
        && let Value::Identifier(s) | Value::String(s) = v
      {
        name = Some(s);
      }
    }
    let name = name.ok_or_else(|| ParseError::new("buffer missing 'name'", range.clone()))?;
    Ok(Buffer { name, range })
  }

  fn parse_subpass_from_props(
    &mut self,
    mut props: Vec<(String, Value)>,
    range: TextRange,
  ) -> Result<Subpass, ParseError> {
    let mut name = None;
    let mut subpass_type = None;
    for (k, v) in props.drain(..) {
      match k.to_lowercase().as_str() {
        "name" => {
          if let Value::Identifier(s) | Value::String(s) = v {
            name = Some(s);
          }
        }
        "type" => {
          if let Value::Identifier(s) = v {
            subpass_type = Some(s);
          }
        }
        _ => {}
      }
    }
    let name = name.ok_or_else(|| ParseError::new("subpass missing 'name'", range.clone()))?;
    let subpass_type =
      subpass_type.ok_or_else(|| ParseError::new("subpass missing 'type'", range.clone()))?;
    Ok(Subpass {
      name,
      subpass_type,
      range,
    })
  }

  fn parse_output_from_props(
    &mut self,
    mut props: Vec<(String, Value)>,
    range: TextRange,
  ) -> Result<Output, ParseError> {
    let mut name = None;
    let mut output_type = None;
    for (k, v) in props.drain(..) {
      match k.to_lowercase().as_str() {
        "name" => {
          if let Value::Identifier(s) | Value::String(s) = v {
            name = Some(s);
          }
        }
        "type" => {
          if let Value::Identifier(s) = v {
            output_type = Some(s);
          }
        }
        _ => {}
      }
    }
    let name = name.ok_or_else(|| ParseError::new("output missing 'name'", range.clone()))?;
    let output_type =
      output_type.ok_or_else(|| ParseError::new("output missing 'type'", range.clone()))?;
    Ok(Output {
      name,
      output_type,
      range,
    })
  }

  fn parse_value(&mut self) -> Result<Value, ParseError> {
    let token = match self.tokens.peek() {
      Some(t) => t.clone(),
      None => return Err(ParseError::at_eof("value")),
    };
    match token.token_type {
      TokenType::StringLiteral => {
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
      TokenType::NumberLiteral => {
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
      TokenType::BoolLiteral => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("boolean value"))?;
        let val = t.value.eq_ignore_ascii_case("true");
        self.last_token = Some(t);
        Ok(Value::Bool(val))
      }
      TokenType::NullLiteral => {
        let t = self
          .tokens
          .next()
          .ok_or_else(|| ParseError::at_eof("null value"))?;
        self.last_token = Some(t);
        Ok(Value::Null)
      }
      TokenType::Identifier => {
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
          } else {
            // Error recovery: skip token to avoid infinite loop
            if let Some(t) = self.tokens.next() {
              self.last_token = Some(t);
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
          } else {
            // Error recovery: skip token to avoid infinite loop
            if let Some(t) = self.tokens.next() {
              self.last_token = Some(t);
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
    // All identifiers are now TokenType::Identifier (or BoolLiteral for keys like "true")
    if token.is_type(&TokenType::Identifier)
      || token.is_type(&TokenType::BoolLiteral)
      || token.is_type(&TokenType::NullLiteral)
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
  use crate::lexer::Lexer;

  #[test]
  fn test_parse_simple_material() {
    let input = r#"material { name: TestMat, shadingModel: lit }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let result = parser.parse_material();
    assert!(result.is_ok());
  }

  #[test]
  fn test_parse_full_mat_file() {
    let input = r#"material {
      name : TestMaterial,
      shadingModel : lit
    }
    vertex {
      void materialVertex(inout MaterialVertexInputs material) {
        // Vertex shader code
      }
    }
    fragment {
      void material(inout MaterialInputs material) {
        prepareMaterial(material);
      }
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let matfile = parser.parse();

    assert!(
      matfile.errors.is_empty(),
      "Parse errors: {:?}",
      matfile.errors
    );
    assert_eq!(
      matfile.material.name.as_ref().unwrap().value,
      "TestMaterial"
    );
    assert_eq!(
      matfile.material.shading_model.as_ref().unwrap().value,
      "lit"
    );
    assert_eq!(matfile.shaders.len(), 2);
    assert!(matches!(
      matfile.shaders[0].block_type,
      ShaderBlockType::Vertex
    ));
    assert!(matches!(
      matfile.shaders[1].block_type,
      ShaderBlockType::Fragment
    ));
  }

  #[test]
  fn test_parse_with_compute_and_tool() {
    let input = r#"material { name : ComputeMat, shadingModel : unlit }
    compute { groupSize : [8, 8, 1] }
    tool { /* tool block */ }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let matfile = parser.parse();

    assert!(
      matfile.errors.is_empty(),
      "Parse errors: {:?}",
      matfile.errors
    );
    assert_eq!(matfile.shaders.len(), 2);
    assert!(matches!(
      matfile.shaders[0].block_type,
      ShaderBlockType::Compute
    ));
    assert!(matches!(
      matfile.shaders[1].block_type,
      ShaderBlockType::Tool
    ));
  }

  #[test]
  fn test_parse_error_recovery() {
    // Missing closing brace for material, but vertex block follows
    let input = r#"material {
      name : BrokenMat,
      shadingModel : lit
    vertex {
      void main() {}
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let matfile = parser.parse();

    // Should have recovered and parsed the material (with stray vertex keyword inside)
    // or reported errors but still produced a result
    assert!(!matfile.errors.is_empty() || matfile.material.name.is_some());
  }

  #[test]
  fn test_parse_new_blocks() {
    let input = r#"material {
      name : "NewBlockMat"
      shadingModel : lit
      constants : [
        { name : PI, type : float, default : 3.14159 }
      ]
      variables : [worldPosition, worldNormal]
      buffers : [
        { name : instanceData }
      ]
      subpasses : [
        { name : subpass0, type : float4 }
      ]
      outputs : [
        { name : color, type : float4 }
      ]
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let matfile = parser.parse();

    assert!(
      matfile.errors.is_empty(),
      "Parse errors: {:?}",
      matfile.errors
    );
    assert_eq!(matfile.material.name.as_ref().unwrap().value, "NewBlockMat");
    assert_eq!(
      matfile.material.constants.len(),
      1,
      "constants count mismatch"
    );
    assert_eq!(matfile.material.constants[0].name, "PI");
    assert_eq!(matfile.material.constants[0].const_type, "float");
    assert!(
      matfile.material.constants[0].default_value.is_some(),
      "default_value should be Some, got: {:?}",
      matfile.material.constants[0].default_value
    );
    assert_eq!(
      matfile.material.variables.len(),
      2,
      "variables count mismatch"
    );
    assert_eq!(matfile.material.variables[0].name, "worldPosition");
    assert_eq!(matfile.material.variables[1].name, "worldNormal");
    assert_eq!(matfile.material.buffers.len(), 1, "buffers count mismatch");
    assert_eq!(matfile.material.buffers[0].name, "instanceData");
    assert_eq!(
      matfile.material.subpasses.len(),
      1,
      "subpasses count mismatch"
    );
    assert_eq!(matfile.material.subpasses[0].name, "subpass0");
    assert_eq!(matfile.material.subpasses[0].subpass_type, "float4");
    assert_eq!(matfile.material.outputs.len(), 1, "outputs count mismatch");
    assert_eq!(matfile.material.outputs[0].name, "color");
    assert_eq!(matfile.material.outputs[0].output_type, "float4");
  }

  #[test]
  fn test_parse_new_blocks_missing_required() {
    let input = r#"material {
      name : "BadMat"
      shadingModel : lit
      constants : [
        { type : float }
      ]
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let matfile = parser.parse();

    // The constant without name should be skipped, resulting in 0 constants
    assert_eq!(matfile.material.constants.len(), 0);
    // But material should still parse
    assert_eq!(matfile.material.name.as_ref().unwrap().value, "BadMat");
  }

  #[test]
  fn test_parse_all_test_files() {
    use std::fs;
    let test_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../test");
    let entries = fs::read_dir(test_dir).expect("Failed to read test directory");

    let mut parsed_count = 0;
    for entry in entries {
      let entry = entry.expect("Failed to read directory entry");
      let path = entry.path();
      if path.extension().and_then(|s| s.to_str()) != Some("mat") {
        continue;
      }

      let content = fs::read_to_string(&path).expect(&format!("Failed to read {:?}", path));
      let mut lexer = Lexer::new(&content);
      let tokens = lexer.tokenize();
      let mut parser = Parser::new(tokens);
      let matfile = parser.parse();

      assert!(
        matfile.errors.is_empty(),
        "Parse errors in {}: {:?}",
        path.file_name().unwrap().to_string_lossy(),
        matfile.errors
      );
      assert!(
        matfile.material.name.is_some(),
        "Material name missing in {}",
        path.file_name().unwrap().to_string_lossy()
      );
      parsed_count += 1;
    }

    assert!(parsed_count >= 1, "No .mat files found in test directory");
  }
}
