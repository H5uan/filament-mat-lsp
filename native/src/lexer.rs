use crate::schema::{KeywordType, lookup_keyword};
use crate::token::{Token, TokenType};
use std::iter::Peekable;
use std::str::Chars;

// ---------------------------------------------------------------------------
// Lexer state
// ---------------------------------------------------------------------------
struct LexerState<'a> {
  chars: Peekable<Chars<'a>>,
  line: u32,
  column: u32,
}

impl<'a> LexerState<'a> {
  fn new(input: &'a str) -> Self {
    Self {
      chars: input.chars().peekable(),
      line: 1,
      column: 1,
    }
  }

  fn advance(&mut self) -> Option<char> {
    let ch = self.chars.next()?;
    match ch {
      '\n' => {
        self.line += 1;
        self.column = 1;
      }
      '\r' => {
        if let Some('\n') = self.chars.peek() {
          self.chars.next();
        }
        self.line += 1;
        self.column = 1;
      }
      _ => self.column += 1,
    }
    Some(ch)
  }

  fn peek(&mut self) -> Option<&char> {
    self.chars.peek()
  }

  fn current_pos(&self) -> (u32, u32) {
    (self.line, self.column)
  }
}

// ---------------------------------------------------------------------------
// Lexer modes
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
enum LexerMode {
  TopLevel,
  MaterialBlock { brace_depth: usize },
  ShaderBlock { brace_depth: usize },
}

// ---------------------------------------------------------------------------
// Unified Lexer
// ---------------------------------------------------------------------------
pub struct Lexer<'a> {
  state: LexerState<'a>,
  mode: LexerMode,
  pending_block_name: Option<String>,
}

impl<'a> Lexer<'a> {
  pub fn new(input: &'a str) -> Self {
    Self {
      state: LexerState::new(input),
      mode: LexerMode::TopLevel,
      pending_block_name: None,
    }
  }

  pub fn tokenize(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();
    while let Some(&ch) = self.state.peek() {
      match &self.mode {
        LexerMode::TopLevel => {
          if ch.is_whitespace() {
            self.state.advance();
            continue;
          }
          if ch == '/' {
            let comment = self.read_comment();
            tokens.push(comment);
            continue;
          }

          // Try to read an identifier (top-level block keyword)
          if ch.is_ascii_alphabetic() || ch == '_' {
            let (line, col) = self.state.current_pos();
            let ident = self.read_identifier_raw();
            let tt = match lookup_keyword(&ident) {
              Some(KeywordType::TopLevelBlock) => {
                self.pending_block_name = Some(ident.clone());
                match ident.as_str() {
                  "material" => TokenType::Material,
                  "vertex" => TokenType::Vertex,
                  "fragment" => TokenType::Fragment,
                  "compute" => TokenType::Compute,
                  "tool" => TokenType::Tool,
                  _ => TokenType::Identifier,
                }
              }
              _ => TokenType::Identifier,
            };
            tokens.push(Token::new(tt, &ident, line, col));
            continue;
          }

          // Expect '{' after a top-level block keyword
          if ch == '{' {
            let (line, col) = self.state.current_pos();
            self.state.advance();
            tokens.push(Token::new(TokenType::LCurly, "{", line, col));

            // Switch mode based on pending block name
            match self.pending_block_name.take().as_deref() {
              Some("material") => {
                self.mode = LexerMode::MaterialBlock { brace_depth: 1 };
              }
              Some(_) => {
                self.mode = LexerMode::ShaderBlock { brace_depth: 1 };
              }
              None => {
                // Stray '{', stay in TopLevel
              }
            }
            continue;
          }

          // Unknown character at top level - skip
          let (line, col) = self.state.current_pos();
          let s = ch.to_string();
          self.state.advance();
          tokens.push(Token::new(TokenType::Unknown, &s, line, col));
        }

        LexerMode::MaterialBlock { brace_depth } => {
          let depth = *brace_depth;
          if let Some(tok) = self.tokenize_material_token(depth) {
            match tok.token_type {
              TokenType::LCurly => {
                self.mode = LexerMode::MaterialBlock {
                  brace_depth: depth + 1,
                };
              }
              TokenType::RCurly => {
                if depth == 1 {
                  self.mode = LexerMode::TopLevel;
                } else {
                  self.mode = LexerMode::MaterialBlock {
                    brace_depth: depth - 1,
                  };
                }
              }
              _ => {}
            }
            tokens.push(tok);
          }
        }

        LexerMode::ShaderBlock { brace_depth } => {
          let depth = *brace_depth;
          if let Some((tok, new_depth)) = self.tokenize_shader_token(depth) {
            if tok.token_type == TokenType::RCurly && new_depth == 0 {
              self.mode = LexerMode::TopLevel;
            } else {
              self.mode = LexerMode::ShaderBlock {
                brace_depth: new_depth,
              };
            }
            tokens.push(tok);
          }
        }
      }
    }
    tokens
  }

