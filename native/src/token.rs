use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TokenType {
  // Top-level blocks
  Material,
  Vertex,
  Fragment,
  Compute,

  // Material properties
  Name,
  ShadingModel,
  Requires,
  Culling,
  Blending,
  MaskThreshold,
  AlphaToCoverage,
  VertexDomain,
  MaterialDomain,
  Interpolation,
  DoubleSided,
  ColorWrite,
  DepthWrite,
  DepthTest,
  Instanced,
  RefractionMode,
  RefractionType,
  ReflectionMode,
  TransparencyMode,
  ShadowMultiplier,
  SpecularAntiAliasing,
  SpecularAntiAliasingVariance,
  SpecularAntiAliasingThreshold,
  ClearCoatIorChange,
  FlipUv,
  LinearFog,
  MultiBounceAmbientOcclusion,
  SpecularAmbientOcclusion,
  CustomSurfaceShading,
  StereoscopicType,
  StereoscopicEyeCount,
  GroupSize,
  VariantFilter,
  Parameters,
  Constants,
  Variables,

  // Property names
  Type,
  Precision,
  Format,
  Filterable,
  Multisample,
  TransformName,
  Stages,
  Default,

  // Enums - shading model
  Lit,
  Unlit,
  Subsurface,
  Cloth,
  SpecularGlossiness,

  // Enums - culling
  Front,
  Back,
  None,

  // Enums - blending
  Opaque,
  Transparent,
  Fade,
  Masked,
  Add,
  Custom,

  // Enums - vertex domain
  Object,
  World,
  View,
  Device,

  // Enums - material domain
  Surface,
  PostProcess,

  // Enums - interpolation
  Smooth,
  Flat,

  // Enums - refraction mode
  ScreenSpace,
  Cubemap,

  // Enums - refraction type
  Solid,
  Thin,

  // Enums - transparency mode
  TwoPassesOneSide,
  TwoPassesTwoSides,

  // Enums - vertex attributes
  Position,
  Normal,
  Uv0,
  Uv1,
  Color,
  Tangents,
  Custom0,
  Custom1,
  Custom2,
  Custom3,
  Custom4,
  BoneIndices,
  BoneWeights,

  // Parameter types
  Bool,
  Bool2,
  Bool3,
  Bool4,
  Int,
  Int2,
  Int3,
  Int4,
  Uint,
  Uint2,
  Uint3,
  Uint4,
  Float,
  Float2,
  Float3,
  Float4,
  Mat3,
  Mat4,
  Sampler2d,
  Sampler3d,
  SamplerCubemap,
  SamplerExternal,

  // Literals
  True,
  False,
  Null,
  Number,
  String,

  // Punctuation
  LCurly,
  RCurly,
  LBracket,
  RBracket,
  Colon,
  Comma,

  // Other
  Identifier,
  Comment,
  GlslCode,
  Whitespace,
  Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Token {
  pub token_type: String,
  pub value: String,
  pub line: u32,
  pub column: u32,
}

impl Token {
  pub fn new(token_type: TokenType, value: &str, line: u32, column: u32) -> Self {
    Self {
      token_type: format!("{token_type:?}"),
      value: value.to_string(),
      line,
      column,
    }
  }
}

// Helper trait to make working with tokens easier
pub trait TokenExt {
  fn is_type(&self, token_type: &TokenType) -> bool;
}

impl TokenExt for Token {
  fn is_type(&self, token_type: &TokenType) -> bool {
    self.token_type == format!("{token_type:?}")
  }
}
