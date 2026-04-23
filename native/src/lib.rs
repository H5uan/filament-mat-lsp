#[macro_use]
extern crate napi_derive;

pub mod completion;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod token;

use completion::{CompletionContext, CompletionEngine};
use diagnostics::{DiagnosticSeverity, Validator};
use lexer::{JsonishLexer, MaterialLexer};
use parser::Parser;
use serde::Serialize;
use token::Token;

#[derive(Serialize)]
struct JsDiagnostic {
  message: String,
  severity: String,
}

#[derive(Serialize)]
struct JsCompletionItem {
  label: String,
  kind: String,
  documentation: Option<String>,
}

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
pub fn validate_material_from_jsonish(input: String) -> String {
  let mut lexer = JsonishLexer::new(&input);
  let tokens = lexer.tokenize();
  let mut parser = Parser::new(tokens);
  let mut diagnostics = vec![];

  if let Some(material) = parser.parse_material() {
    let validator = Validator::new();
    diagnostics = validator.validate_material(&material);
  }

  let js_diagnostics: Vec<JsDiagnostic> = diagnostics
    .into_iter()
    .map(|d| JsDiagnostic {
      message: d.message,
      severity: match d.severity {
        DiagnosticSeverity::Error => "error".to_string(),
        DiagnosticSeverity::Warning => "warning".to_string(),
      },
    })
    .collect();

  serde_json::to_string(&js_diagnostics).unwrap_or_else(|_| "[]".to_string())
}

#[napi]
pub fn get_completions(context: String) -> String {
  let ctx = match context.as_str() {
    "material" => CompletionContext::MaterialBlock,
    "shadingModel" => CompletionContext::ShadingModelValue,
    "blending" => CompletionContext::BlendingValue,
    "parameterType" => CompletionContext::ParameterType,
    "requires" => CompletionContext::RequiresValue,
    _ => return "[]".to_string(),
  };
  let engine = CompletionEngine::new();
  let items: Vec<JsCompletionItem> = engine
    .get_completions(ctx)
    .into_iter()
    .map(|c| JsCompletionItem {
      label: c.label,
      kind: match c.kind {
        completion::CompletionItemKind::Property => "property".to_string(),
        completion::CompletionItemKind::EnumValue => "enum".to_string(),
        completion::CompletionItemKind::Type => "type".to_string(),
      },
      documentation: c.documentation,
    })
    .collect();

  serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
}

#[napi]
pub fn hello() -> String {
  "Hello from filament-mat-core!".to_string()
}

#[cfg(test)]
mod tests {
  use crate::lexer::{JsonishLexer, MaterialLexer};
  use crate::parser::Parser;

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

  #[test]
  fn test_parser_simple_material() {
    let input = r#"{ name: TestMat, shadingModel: lit }"#;
    let mut lexer = JsonishLexer::new(input);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let result = parser.parse_material();
    assert!(result.is_some());
  }
}