  // -------------------------------------------------------------------------
  // Material block tokenization (Jsonish)
  // -------------------------------------------------------------------------
  fn tokenize_material_token(&mut self, _brace_depth: usize) -> Option<Token> {
    let ch = loop {
      let &c = self.state.peek()?;

      if c.is_whitespace() {
        self.state.advance();
        continue;
      }

      if c == '/' {
        return Some(self.read_comment());
      }

      break c;
    };

    let (line, col) = self.state.current_pos();

    match ch {
      '{' => {
        self.state.advance();
        Some(Token::new(TokenType::LCurly, "{", line, col))
      }
      '}' => {
        self.state.advance();
        Some(Token::new(TokenType::RCurly, "}", line, col))
      }
      '[' => {
        self.state.advance();
        Some(Token::new(TokenType::LBracket, "[", line, col))
      }
      ']' => {
        self.state.advance();
        Some(Token::new(TokenType::RBracket, "]", line, col))
      }
      ':' => {
        self.state.advance();
        Some(Token::new(TokenType::Colon, ":", line, col))
      }
      ',' => {
        self.state.advance();
        Some(Token::new(TokenType::Comma, ",", line, col))
      }
      '"' => Some(self.read_string()),
      '0'..='9' | '-' => Some(self.read_number()),
      'a'..='z' | 'A'..='Z' | '_' => {
        let ident = self.read_identifier_raw();
        let tt = Self::map_identifier_to_token_type(&ident);
        Some(Token::new(tt, &ident, line, col))
      }
      _ => {
        let s = ch.to_string();
        self.state.advance();
        Some(Token::new(TokenType::Unknown, &s, line, col))
      }
    }
  }

  // -------------------------------------------------------------------------
  // Shader block tokenization (raw GLSL)
  // -------------------------------------------------------------------------
  fn tokenize_shader_token(&mut self, mut brace_depth: usize) -> Option<(Token, usize)> {
    let start_line = self.state.line;
    let start_col = self.state.column;
    let mut code = String::new();

    while let Some(&ch) = self.state.peek() {
      if ch == '{' {
        self.state.advance();
        brace_depth += 1;
        code.push('{');
      } else if ch == '}' {
        if brace_depth == 1 {
          // This is the closing brace of the shader block.
          // Return accumulated GLSL code WITHOUT consuming '}'.
          // Next call will see '}' and emit RCurly.
          if !code.is_empty() {
            return Some((
              Token::new(TokenType::GlslCode, &code, start_line, start_col),
              brace_depth,
            ));
          } else {
            // No code accumulated, consume '}' and return it
            self.state.advance();
            return Some((
              Token::new(TokenType::RCurly, "}", self.state.line, self.state.column),
              0,
            ));
          }
        } else {
          self.state.advance();
          brace_depth -= 1;
          code.push('}');
        }
      } else {
        code.push(ch);
        self.state.advance();
      }
    }

    // EOF while in shader block
    if !code.is_empty() {
      Some((
        Token::new(TokenType::GlslCode, &code, start_line, start_col),
        brace_depth,
      ))
    } else {
      None
    }
  }

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------
  fn read_comment(&mut self) -> Token {
    let (line, col) = self.state.current_pos();
    self.state.advance(); // '/'
    if let Some(&'/') = self.state.peek() {
      self.state.advance();
      let mut comment = String::from("//");
      while let Some(&ch) = self.state.peek() {
        if ch == '\n' {
          break;
        }
        comment.push(ch);
        self.state.advance();
      }
      Token::new(TokenType::Comment, &comment, line, col)
    } else if let Some(&'*') = self.state.peek() {
      self.state.advance();
      let mut comment = String::from("/*");
      while let Some(&ch) = self.state.peek() {
        comment.push(ch);
        self.state.advance();
        if ch == '*' && self.state.peek() == Some(&'/') {
          comment.push('/');
          self.state.advance();
          break;
        }
      }
      Token::new(TokenType::Comment, &comment, line, col)
    } else {
      Token::new(TokenType::Unknown, "/", line, col)
    }
  }

