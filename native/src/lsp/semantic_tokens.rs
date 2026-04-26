use filament_mat_lsp::lexer::Lexer;
use filament_mat_lsp::token::TokenType;
use lsp_types::SemanticToken;

/// Generate semantic token data for a .mat file.
pub fn generate_semantic_tokens(text: &str) -> Vec<SemanticToken> {
  let mut lexer = Lexer::new(text);
  let tokens = lexer.tokenize();

  let mut data = Vec::new();
  let mut last_line = 0u32;
  let mut last_char = 0u32;

  for token in tokens {
    let token_type_idx = match token.token_type {
      // Comments
      TokenType::Comment => Some(0u32), // comment

      // Properties (material block keywords)
      TokenType::Name
      | TokenType::ApiLevel
      | TokenType::FeatureLevel
      | TokenType::ShadingModel
      | TokenType::Domain
      | TokenType::Interpolation
      | TokenType::Quality
      | TokenType::Requires
      | TokenType::Parameters
      | TokenType::Constants
      | TokenType::Variables
      | TokenType::Buffers
      | TokenType::Subpasses
      | TokenType::Outputs
      | TokenType::Culling
      | TokenType::Blending
      | TokenType::BlendFunction
      | TokenType::PostLightingBlending
      | TokenType::Transparency
      | TokenType::MaskThreshold
      | TokenType::AlphaToCoverage
      | TokenType::VertexDomain
      | TokenType::VertexDomainDeviceJittered
      | TokenType::MaterialDomain
      | TokenType::DoubleSided
      | TokenType::ColorWrite
      | TokenType::DepthWrite
      | TokenType::DepthCulling
      | TokenType::DepthTest
      | TokenType::RefractionMode
      | TokenType::RefractionType
      | TokenType::Reflections
      | TokenType::ReflectionMode
      | TokenType::ShadowMultiplier
      | TokenType::TransparentShadow
      | TokenType::ClearCoatIorChange
      | TokenType::MultiBounceAmbientOcclusion
      | TokenType::SpecularAmbientOcclusion
      | TokenType::SpecularAntiAliasing
      | TokenType::SpecularAntiAliasingVariance
      | TokenType::SpecularAntiAliasingThreshold
      | TokenType::CustomSurfaceShading
      | TokenType::FlipUv
      | TokenType::LinearFog
      | TokenType::ShadowFarAttenuation
      | TokenType::FramebufferFetch
      | TokenType::LegacyMorphing
      | TokenType::UseDefaultDepthVariant
      | TokenType::VariantFilter
      | TokenType::GroupSize
      | TokenType::StereoscopicType
      | TokenType::StereoscopicEyeCount => Some(1), // property

      // Parameter fields
      TokenType::Type
      | TokenType::Precision
      | TokenType::Format
      | TokenType::Filterable
      | TokenType::Multisample
      | TokenType::TransformName
      | TokenType::Stages
      | TokenType::Qualifiers
      | TokenType::Fields
      | TokenType::Target
      | TokenType::Location => Some(2), // parameter field

      // Enum values
      TokenType::Lit
      | TokenType::Unlit
      | TokenType::Subsurface
      | TokenType::Cloth
      | TokenType::SpecularGlossiness
      | TokenType::Front
      | TokenType::Back
      | TokenType::FrontAndBack
      | TokenType::None
      | TokenType::Opaque
      | TokenType::Transparent
      | TokenType::Fade
      | TokenType::Masked
      | TokenType::Add
      | TokenType::Multiply
      | TokenType::Screen
      | TokenType::Custom
      | TokenType::Object
      | TokenType::World
      | TokenType::View
      | TokenType::Device
      | TokenType::Surface
      | TokenType::PostProcess
      | TokenType::Smooth
      | TokenType::Flat
      | TokenType::ScreenSpace
      | TokenType::Cubemap
      | TokenType::Solid
      | TokenType::Thin
      | TokenType::Default
      | TokenType::TwoPassesOneSide
      | TokenType::TwoPassesTwoSides
      | TokenType::Instanced
      | TokenType::Multiview
      | TokenType::Low
      | TokenType::Medium
      | TokenType::High
      | TokenType::Simple
      | TokenType::BentNormals
      | TokenType::Shadow
      | TokenType::Zero
      | TokenType::One
      | TokenType::SrcColor
      | TokenType::OneMinusSrcColor
      | TokenType::DstColor
      | TokenType::OneMinusDstColor
      | TokenType::SrcAlpha
      | TokenType::OneMinusSrcAlpha
      | TokenType::DstAlpha
      | TokenType::OneMinusDstAlpha
      | TokenType::SrcAlphaSaturate
      | TokenType::DirectionalLighting
      | TokenType::DynamicLighting
      | TokenType::ShadowReceiver
      | TokenType::Skinning
      | TokenType::Fog
      | TokenType::Vsm
      | TokenType::Ssr
      | TokenType::Stereo
      | TokenType::Position
      | TokenType::Normal
      | TokenType::Uv0
      | TokenType::Uv1
      | TokenType::Color
      | TokenType::Tangents
      | TokenType::Custom0
      | TokenType::Custom1
      | TokenType::Custom2
      | TokenType::Custom3
      | TokenType::Custom4
      | TokenType::Custom5
      | TokenType::Custom6
      | TokenType::Custom7
      | TokenType::BoneIndices
      | TokenType::BoneWeights => Some(3), // enumMember

      // Parameter types
      TokenType::Bool
      | TokenType::Bool2
      | TokenType::Bool3
      | TokenType::Bool4
      | TokenType::Int
      | TokenType::Int2
      | TokenType::Int3
      | TokenType::Int4
      | TokenType::Uint
      | TokenType::Uint2
      | TokenType::Uint3
      | TokenType::Uint4
      | TokenType::Float
      | TokenType::Float2
      | TokenType::Float3
      | TokenType::Float4
      | TokenType::Mat3
      | TokenType::Mat4
      | TokenType::Float3x3
      | TokenType::Float4x4
      | TokenType::Sampler2d
      | TokenType::Sampler2dArray
      | TokenType::Sampler3d
      | TokenType::SamplerCubemap
      | TokenType::SamplerExternal
      | TokenType::SamplerCubemapArray
      | TokenType::SubpassInput => Some(4), // type

      // Literals
      TokenType::String => Some(5),                  // string
      TokenType::Number => Some(6),                  // number
      TokenType::True | TokenType::False => Some(7), // keyword (for booleans)

      // Block keywords
      TokenType::Material
      | TokenType::Vertex
      | TokenType::Fragment
      | TokenType::Compute
      | TokenType::Tool => Some(8), // keyword

      // Skip punctuation, whitespace, unknown, identifiers, GLSL code
      _ => None,
    };

    if let Some(tt) = token_type_idx {
      let line = token.line;
      let col = token.column;
      let len = token.value.len() as u32;

      let delta_line = line - last_line;
      let delta_start = if delta_line == 0 {
        col - last_char
      } else {
        col
      };

      data.push(SemanticToken {
        delta_line,
        delta_start,
        length: len,
        token_type: tt,
        token_modifiers_bitset: 0,
      });

      last_line = line;
      last_char = col;
    }
  }

  data
}

/// Legend token types (must match indices above).
pub fn token_types() -> Vec<&'static str> {
  vec![
    "comment",
    "property",
    "parameter",
    "enumMember",
    "type",
    "string",
    "number",
    "keyword",
    "keyword",
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_semantic_tokens_basic() {
    let text = r#"material {
      name : Test,
      shadingModel : lit
    }"#;
    let data = generate_semantic_tokens(text);
    assert!(!data.is_empty());
    // Verify tokens have valid properties
    for token in &data {
      assert!(token.length > 0);
      assert!(token.token_type <= 8);
    }
  }
}
