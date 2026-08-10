use crate::schema::{
  KeywordType, PropertyDef, ValueType, get_enum_values, get_keywords_by_type, get_properties,
};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct CompletionItem {
  pub label: String,
  pub kind: CompletionItemKind,
  pub documentation: Option<String>,
  /// Text to insert when accepting this completion. If None, uses `label`.
  pub insert_text: Option<String>,
  /// Format of the insert text.
  pub insert_text_format: InsertTextFormat,
  /// Text used for filtering against the user's input. If None, uses `label`.
  pub filter_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionItemKind {
  Property,
  EnumValue,
  Type,
  Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertTextFormat {
  PlainText,
  Snippet,
}

/// Definition of a Filament material struct field (e.g. `material.baseColor`).
#[derive(Debug, Clone)]
pub struct MaterialFieldDef {
  pub name: &'static str,
  pub field_type: &'static str,
  pub docs: &'static str,
  /// Shading models this field is valid for. `None` = all models.
  pub valid_shading_models: Option<&'static [&'static str]>,
  /// Whether this field is available in vertex and/or fragment shader.
  pub vertex_only: bool,
}

pub struct CompletionEngine;

static MATERIAL_PROPERTIES: OnceLock<Vec<CompletionItem>> = OnceLock::new();
static PARAMETER_TYPES: OnceLock<Vec<CompletionItem>> = OnceLock::new();
static VERTEX_ATTRIBUTES: OnceLock<Vec<CompletionItem>> = OnceLock::new();
static PARAMETER_FIELDS: OnceLock<Vec<CompletionItem>> = OnceLock::new();
static MATERIAL_FIELDS: OnceLock<Vec<MaterialFieldDef>> = OnceLock::new();

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
      CompletionContext::MaterialField {
        shading_model,
        vertex_scope,
      } => Self::get_material_fields(shading_model.as_deref(), vertex_scope),
    }
  }

  /// Get all known Filament material struct fields.
  pub fn get_material_field_defs() -> &'static Vec<MaterialFieldDef> {
    MATERIAL_FIELDS.get_or_init(|| {
      vec![
        // Fields available in all shading models
        MaterialFieldDef {
          name: "emissive",
          field_type: "float4",
          docs: "Emissive color in linear space. Added to the final color after lighting.",
          valid_shading_models: None,
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "normal",
          field_type: "float3",
          docs: "Normal in tangent space. Applied after geometry normal.",
          valid_shading_models: None,
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "postLightingColor",
          field_type: "float4",
          docs: "Additional color applied after lighting. Not affected by light.",
          valid_shading_models: None,
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "ambientOcclusion",
          field_type: "float",
          docs: "Ambient occlusion factor. 1.0 = fully occluded.",
          valid_shading_models: None,
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "bentNormal",
          field_type: "float3",
          docs: "Bent normal for specular AO.",
          valid_shading_models: None,
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "perceptualRoughness",
          field_type: "float",
          docs: "Perceptual roughness (computed from roughness). Read-only.",
          valid_shading_models: None,
          vertex_only: false,
        },
        // Lit / subsurface / cloth / specularGlossiness
        MaterialFieldDef {
          name: "baseColor",
          field_type: "float4",
          docs: "Base color (albedo) in linear sRGB.",
          valid_shading_models: Some(&["lit", "subsurface", "cloth", "specularGlossiness"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "roughness",
          field_type: "float",
          docs: "Perceptual roughness in [0, 1]. 0 = smooth, 1 = rough.",
          valid_shading_models: Some(&["lit", "subsurface", "cloth", "specularGlossiness"]),
          vertex_only: false,
        },
        // Lit / subsurface / specularGlossiness
        MaterialFieldDef {
          name: "metallic",
          field_type: "float",
          docs: "Metallic factor in [0, 1]. 0 = dielectric, 1 = metal.",
          valid_shading_models: Some(&["lit", "subsurface", "specularGlossiness"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "reflectance",
          field_type: "float",
          docs: "Surface reflectance in [0, 1]. Default 0.5.",
          valid_shading_models: Some(&["lit", "subsurface", "specularGlossiness"]),
          vertex_only: false,
        },
        // Lit only
        MaterialFieldDef {
          name: "clearCoat",
          field_type: "float",
          docs: "Clear coat layer strength in [0, 1].",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "clearCoatRoughness",
          field_type: "float",
          docs: "Clear coat roughness in [0, 1].",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "clearCoatNormal",
          field_type: "float3",
          docs: "Clear coat normal in tangent space.",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "anisotropy",
          field_type: "float",
          docs: "Anisotropy factor in [-1, 1].",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "anisotropyDirection",
          field_type: "float3",
          docs: "Anisotropy direction in tangent space.",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "transmission",
          field_type: "float",
          docs: "Transmission factor in [0, 1]. Requires refractionMode.",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "ior",
          field_type: "float",
          docs: "Index of refraction. Default 1.5.",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "absorption",
          field_type: "float",
          docs: "Absorption coefficient for volumetric transmission.",
          valid_shading_models: Some(&["lit"]),
          vertex_only: false,
        },
        // Lit / cloth
        MaterialFieldDef {
          name: "sheenColor",
          field_type: "float3",
          docs: "Sheen color in linear sRGB.",
          valid_shading_models: Some(&["lit", "cloth"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "sheenRoughness",
          field_type: "float",
          docs: "Sheen roughness in [0, 1].",
          valid_shading_models: Some(&["lit", "cloth"]),
          vertex_only: false,
        },
        // Subsurface only
        MaterialFieldDef {
          name: "subsurfaceColor",
          field_type: "float3",
          docs: "Subsurface scattering color.",
          valid_shading_models: Some(&["subsurface"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "subsurfacePower",
          field_type: "float",
          docs: "Subsurface scattering power. Default 12.0.",
          valid_shading_models: Some(&["subsurface"]),
          vertex_only: false,
        },
        // SpecularGlossiness only
        MaterialFieldDef {
          name: "specularColor",
          field_type: "float3",
          docs: "Specular color in linear sRGB.",
          valid_shading_models: Some(&["specularGlossiness"]),
          vertex_only: false,
        },
        MaterialFieldDef {
          name: "glossiness",
          field_type: "float",
          docs: "Glossiness in [0, 1]. 1 = glossy, 0 = rough. Inverse of roughness.",
          valid_shading_models: Some(&["specularGlossiness"]),
          vertex_only: false,
        },
        // Vertex-only fields
        MaterialFieldDef {
          name: "clipSpaceTransform",
          field_type: "mat4",
          docs: "Clip space transform for vertex shader.",
          valid_shading_models: None,
          vertex_only: true,
        },
        MaterialFieldDef {
          name: "vertexNormal",
          field_type: "float3",
          docs: "Vertex normal in world space. Read-only.",
          valid_shading_models: None,
          vertex_only: true,
        },
        MaterialFieldDef {
          name: "worldPosition",
          field_type: "float3",
          docs: "World space position. Read-only.",
          valid_shading_models: None,
          vertex_only: true,
        },
        MaterialFieldDef {
          name: "eyeDirection",
          field_type: "float3",
          docs: "Eye direction in world space. Read-only.",
          valid_shading_models: None,
          vertex_only: true,
        },
        MaterialFieldDef {
          name: "viewSpacePosition",
          field_type: "float3",
          docs: "View space position. Read-only.",
          valid_shading_models: None,
          vertex_only: true,
        },
      ]
    })
  }

  fn get_material_fields(shading_model: Option<&str>, vertex_scope: bool) -> Vec<CompletionItem> {
    Self::get_material_field_defs()
      .iter()
      .filter(|def| {
        // Filter by shading model
        if let Some(models) = def.valid_shading_models
          && let Some(sm) = shading_model
          && !models.contains(&sm)
        {
          return false;
        }
        // Filter by scope
        if !vertex_scope && def.vertex_only {
          return false;
        }
        true
      })
      .map(|def| CompletionItem {
        label: def.name.to_string(),
        kind: CompletionItemKind::Field,
        documentation: Some(format_documentation_for_field(def)),
        insert_text: Some(def.name.to_string()),
        insert_text_format: InsertTextFormat::PlainText,
        filter_text: Some(def.name.to_string()),
      })
      .collect()
  }

  fn get_material_properties() -> &'static Vec<CompletionItem> {
    MATERIAL_PROPERTIES.get_or_init(|| {
      get_properties()
        .iter()
        .map(|p| CompletionItem {
          label: p.name.to_string(),
          kind: CompletionItemKind::Property,
          documentation: Some(format_documentation(p)),
          insert_text: Some(make_property_snippet(p)),
          insert_text_format: InsertTextFormat::Snippet,
          filter_text: Some(p.name.to_string()),
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
          insert_text: None,
          insert_text_format: InsertTextFormat::PlainText,
          filter_text: None,
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
          insert_text: None,
          insert_text_format: InsertTextFormat::PlainText,
          filter_text: None,
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
          insert_text: None,
          insert_text_format: InsertTextFormat::PlainText,
          filter_text: None,
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
          insert_text: None,
          insert_text_format: InsertTextFormat::PlainText,
          filter_text: None,
        })
        .collect()
    })
  }
}

fn make_property_snippet(prop: &PropertyDef) -> String {
  match prop.value_type {
    ValueType::Identifier if prop.valid_values.is_some() => {
      let choices = prop.valid_values.unwrap().join(",");
      format!("{}: ${{1|{}|}},", prop.name, choices)
    }
    ValueType::String => format!("{}: \"${{1}}\",", prop.name),
    ValueType::Number => format!("{}: ${{1}},", prop.name),
    ValueType::Bool => format!("{}: ${{1|true,false|}},", prop.name),
    ValueType::ArrayOfIdentifiers => format!("{}: [${{1}}],", prop.name),
    ValueType::ArrayOfObjects => format!("{}: [${{1}}],", prop.name),
    ValueType::ArrayOfStrings => format!("{}: [\"${{1}}\"],", prop.name),
    ValueType::Object => format!("{}: {{${{1}}}},", prop.name),
    _ => format!("{}: ${{1}},", prop.name),
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

fn format_documentation_for_field(def: &MaterialFieldDef) -> String {
  let mut doc = def.docs.to_string();
  doc.push_str(&format!("\n\nType: {}", def.field_type));
  if def.vertex_only {
    doc.push_str("\nVertex shader only");
  }
  if let Some(models) = def.valid_shading_models {
    doc.push_str(&format!("\nShading models: {}", models.join(", ")));
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
  MaterialField {
    shading_model: Option<String>,
    vertex_scope: bool,
  },
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

  #[test]
  fn test_get_material_fields_all() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::MaterialField {
      shading_model: None,
      vertex_scope: false,
    });
    // Should include common fields
    assert!(completions.iter().any(|c| c.label == "baseColor"));
    assert!(completions.iter().any(|c| c.label == "emissive"));
    assert!(completions.iter().any(|c| c.label == "normal"));
    // Should NOT include vertex-only fields
    assert!(!completions.iter().any(|c| c.label == "clipSpaceTransform"));
    // Should include lit-specific fields
    assert!(completions.iter().any(|c| c.label == "clearCoat"));
  }

  #[test]
  fn test_get_material_fields_filtered_by_shading_model() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::MaterialField {
      shading_model: Some("unlit".to_string()),
      vertex_scope: false,
    });
    // Unlit should NOT have baseColor, roughness, etc.
    assert!(!completions.iter().any(|c| c.label == "baseColor"));
    assert!(!completions.iter().any(|c| c.label == "clearCoat"));
    // Unlit should have emissive, normal, postLightingColor
    assert!(completions.iter().any(|c| c.label == "emissive"));
    assert!(completions.iter().any(|c| c.label == "normal"));
  }

  #[test]
  fn test_get_material_fields_vertex_scope() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::MaterialField {
      shading_model: None,
      vertex_scope: true,
    });
    // Should include vertex-only fields
    assert!(completions.iter().any(|c| c.label == "clipSpaceTransform"));
    assert!(completions.iter().any(|c| c.label == "vertexNormal"));
    // Should still include common fields
    assert!(completions.iter().any(|c| c.label == "emissive"));
  }
}
