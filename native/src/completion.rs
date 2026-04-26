use crate::schema::{
  KeywordType, PropertyDef, ValueType, get_enum_values, get_keywords_by_type, get_properties,
};

#[derive(Debug, Clone)]
pub struct CompletionItem {
  pub label: String,
  pub kind: CompletionItemKind,
  pub documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionItemKind {
  Property,
  EnumValue,
  Type,
}

pub struct CompletionEngine;

static MATERIAL_PROPERTIES: std::sync::OnceLock<Vec<CompletionItem>> = std::sync::OnceLock::new();
static PARAMETER_TYPES: std::sync::OnceLock<Vec<CompletionItem>> = std::sync::OnceLock::new();
static VERTEX_ATTRIBUTES: std::sync::OnceLock<Vec<CompletionItem>> = std::sync::OnceLock::new();
static PARAMETER_FIELDS: std::sync::OnceLock<Vec<CompletionItem>> = std::sync::OnceLock::new();

impl CompletionEngine {
  pub fn new() -> Self {
    Self
  }

  pub fn get_completions(&self, context: CompletionContext) -> Vec<CompletionItem> {
    match context {
      CompletionContext::MaterialBlock => Self::get_material_properties().clone(),
      CompletionContext::PropertyValue(prop) => Self::get_property_values(&prop),
      CompletionContext::ParameterType => Self::get_parameter_types().clone(),
      CompletionContext::RequiresValue => Self::get_vertex_attributes().clone(),
      CompletionContext::ParameterField => Self::get_parameter_fields().clone(),
    }
  }

  fn get_material_properties() -> &'static Vec<CompletionItem> {
    MATERIAL_PROPERTIES.get_or_init(|| {
      get_properties()
        .iter()
        .map(|p| CompletionItem {
          label: p.name.to_string(),
          kind: CompletionItemKind::Property,
          documentation: Some(format_documentation(p)),
        })
        .collect()
    })
  }

  fn get_property_values(property_name: &str) -> Vec<CompletionItem> {
    if let Some(values) = get_enum_values(property_name) {
      values
        .iter()
        .map(|v| CompletionItem {
          label: v.to_string(),
          kind: CompletionItemKind::EnumValue,
          documentation: None,
        })
        .collect()
    } else {
      Vec::new()
    }
  }

  fn get_parameter_types() -> &'static Vec<CompletionItem> {
    PARAMETER_TYPES.get_or_init(|| {
      get_keywords_by_type(KeywordType::ParameterType)
        .iter()
        .map(|kw| CompletionItem {
          label: kw.to_string(),
          kind: CompletionItemKind::Type,
          documentation: None,
        })
        .collect()
    })
  }

  fn get_vertex_attributes() -> &'static Vec<CompletionItem> {
    VERTEX_ATTRIBUTES.get_or_init(|| {
      get_keywords_by_type(KeywordType::VertexAttribute)
        .iter()
        .map(|kw| CompletionItem {
          label: kw.to_string(),
          kind: CompletionItemKind::EnumValue,
          documentation: None,
        })
        .collect()
    })
  }

  fn get_parameter_fields() -> &'static Vec<CompletionItem> {
    PARAMETER_FIELDS.get_or_init(|| {
      get_keywords_by_type(KeywordType::ParameterField)
        .iter()
        .map(|kw| CompletionItem {
          label: kw.to_string(),
          kind: CompletionItemKind::Property,
          documentation: None,
        })
        .collect()
    })
  }
}

fn format_documentation(prop: &PropertyDef) -> String {
  let mut doc = prop.docs.to_string();
  if let Some(values) = prop.valid_values {
    doc.push_str("\n\nValues: ");
    doc.push_str(&values.join(", "));
  }
  match prop.value_type {
    ValueType::String => doc.push_str("\n\nType: string"),
    ValueType::Number => doc.push_str("\n\nType: number"),
    ValueType::Bool => doc.push_str("\n\nType: boolean"),
    ValueType::Identifier => doc.push_str("\n\nType: identifier"),
    ValueType::ArrayOfIdentifiers => doc.push_str("\n\nType: array of identifiers"),
    ValueType::ArrayOfObjects => doc.push_str("\n\nType: array of objects"),
    ValueType::ArrayOfStrings => doc.push_str("\n\nType: array of strings"),
    ValueType::Object => doc.push_str("\n\nType: object"),
  }
  doc
}

impl Default for CompletionEngine {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
  MaterialBlock,
  PropertyValue(String),
  ParameterType,
  RequiresValue,
  ParameterField,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_material_properties() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::MaterialBlock);
    assert!(!completions.is_empty());
    assert!(completions.iter().any(|c| c.label == "shadingModel"));
    assert!(completions.iter().any(|c| c.label == "blending"));
  }

  #[test]
  fn test_get_property_values() {
    let engine = CompletionEngine::new();
    let completions =
      engine.get_completions(CompletionContext::PropertyValue("shadingModel".to_string()));
    assert!(completions.iter().any(|c| c.label == "lit"));
    assert!(completions.iter().any(|c| c.label == "unlit"));
  }

  #[test]
  fn test_get_parameter_types() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::ParameterType);
    assert!(completions.iter().any(|c| c.label == "float4"));
    assert!(completions.iter().any(|c| c.label == "sampler2d"));
  }

  #[test]
  fn test_get_vertex_attributes() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::RequiresValue);
    assert!(completions.iter().any(|c| c.label == "position"));
    assert!(completions.iter().any(|c| c.label == "uv0"));
  }
}
