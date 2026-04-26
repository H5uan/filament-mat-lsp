//! Central schema definition for all Filament material properties, enums, and types.
//!
//! This module serves as the single source of truth for:
//! - Lexer keyword recognition
//! - Completion providers
//! - Hover documentation
//! - Diagnostics validation
//!
//! When the official Filament format changes, update this file.

use std::collections::HashMap;
use std::sync::OnceLock;

/// A material property definition.
#[derive(Debug, Clone)]
pub struct PropertyDef {
  pub name: &'static str,
  pub value_type: ValueType,
  pub docs: &'static str,
  pub valid_values: Option<&'static [&'static str]>,
}

/// The type of value a property expects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
  String,
  Number,
  Bool,
  Identifier,
  ArrayOfIdentifiers,
  ArrayOfObjects,
  ArrayOfStrings,
  Object,
}

static PROPERTIES: OnceLock<Vec<PropertyDef>> = OnceLock::new();
static KEYWORD_MAP: OnceLock<HashMap<&'static str, KeywordType>> = OnceLock::new();

/// Classification of keywords for the lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordType {
  // Top-level blocks
  TopLevelBlock,
  // Material properties
  MaterialProperty,
  // Parameter object fields
  ParameterField,
  // Shading models
  ShadingModel,
  // Blending modes
  BlendingMode,
  // Culling modes
  CullingMode,
  // Vertex domains
  VertexDomain,
  // Material domains
  MaterialDomain,
  // Interpolation modes
  InterpolationMode,
  // Refraction modes
  RefractionMode,
  // Refraction types
  RefractionType,
  // Reflection modes
  ReflectionMode,
  // Transparency modes
  TransparencyMode,
  // Stereoscopic types
  StereoscopicType,
  // Quality levels
  QualityLevel,
  // Specular AO modes
  SpecularAmbientOcclusionMode,
  // Precision values
  PrecisionValue,
  // Sampler formats
  SamplerFormat,
  // Blend functions
  BlendFunction,
  // Vertex attributes
  VertexAttribute,
  // Variant filter values
  VariantFilterValue,
  // Parameter types (uniform/sampler)
  ParameterType,
  // Boolean
  BoolLiteral,
  // Null
  NullLiteral,
}

