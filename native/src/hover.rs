use crate::schema::{KeywordType, get_keywords_by_type, get_properties};
use std::collections::HashMap;
use std::sync::OnceLock;

static DOCS: OnceLock<HashMap<String, String>> = OnceLock::new();

pub struct HoverEngine;

impl HoverEngine {
  pub fn new() -> Self {
    Self
  }

  fn docs() -> &'static HashMap<String, String> {
    DOCS.get_or_init(|| {
      let mut docs = HashMap::new();

      // Material properties
      for prop in get_properties() {
        let mut doc = prop.docs.to_string();
        if let Some(values) = prop.valid_values {
          doc.push_str("\n\n**Values:** ");
          doc.push_str(&values.join(", "));
        }
        docs.insert(prop.name.to_string(), doc);
      }

      // Enum values from keyword map
      Self::add_enum_docs(&mut docs, KeywordType::ShadingModel, "Shading model");
      Self::add_enum_docs(&mut docs, KeywordType::BlendingMode, "Blending mode");
      Self::add_enum_docs(&mut docs, KeywordType::CullingMode, "Culling mode");
      Self::add_enum_docs(&mut docs, KeywordType::VertexDomain, "Vertex domain");
      Self::add_enum_docs(&mut docs, KeywordType::MaterialDomain, "Material domain");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::InterpolationMode,
        "Interpolation mode",
      );
      Self::add_enum_docs(&mut docs, KeywordType::RefractionMode, "Refraction mode");
      Self::add_enum_docs(&mut docs, KeywordType::RefractionType, "Refraction type");
      Self::add_enum_docs(&mut docs, KeywordType::ReflectionMode, "Reflection mode");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::TransparencyMode,
        "Transparency mode",
      );
      Self::add_enum_docs(
        &mut docs,
        KeywordType::StereoscopicType,
        "Stereoscopic type",
      );
      Self::add_enum_docs(&mut docs, KeywordType::QualityLevel, "Quality level");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::SpecularAmbientOcclusionMode,
        "Specular ambient occlusion mode",
      );
      Self::add_enum_docs(&mut docs, KeywordType::PrecisionValue, "Precision value");
      Self::add_enum_docs(&mut docs, KeywordType::SamplerFormat, "Sampler format");
      Self::add_enum_docs(&mut docs, KeywordType::BlendFunction, "Blend function");
      Self::add_enum_docs(&mut docs, KeywordType::VertexAttribute, "Vertex attribute");
      Self::add_enum_docs(
        &mut docs,
        KeywordType::VariantFilterValue,
        "Variant filter value",
      );
      Self::add_enum_docs(&mut docs, KeywordType::ParameterType, "Parameter type");

      // Filament Shader API
      docs.insert(
        "MaterialInputs".to_string(),
        "Filament shader input struct. Contains fields like baseColor, roughness, metallic, normal, etc.".to_string(),
      );
      docs.insert(
        "MaterialVertexInputs".to_string(),
        "Filament vertex shader input struct. Contains vertex attributes passed to the fragment shader.".to_string(),
      );
      docs.insert(
        "materialParams".to_string(),
        "Runtime material parameter block. Access parameters defined in the material block using materialParams.name.".to_string(),
      );
      docs.insert(
        "prepareMaterial".to_string(),
        "Filament built-in function. Must be called before modifying MaterialInputs in the fragment shader.".to_string(),
      );
      docs.insert(
        "getUV0".to_string(),
        "Returns the first UV coordinate set (uv0) for the current fragment.".to_string(),
      );
      docs.insert(
        "getUV1".to_string(),
        "Returns the second UV coordinate set (uv1) for the current fragment.".to_string(),
      );
      docs.insert(
        "getWorldPosition".to_string(),
        "Returns the world-space position of the current fragment.".to_string(),
      );
      docs.insert(
        "getWorldNormal".to_string(),
        "Returns the world-space normal of the current fragment.".to_string(),
      );
      docs.insert(
        "texture".to_string(),
        "GLSL built-in function for sampling textures.".to_string(),
      );

      docs
    })
  }

  fn add_enum_docs(docs: &mut HashMap<String, String>, keyword_type: KeywordType, category: &str) {
    for kw in get_keywords_by_type(keyword_type) {
      if !docs.contains_key(kw) {
        docs.insert(kw.to_string(), format!("{}: {}", category, kw));
      }
    }
  }

  pub fn get_hover(&self, word: &str) -> Option<&'static String> {
    Self::docs().get(word)
  }
}

impl Default for HoverEngine {
  fn default() -> Self {
    Self::new()
  }
}
