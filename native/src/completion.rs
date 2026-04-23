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

impl CompletionEngine {
  pub fn new() -> Self {
    Self
  }

  pub fn get_completions(&self, context: CompletionContext) -> Vec<CompletionItem> {
    match context {
      CompletionContext::MaterialBlock => self.get_material_properties(),
      CompletionContext::ShadingModelValue => self.get_shading_model_values(),
      CompletionContext::BlendingValue => self.get_blending_values(),
      CompletionContext::ParameterType => self.get_parameter_types(),
      CompletionContext::RequiresValue => self.get_requires_values(),
    }
  }

  fn get_material_properties(&self) -> Vec<CompletionItem> {
    vec![
      Self::property("name", "Material name identifier"),
      Self::property("shadingModel", "Shading model (lit/unlit/subsurface/etc)"),
      Self::property("requires", "Required vertex attributes"),
      Self::property("parameters", "Material parameters list"),
      Self::property("constants", "Material constants"),
      Self::property("culling", "Face culling (front/back/none)"),
      Self::property("blending", "Blending mode"),
      Self::property("vertexDomain", "Vertex domain (object/world/view/device)"),
      Self::property("doubleSided", "Whether material is two-sided"),
      Self::property("colorWrite", "Enable color write"),
      Self::property("depthWrite", "Enable depth write"),
    ]
  }

  fn get_shading_model_values(&self) -> Vec<CompletionItem> {
    vec![
      Self::enum_value("lit", "Standard PBR shading"),
      Self::enum_value("unlit", "Unlit shading, no lighting"),
      Self::enum_value("subsurface", "Subsurface scattering"),
      Self::enum_value("cloth", "Cloth shading"),
      Self::enum_value("specularGlossiness", "Specular-glossiness workflow"),
    ]
  }

  fn get_blending_values(&self) -> Vec<CompletionItem> {
    vec![
      Self::enum_value("opaque", "Opaque blending"),
      Self::enum_value("transparent", "Alpha blending"),
      Self::enum_value("fade", "Fade transparency"),
      Self::enum_value("masked", "Alpha mask (binary)"),
      Self::enum_value("add", "Additive blending"),
      Self::enum_value("custom", "Custom blending"),
    ]
  }

  fn get_parameter_types(&self) -> Vec<CompletionItem> {
    vec![
      Self::param_type("bool", "Boolean value"),
      Self::param_type("bool2", "2-component boolean vector"),
      Self::param_type("bool3", "3-component boolean vector"),
      Self::param_type("bool4", "4-component boolean vector"),
      Self::param_type("int", "Integer value"),
      Self::param_type("int2", "2-component integer vector"),
      Self::param_type("int3", "3-component integer vector"),
      Self::param_type("int4", "4-component integer vector"),
      Self::param_type("uint", "Unsigned integer"),
      Self::param_type("uint2", "2-component uint vector"),
      Self::param_type("uint3", "3-component uint vector"),
      Self::param_type("uint4", "4-component uint vector"),
      Self::param_type("float", "Floating point value"),
      Self::param_type("float2", "2-component float vector"),
      Self::param_type("float3", "3-component float vector"),
      Self::param_type("float4", "4-component float vector"),
      Self::param_type("mat3", "3x3 matrix"),
      Self::param_type("mat4", "4x4 matrix"),
      Self::param_type("sampler2d", "2D texture sampler"),
      Self::param_type("sampler3d", "3D texture sampler"),
      Self::param_type("samplerCubemap", "Cube map sampler"),
      Self::param_type("samplerExternal", "External image sampler"),
    ]
  }

  fn get_requires_values(&self) -> Vec<CompletionItem> {
    vec![
      Self::enum_value("position", "Vertex position"),
      Self::enum_value("normal", "Vertex normal"),
      Self::enum_value("uv0", "UV coordinate set 0"),
      Self::enum_value("uv1", "UV coordinate set 1"),
      Self::enum_value("color", "Vertex color"),
      Self::enum_value("tangents", "Tangent and bitangent"),
      Self::enum_value("custom0", "Custom attribute 0"),
      Self::enum_value("custom1", "Custom attribute 1"),
      Self::enum_value("custom2", "Custom attribute 2"),
      Self::enum_value("custom3", "Custom attribute 3"),
      Self::enum_value("custom4", "Custom attribute 4"),
      Self::enum_value("boneIndices", "Bone indices for skinning"),
      Self::enum_value("boneWeights", "Bone weights for skinning"),
    ]
  }

  fn property(label: &str, docs: &str) -> CompletionItem {
    CompletionItem {
      label: label.to_string(),
      kind: CompletionItemKind::Property,
      documentation: Some(docs.to_string()),
    }
  }

  fn enum_value(label: &str, docs: &str) -> CompletionItem {
    CompletionItem {
      label: label.to_string(),
      kind: CompletionItemKind::EnumValue,
      documentation: Some(docs.to_string()),
    }
  }

  fn param_type(label: &str, docs: &str) -> CompletionItem {
    CompletionItem {
      label: label.to_string(),
      kind: CompletionItemKind::Type,
      documentation: Some(docs.to_string()),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
  MaterialBlock,
  ShadingModelValue,
  BlendingValue,
  ParameterType,
  RequiresValue,
}

impl Default for CompletionEngine {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_material_properties() {
    let engine = CompletionEngine::new();
    let completions = engine.get_completions(CompletionContext::MaterialBlock);
    assert!(!completions.is_empty());
  }
}