/// Get all material property definitions.
pub fn get_properties() -> &'static [PropertyDef] {
  PROPERTIES.get_or_init(|| {
    vec![
      PropertyDef {
        name: "name",
        value_type: ValueType::String,
        docs: "Material name identifier. Used to reference the material in code.",
        valid_values: None,
      },
      PropertyDef {
        name: "apiLevel",
        value_type: ValueType::Number,
        docs: "API level: 1 (stable) or 2 (unstable).",
        valid_values: None,
      },
      PropertyDef {
        name: "featureLevel",
        value_type: ValueType::Number,
        docs: "Feature level: 1, 2, or 3.",
        valid_values: None,
      },
      PropertyDef {
        name: "shadingModel",
        value_type: ValueType::Identifier,
        docs: "Shading model defines how the material interacts with light.",
        valid_values: Some(&["lit", "unlit", "subsurface", "cloth", "specularGlossiness"]),
      },
      PropertyDef {
        name: "domain",
        value_type: ValueType::Identifier,
        docs: "Material domain: surface, postprocess, or compute.",
        valid_values: Some(&["surface", "postprocess", "compute"]),
      },
      PropertyDef {
        name: "interpolation",
        value_type: ValueType::Identifier,
        docs: "Interpolation mode for varyings: smooth or flat.",
        valid_values: Some(&["smooth", "flat"]),
      },
      PropertyDef {
        name: "quality",
        value_type: ValueType::Identifier,
        docs: "Shader quality level: default, low, normal, or high.",
        valid_values: Some(&["default", "low", "normal", "high"]),
      },
      PropertyDef {
        name: "parameters",
        value_type: ValueType::ArrayOfObjects,
        docs: "Material parameters that can be set at runtime.",
        valid_values: None,
      },
      PropertyDef {
        name: "constants",
        value_type: ValueType::ArrayOfObjects,
        docs: "Compile-time constants for the material.",
        valid_values: None,
      },
      PropertyDef {
        name: "variables",
        value_type: ValueType::ArrayOfStrings,
        docs: "Vertex-to-fragment varyings.",
        valid_values: None,
      },
      PropertyDef {
        name: "requires",
        value_type: ValueType::ArrayOfIdentifiers,
        docs: "Required vertex attributes.",
        valid_values: None,
      },
      PropertyDef {
        name: "blending",
        value_type: ValueType::Identifier,
        docs: "Blending mode for transparency.",
        valid_values: Some(&[
          "opaque",
          "transparent",
          "fade",
          "masked",
          "add",
          "multiply",
          "screen",
          "custom",
        ]),
      },
      PropertyDef {
        name: "postLightingBlending",
        value_type: ValueType::Identifier,
        docs: "Blending mode applied after lighting.",
        valid_values: Some(&["opaque", "transparent", "add", "multiply", "screen"]),
      },
      PropertyDef {
        name: "transparency",
        value_type: ValueType::Identifier,
        docs: "Transparency rendering mode.",
        valid_values: Some(&["default", "twoPassesOneSide", "twoPassesTwoSides"]),
      },
      PropertyDef {
        name: "maskThreshold",
        value_type: ValueType::Number,
        docs: "Alpha mask threshold for masked blending. Default 0.4.",
        valid_values: None,
      },
      PropertyDef {
        name: "alphaToCoverage",
        value_type: ValueType::Bool,
        docs: "Enable alpha to coverage. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "vertexDomain",
        value_type: ValueType::Identifier,
        docs: "Vertex transformation domain.",
        valid_values: Some(&["object", "world", "view", "device"]),
      },
      PropertyDef {
        name: "vertexDomainDeviceJittered",
        value_type: ValueType::Bool,
        docs: "Apply TAA jitter in device domain. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "culling",
        value_type: ValueType::Identifier,
        docs: "Face culling mode.",
        valid_values: Some(&["back", "front", "frontAndBack", "none"]),
      },
      PropertyDef {
        name: "colorWrite",
        value_type: ValueType::Bool,
        docs: "Enable color buffer writing. Default true.",
        valid_values: None,
      },
      PropertyDef {
        name: "depthWrite",
        value_type: ValueType::Bool,
        docs: "Enable depth buffer writing. Default true for opaque, false for transparent.",
        valid_values: None,
      },
      PropertyDef {
        name: "depthCulling",
        value_type: ValueType::Bool,
        docs: "Enable depth testing. Default true.",
        valid_values: None,
      },
      PropertyDef {
        name: "doubleSided",
        value_type: ValueType::Bool,
        docs: "Render both sides of the geometry. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "instanced",
        value_type: ValueType::Bool,
        docs: "Enable instanced rendering. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "refractionMode",
        value_type: ValueType::Identifier,
        docs: "Refraction technique.",
        valid_values: Some(&["none", "cubemap", "screenspace"]),
      },
      PropertyDef {
        name: "refractionType",
        value_type: ValueType::Identifier,
        docs: "Refraction volume type.",
        valid_values: Some(&["solid", "thin"]),
      },
      PropertyDef {
        name: "reflections",
        value_type: ValueType::Identifier,
        docs: "Reflection technique.",
        valid_values: Some(&["default", "screenspace"]),
      },
      PropertyDef {
        name: "shadowMultiplier",
        value_type: ValueType::Bool,
        docs: "Multiply shadow attenuation into color. Unlit only. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "transparentShadow",
        value_type: ValueType::Bool,
        docs: "Enable transparent shadow rendering. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "clearCoatIorChange",
        value_type: ValueType::Bool,
        docs: "Adjust IOR for clear coat layer. Default true.",
        valid_values: None,
      },
      PropertyDef {
        name: "multiBounceAmbientOcclusion",
        value_type: ValueType::Bool,
        docs: "Enable multi-bounce ambient occlusion. Default false on mobile, true on desktop.",
        valid_values: None,
      },
      PropertyDef {
        name: "specularAmbientOcclusion",
        value_type: ValueType::Identifier,
        docs: "Specular ambient occlusion mode.",
        valid_values: Some(&["none", "simple", "bentNormals"]),
      },
      PropertyDef {
        name: "specularAntiAliasing",
        value_type: ValueType::Bool,
        docs: "Enable specular anti-aliasing. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "specularAntiAliasingVariance",
        value_type: ValueType::Number,
        docs: "Specular anti-aliasing variance. Default 0.15.",
        valid_values: None,
      },
      PropertyDef {
        name: "specularAntiAliasingThreshold",
        value_type: ValueType::Number,
        docs: "Specular anti-aliasing threshold. Default 0.2.",
        valid_values: None,
      },
      PropertyDef {
        name: "customSurfaceShading",
        value_type: ValueType::Bool,
        docs: "Use custom surface shading. Lit only. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "flipUV",
        value_type: ValueType::Bool,
        docs: "Flip UV coordinates vertically. Default true.",
        valid_values: None,
      },
      PropertyDef {
        name: "linearFog",
        value_type: ValueType::Bool,
        docs: "Enable linear fog. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "shadowFarAttenuation",
        value_type: ValueType::Bool,
        docs: "Enable shadow far plane attenuation. Default true.",
        valid_values: None,
      },
      PropertyDef {
        name: "framebufferFetch",
        value_type: ValueType::Bool,
        docs: "Enable framebuffer fetch. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "legacyMorphing",
        value_type: ValueType::Bool,
        docs: "Use legacy morphing. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "useDefaultDepthVariant",
        value_type: ValueType::Bool,
        docs: "Use default depth prepass variant. Default false.",
        valid_values: None,
      },
      PropertyDef {
        name: "variantFilter",
        value_type: ValueType::ArrayOfIdentifiers,
        docs: "Shader variants to exclude.",
        valid_values: None,
      },
      PropertyDef {
        name: "groupSize",
        value_type: ValueType::ArrayOfIdentifiers,
        docs: "Compute dispatch size [x, y, z]. Compute domain only.",
        valid_values: None,
      },
      PropertyDef {
        name: "stereoscopicType",
        value_type: ValueType::Identifier,
        docs: "Stereoscopic rendering mode.",
        valid_values: Some(&["none", "instanced", "multiview"]),
      },
      PropertyDef {
        name: "stereoscopicEyeCount",
        value_type: ValueType::Number,
        docs: "Number of eyes for stereoscopic rendering.",
        valid_values: None,
      },
    ]
  })
}

