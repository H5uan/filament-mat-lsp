use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum TokenType {
  // Top-level blocks
  Material,
  Vertex,
  Fragment,
  Compute,
  Tool,

  // Material properties
  Name,
  ApiLevel,
  FeatureLevel,
  ShadingModel,
  Domain,
  Interpolation,
  Quality,
  Requires,
  Parameters,
  Constants,
  Variables,
  Buffers,
  Subpasses,
  Outputs,
  Culling,
  Blending,
  BlendFunction,
  PostLightingBlending,
  Transparency,
  MaskThreshold,
  AlphaToCoverage,
  VertexDomain,
  VertexDomainDeviceJittered,
  MaterialDomain,
  DoubleSided,
  ColorWrite,
  DepthWrite,
  DepthCulling,
  DepthTest,
  Instanced,
  RefractionMode,
  RefractionType,
  Reflections,
  ReflectionMode,
  ShadowMultiplier,
  TransparentShadow,
  ClearCoatIorChange,
  MultiBounceAmbientOcclusion,
  SpecularAmbientOcclusion,
  SpecularAntiAliasing,
  SpecularAntiAliasingVariance,
  SpecularAntiAliasingThreshold,
  CustomSurfaceShading,
  FlipUv,
  LinearFog,
  ShadowFarAttenuation,
  FramebufferFetch,
  LegacyMorphing,
  UseDefaultDepthVariant,
  VariantFilter,
  GroupSize,
  StereoscopicType,
  StereoscopicEyeCount,

  // Property names (parameter/constant/output fields)
  Type,
  Precision,
  Format,
  Filterable,
  Multisample,
  TransformName,
  Stages,
  Default,
  Qualifiers,
  Fields,
  Target,
  Location,

  // Enums - shading model
  Lit,
  Unlit,
  Subsurface,
  Cloth,
  SpecularGlossiness,

  // Enums - culling
  Front,
  Back,
  FrontAndBack,
  None,

  // Enums - blending
  Opaque,
  Transparent,
  Fade,
  Masked,
  Add,
  Multiply,
  Screen,
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
  // Default already defined above
  TwoPassesOneSide,
  TwoPassesTwoSides,

  // Enums - stereoscopic type
  // Instanced already defined above
  Multiview,

  // Enums - quality / precision
  Low,
  Medium,
  High,

  // Enums - specular ambient occlusion
  Simple,
  BentNormals,

  // Enums - sampler format
  Shadow,

  // Enums - blend functions
  Zero,
  One,
  SrcColor,
  OneMinusSrcColor,
  DstColor,
  OneMinusDstColor,
  SrcAlpha,
  OneMinusSrcAlpha,
  DstAlpha,
  OneMinusDstAlpha,
  SrcAlphaSaturate,

  // Enums - variant filter
  DirectionalLighting,
  DynamicLighting,
  ShadowReceiver,
  Skinning,
  Fog,
  Vsm,
  Ssr,
  Stereo,

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
  Custom5,
  Custom6,
  Custom7,
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
  Float3x3,
  Float4x4,
  Sampler2d,
  Sampler2dArray,
  Sampler3d,
  SamplerCubemap,
  SamplerExternal,
  SamplerCubemapArray,
  SubpassInput,

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
  pub token_type: TokenType,
  pub value: String,
  pub line: u32,
  pub column: u32,
}

impl Token {
  pub fn new(token_type: TokenType, value: &str, line: u32, column: u32) -> Self {
    Self {
      token_type,
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
    self.token_type == *token_type
  }
}