  fn read_string(&mut self) -> Token {
    let (line, col) = self.state.current_pos();
    self.state.advance(); // '"'
    let mut s = String::from("\"");
    while let Some(&ch) = self.state.peek() {
      if ch == '"' {
        s.push('"');
        self.state.advance();
        break;
      } else if ch == '\\' {
        s.push('\\');
        self.state.advance();
        if let Some(&escaped) = self.state.peek() {
          s.push(escaped);
          self.state.advance();
        }
      } else {
        s.push(ch);
        self.state.advance();
      }
    }
    Token::new(TokenType::String, &s, line, col)
  }

  fn read_number(&mut self) -> Token {
    let (line, col) = self.state.current_pos();
    let mut num = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == 'e' || ch == 'E' || ch == '+' {
        num.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    Token::new(TokenType::Number, &num, line, col)
  }

  fn read_identifier_raw(&mut self) -> String {
    let mut ident = String::new();
    while let Some(&ch) = self.state.peek() {
      if ch.is_ascii_alphanumeric() || ch == '_' {
        ident.push(ch);
        self.state.advance();
      } else {
        break;
      }
    }
    ident
  }

  fn map_identifier_to_token_type(ident: &str) -> TokenType {
    match lookup_keyword(ident) {
      Some(KeywordType::MaterialProperty) => match ident {
        "name" => TokenType::Name,
        "apiLevel" => TokenType::ApiLevel,
        "featureLevel" => TokenType::FeatureLevel,
        "shadingModel" => TokenType::ShadingModel,
        "domain" => TokenType::Domain,
        "interpolation" => TokenType::Interpolation,
        "quality" => TokenType::Quality,
        "requires" => TokenType::Requires,
        "parameters" => TokenType::Parameters,
        "constants" => TokenType::Constants,
        "variables" => TokenType::Variables,
        "buffers" => TokenType::Buffers,
        "subpasses" => TokenType::Subpasses,
        "outputs" => TokenType::Outputs,
        "culling" => TokenType::Culling,
        "blending" => TokenType::Blending,
        "blendFunction" => TokenType::BlendFunction,
        "postLightingBlending" => TokenType::PostLightingBlending,
        "transparency" => TokenType::Transparency,
        "maskThreshold" => TokenType::MaskThreshold,
        "alphaToCoverage" => TokenType::AlphaToCoverage,
        "vertexDomain" => TokenType::VertexDomain,
        "vertexDomainDeviceJittered" => TokenType::VertexDomainDeviceJittered,
        "materialDomain" => TokenType::MaterialDomain,
        "doubleSided" => TokenType::DoubleSided,
        "colorWrite" => TokenType::ColorWrite,
        "depthWrite" => TokenType::DepthWrite,
        "depthCulling" => TokenType::DepthCulling,
        "depthTest" => TokenType::DepthTest,
        "instanced" => TokenType::Instanced,
        "refractionMode" => TokenType::RefractionMode,
        "refractionType" => TokenType::RefractionType,
        "reflections" => TokenType::Reflections,
        "reflectionMode" => TokenType::ReflectionMode,
        "shadowMultiplier" => TokenType::ShadowMultiplier,
        "transparentShadow" => TokenType::TransparentShadow,
        "clearCoatIorChange" => TokenType::ClearCoatIorChange,
        "multiBounceAmbientOcclusion" => TokenType::MultiBounceAmbientOcclusion,
        "specularAmbientOcclusion" => TokenType::SpecularAmbientOcclusion,
        "specularAntiAliasing" => TokenType::SpecularAntiAliasing,
        "specularAntiAliasingVariance" => TokenType::SpecularAntiAliasingVariance,
        "specularAntiAliasingThreshold" => TokenType::SpecularAntiAliasingThreshold,
        "customSurfaceShading" => TokenType::CustomSurfaceShading,
        "flipUV" => TokenType::FlipUv,
        "linearFog" => TokenType::LinearFog,
        "shadowFarAttenuation" => TokenType::ShadowFarAttenuation,
        "framebufferFetch" => TokenType::FramebufferFetch,
        "legacyMorphing" => TokenType::LegacyMorphing,
        "useDefaultDepthVariant" => TokenType::UseDefaultDepthVariant,
        "variantFilter" => TokenType::VariantFilter,
        "groupSize" => TokenType::GroupSize,
        "stereoscopicType" => TokenType::StereoscopicType,
        "stereoscopicEyeCount" => TokenType::StereoscopicEyeCount,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::ParameterField) => match ident {
        "type" => TokenType::Type,
        "name" => TokenType::Name,
        "precision" => TokenType::Precision,
        "format" => TokenType::Format,
        "filterable" => TokenType::Filterable,
        "multisample" => TokenType::Multisample,
        "transformName" => TokenType::TransformName,
        "stages" => TokenType::Stages,
        "default" => TokenType::Default,
        "qualifiers" => TokenType::Qualifiers,
        "fields" => TokenType::Fields,
        "target" => TokenType::Target,
        "location" => TokenType::Location,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::ShadingModel) => match ident {
        "lit" => TokenType::Lit,
        "unlit" => TokenType::Unlit,
        "subsurface" => TokenType::Subsurface,
        "cloth" => TokenType::Cloth,
        "specularGlossiness" => TokenType::SpecularGlossiness,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::BlendingMode) => match ident {
        "opaque" => TokenType::Opaque,
        "transparent" => TokenType::Transparent,
        "fade" => TokenType::Fade,
        "masked" => TokenType::Masked,
        "add" => TokenType::Add,
        "multiply" => TokenType::Multiply,
        "screen" => TokenType::Screen,
        "custom" => TokenType::Custom,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::CullingMode) => match ident {
        "front" => TokenType::Front,
        "back" => TokenType::Back,
        "frontAndBack" => TokenType::FrontAndBack,
        "none" => TokenType::None,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::VertexDomain) => match ident {
        "object" => TokenType::Object,
        "world" => TokenType::World,
        "view" => TokenType::View,
        "device" => TokenType::Device,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::MaterialDomain) => match ident {
        "surface" => TokenType::Surface,
        "postprocess" => TokenType::PostProcess,
        "compute" => TokenType::Compute,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::InterpolationMode) => match ident {
        "smooth" => TokenType::Smooth,
        "flat" => TokenType::Flat,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::RefractionMode) => match ident {
        "none" => TokenType::None,
        "cubemap" => TokenType::Cubemap,
        "screenspace" => TokenType::ScreenSpace,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::RefractionType) => match ident {
        "solid" => TokenType::Solid,
        "thin" => TokenType::Thin,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::ReflectionMode) => match ident {
        "default" => TokenType::Default,
        "screenspace" => TokenType::ScreenSpace,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::TransparencyMode) => match ident {
        "default" => TokenType::Default,
        "twoPassesOneSide" => TokenType::TwoPassesOneSide,
        "twoPassesTwoSides" => TokenType::TwoPassesTwoSides,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::StereoscopicType) => match ident {
        "none" => TokenType::None,
        "instanced" => TokenType::Instanced,
        "multiview" => TokenType::Multiview,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::QualityLevel) => match ident {
        "default" => TokenType::Default,
        "low" => TokenType::Low,
        "normal" => TokenType::Normal,
        "high" => TokenType::High,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::SpecularAmbientOcclusionMode) => match ident {
        "none" => TokenType::None,
        "simple" => TokenType::Simple,
        "bentNormals" => TokenType::BentNormals,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::PrecisionValue) => match ident {
        "default" => TokenType::Default,
        "low" => TokenType::Low,
        "medium" => TokenType::Medium,
        "high" => TokenType::High,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::SamplerFormat) => match ident {
        "float" => TokenType::Float,
        "int" => TokenType::Int,
        "uint" => TokenType::Uint,
        "shadow" => TokenType::Shadow,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::BlendFunction) => match ident {
        "zero" => TokenType::Zero,
        "one" => TokenType::One,
        "srcColor" => TokenType::SrcColor,
        "oneMinusSrcColor" => TokenType::OneMinusSrcColor,
        "dstColor" => TokenType::DstColor,
        "oneMinusDstColor" => TokenType::OneMinusDstColor,
        "srcAlpha" => TokenType::SrcAlpha,
        "oneMinusSrcAlpha" => TokenType::OneMinusSrcAlpha,
        "dstAlpha" => TokenType::DstAlpha,
        "oneMinusDstAlpha" => TokenType::OneMinusDstAlpha,
        "srcAlphaSaturate" => TokenType::SrcAlphaSaturate,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::VertexAttribute) => match ident {
        "position" => TokenType::Position,
        "normal" => TokenType::Normal,
        "uv0" => TokenType::Uv0,
        "uv1" => TokenType::Uv1,
        "color" => TokenType::Color,
        "tangents" => TokenType::Tangents,
        "custom0" => TokenType::Custom0,
        "custom1" => TokenType::Custom1,
        "custom2" => TokenType::Custom2,
        "custom3" => TokenType::Custom3,
        "custom4" => TokenType::Custom4,
        "custom5" => TokenType::Custom5,
        "custom6" => TokenType::Custom6,
        "custom7" => TokenType::Custom7,
        "boneIndices" => TokenType::BoneIndices,
        "boneWeights" => TokenType::BoneWeights,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::VariantFilterValue) => match ident {
        "directionalLighting" => TokenType::DirectionalLighting,
        "dynamicLighting" => TokenType::DynamicLighting,
        "shadowReceiver" => TokenType::ShadowReceiver,
        "skinning" => TokenType::Skinning,
        "fog" => TokenType::Fog,
        "vsm" => TokenType::Vsm,
        "ssr" => TokenType::Ssr,
        "stereo" => TokenType::Stereo,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::ParameterType) => match ident {
        "bool" => TokenType::Bool,
        "bool2" => TokenType::Bool2,
        "bool3" => TokenType::Bool3,
        "bool4" => TokenType::Bool4,
        "int" => TokenType::Int,
        "int2" => TokenType::Int2,
        "int3" => TokenType::Int3,
        "int4" => TokenType::Int4,
        "uint" => TokenType::Uint,
        "uint2" => TokenType::Uint2,
        "uint3" => TokenType::Uint3,
        "uint4" => TokenType::Uint4,
        "float" => TokenType::Float,
        "float2" => TokenType::Float2,
        "float3" => TokenType::Float3,
        "float4" => TokenType::Float4,
        "mat3" => TokenType::Mat3,
        "mat4" => TokenType::Mat4,
        "float3x3" => TokenType::Float3x3,
        "float4x4" => TokenType::Float4x4,
        "sampler2d" => TokenType::Sampler2d,
        "sampler2dArray" => TokenType::Sampler2dArray,
        "sampler3d" => TokenType::Sampler3d,
        "samplerCubemap" => TokenType::SamplerCubemap,
        "samplerExternal" => TokenType::SamplerExternal,
        "samplerCubemapArray" => TokenType::SamplerCubemapArray,
        "subpassInput" => TokenType::SubpassInput,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::BoolLiteral) => match ident {
        "true" => TokenType::True,
        "false" => TokenType::False,
        _ => TokenType::Identifier,
      },
      Some(KeywordType::NullLiteral) => TokenType::Null,
      _ => TokenType::Identifier,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::token::TokenExt;

  #[test]
  fn test_lex_toplevel_blocks() {
    let input = "material {} vertex {} fragment {} compute {}";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    assert_eq!(tokens.len(), 12);
    assert!(tokens[0].is_type(&TokenType::Material));
    assert!(tokens[1].is_type(&TokenType::LCurly));
    assert!(tokens[2].is_type(&TokenType::RCurly));
    assert!(tokens[3].is_type(&TokenType::Vertex));
    assert!(tokens[4].is_type(&TokenType::LCurly));
    assert!(tokens[5].is_type(&TokenType::RCurly));
    assert!(tokens[6].is_type(&TokenType::Fragment));
    assert!(tokens[7].is_type(&TokenType::LCurly));
    assert!(tokens[8].is_type(&TokenType::RCurly));
    assert!(tokens[9].is_type(&TokenType::Compute));
    assert!(tokens[10].is_type(&TokenType::LCurly));
    assert!(tokens[11].is_type(&TokenType::RCurly));
  }

  #[test]
  fn test_lex_material_properties() {
    let input = "material { name : Test, shadingModel : lit, blending : opaque }";
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    let types: Vec<_> = tokens.iter().map(|t| &t.token_type).collect();
    assert!(types.contains(&&TokenType::Name));
    assert!(types.contains(&&TokenType::ShadingModel));
    assert!(types.contains(&&TokenType::Lit));
    assert!(types.contains(&&TokenType::Blending));
    assert!(types.contains(&&TokenType::Opaque));
  }

  #[test]
  fn test_lex_shader_block() {
    let input = r#"fragment {
      void material(inout MaterialInputs material) {
        prepareMaterial(material);
      }
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    // Should have Fragment, LCurly, GlslCode, RCurly
    assert!(tokens[0].is_type(&TokenType::Fragment));
    assert!(tokens[1].is_type(&TokenType::LCurly));
    // Find GlslCode token
    let has_glsl = tokens.iter().any(|t| t.is_type(&TokenType::GlslCode));
    assert!(has_glsl, "Expected GlslCode token in shader block");
    assert!(tokens.last().unwrap().is_type(&TokenType::RCurly));
  }

  #[test]
  fn test_lex_all_properties() {
    let input = r#"material {
      name : Test,
      apiLevel : 1,
      featureLevel : 2,
      shadingModel : lit,
      domain : surface,
      interpolation : smooth,
      quality : high,
      requires : [position, uv0],
      parameters : [{type : float4, name : color}],
      culling : back,
      blending : transparent,
      postLightingBlending : add,
      transparency : twoPassesOneSide,
      maskThreshold : 0.5,
      alphaToCoverage : true,
      vertexDomain : world,
      vertexDomainDeviceJittered : false,
      depthCulling : true,
      reflections : screenspace,
      specularAmbientOcclusion : bentNormals,
      variantFilter : [directionalLighting, shadowReceiver],
      stereoscopicType : multiview
    }"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize();
    // Should not contain any Unknown tokens
    let has_unknown = tokens.iter().any(|t| t.is_type(&TokenType::Unknown));
    assert!(
      !has_unknown,
      "Lexer produced Unknown tokens for known properties"
    );
  }
}