/// Get enum values for a given property name.
pub fn get_enum_values(property_name: &str) -> Option<&'static [&'static str]> {
  get_properties()
    .iter()
    .find(|p| p.name == property_name)
    .and_then(|p| p.valid_values)
}

/// Get the keyword type map used by the lexer.
pub fn get_keyword_map() -> &'static HashMap<&'static str, KeywordType> {
  KEYWORD_MAP.get_or_init(|| {
    let mut map = HashMap::new();

    // Material properties
    for p in get_properties() {
      map.insert(p.name, KeywordType::MaterialProperty);
    }

    // Parameter fields
    for kw in [
      "type",
      "name",
      "precision",
      "format",
      "filterable",
      "multisample",
      "transformName",
      "stages",
      "default",
    ] {
      map.insert(kw, KeywordType::ParameterField);
    }

    // Shading models
    for kw in ["lit", "unlit", "subsurface", "cloth", "specularGlossiness"] {
      map.insert(kw, KeywordType::ShadingModel);
    }

    // Blending modes
    for kw in [
      "opaque",
      "transparent",
      "fade",
      "masked",
      "add",
      "multiply",
      "screen",
      "custom",
    ] {
      map.insert(kw, KeywordType::BlendingMode);
    }

    // Culling modes
    for kw in ["back", "front", "frontAndBack", "none"] {
      map.insert(kw, KeywordType::CullingMode);
    }

    // Vertex domains
    for kw in ["object", "world", "view", "device"] {
      map.insert(kw, KeywordType::VertexDomain);
    }

    // Material domains
    for kw in ["surface", "postprocess", "compute"] {
      map.insert(kw, KeywordType::MaterialDomain);
    }

    // Interpolation modes
    for kw in ["smooth", "flat"] {
      map.insert(kw, KeywordType::InterpolationMode);
    }

    // Refraction modes
    for kw in ["none", "cubemap", "screenspace"] {
      map.insert(kw, KeywordType::RefractionMode);
    }

    // Refraction types
    for kw in ["solid", "thin"] {
      map.insert(kw, KeywordType::RefractionType);
    }

    // Reflection modes
    for kw in ["default", "screenspace"] {
      map.insert(kw, KeywordType::ReflectionMode);
    }

    // Transparency modes
    for kw in ["default", "twoPassesOneSide", "twoPassesTwoSides"] {
      map.insert(kw, KeywordType::TransparencyMode);
    }

    // Stereoscopic types
    for kw in ["none", "instanced", "multiview"] {
      map.insert(kw, KeywordType::StereoscopicType);
    }

    // Quality levels
    for kw in ["default", "low", "normal", "high"] {
      map.insert(kw, KeywordType::QualityLevel);
    }

    // Specular AO modes
    for kw in ["none", "simple", "bentNormals"] {
      map.insert(kw, KeywordType::SpecularAmbientOcclusionMode);
    }

    // Precision values
    for kw in ["default", "low", "medium", "high"] {
      map.insert(kw, KeywordType::PrecisionValue);
    }

    // Sampler formats
    for kw in ["float", "int", "uint", "shadow"] {
      map.insert(kw, KeywordType::SamplerFormat);
    }

    // Blend functions
    for kw in [
      "zero",
      "one",
      "srcColor",
      "oneMinusSrcColor",
      "dstColor",
      "oneMinusDstColor",
      "srcAlpha",
      "oneMinusSrcAlpha",
      "dstAlpha",
      "oneMinusDstAlpha",
      "srcAlphaSaturate",
    ] {
      map.insert(kw, KeywordType::BlendFunction);
    }

    // Vertex attributes
    for kw in [
      "position",
      "normal",
      "uv0",
      "uv1",
      "color",
      "tangents",
      "custom0",
      "custom1",
      "custom2",
      "custom3",
      "custom4",
      "custom5",
      "custom6",
      "custom7",
      "boneIndices",
      "boneWeights",
    ] {
      map.insert(kw, KeywordType::VertexAttribute);
    }

    // Variant filter values
    for kw in [
      "directionalLighting",
      "dynamicLighting",
      "shadowReceiver",
      "skinning",
      "fog",
      "vsm",
      "ssr",
      "stereo",
    ] {
      map.insert(kw, KeywordType::VariantFilterValue);
    }

    // Parameter types
    for kw in [
      "bool",
      "bool2",
      "bool3",
      "bool4",
      "int",
      "int2",
      "int3",
      "int4",
      "uint",
      "uint2",
      "uint3",
      "uint4",
      "float",
      "float2",
      "float3",
      "float4",
      "mat3",
      "mat4",
      "float3x3",
      "float4x4",
      "sampler2d",
      "sampler2dArray",
      "sampler3d",
      "samplerCubemap",
      "samplerExternal",
      "samplerCubemapArray",
      "subpassInput",
    ] {
      map.insert(kw, KeywordType::ParameterType);
    }

    // Boolean literals
    for kw in ["true", "false"] {
      map.insert(kw, KeywordType::BoolLiteral);
    }

    // Null literal
    map.insert("null", KeywordType::NullLiteral);

    // Top-level blocks — inserted LAST so they take priority over property values
    for kw in ["material", "vertex", "fragment", "compute", "tool"] {
      map.insert(kw, KeywordType::TopLevelBlock);
    }

    map
  })
}

/// Look up the keyword type for a given identifier string.
pub fn lookup_keyword(ident: &str) -> Option<KeywordType> {
  get_keyword_map().get(ident).copied()
}

/// Check if a string is a known keyword.
pub fn is_keyword(ident: &str) -> bool {
  get_keyword_map().contains_key(ident)
}

/// Get all completion keywords of a specific type.
pub fn get_keywords_by_type(keyword_type: KeywordType) -> Vec<&'static str> {
  get_keyword_map()
    .iter()
    .filter(|(_, kt)| **kt == keyword_type)
    .map(|(k, _)| *k)
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_schema_loaded() {
    assert!(!get_properties().is_empty());
    assert!(!get_keyword_map().is_empty());
  }

  #[test]
  fn test_lookup_keywords() {
    assert_eq!(lookup_keyword("material"), Some(KeywordType::TopLevelBlock));
    assert_eq!(
      lookup_keyword("shadingModel"),
      Some(KeywordType::MaterialProperty)
    );
    assert_eq!(lookup_keyword("lit"), Some(KeywordType::ShadingModel));
    assert_eq!(lookup_keyword("opaque"), Some(KeywordType::BlendingMode));
    assert_eq!(lookup_keyword("float4"), Some(KeywordType::ParameterType));
    assert_eq!(lookup_keyword("unknown_prop"), None);
  }

  #[test]
  fn test_enum_values() {
    let blending = get_enum_values("blending").unwrap();
    assert!(blending.contains(&"opaque"));
    assert!(blending.contains(&"custom"));

    let culling = get_enum_values("culling").unwrap();
    assert!(culling.contains(&"frontAndBack"));
  }
}
