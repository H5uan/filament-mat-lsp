use filament_mat_lsp::lexer::Lexer;
use filament_mat_lsp::schema::{KeywordType, lookup_keyword};
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

      // String / number literals
      TokenType::StringLiteral => Some(5), // string
      TokenType::NumberLiteral => Some(6), // number
      TokenType::BoolLiteral => Some(7),   // keyword (for booleans)

      // Block keywords: material, vertex, fragment, compute, tool
      TokenType::BlockKeyword => Some(8), // keyword

      // Identifiers: classify by schema
      TokenType::Identifier => {
        match lookup_keyword(&token.value) {
          Some(KeywordType::MaterialProperty) => Some(1), // property
          Some(KeywordType::ParameterField) => Some(2),   // parameter
          Some(KeywordType::ShadingModel)
          | Some(KeywordType::BlendingMode)
          | Some(KeywordType::CullingMode)
          | Some(KeywordType::VertexDomain)
          | Some(KeywordType::MaterialDomain)
          | Some(KeywordType::InterpolationMode)
          | Some(KeywordType::RefractionMode)
          | Some(KeywordType::RefractionType)
          | Some(KeywordType::ReflectionMode)
          | Some(KeywordType::TransparencyMode)
          | Some(KeywordType::StereoscopicType)
          | Some(KeywordType::QualityLevel)
          | Some(KeywordType::SpecularAmbientOcclusionMode)
          | Some(KeywordType::PrecisionValue)
          | Some(KeywordType::SamplerFormat)
          | Some(KeywordType::BlendFunction)
          | Some(KeywordType::VertexAttribute)
          | Some(KeywordType::VariantFilterValue) => Some(3), // enumMember
          Some(KeywordType::ParameterType) => Some(4),    // type
          Some(KeywordType::TopLevelBlock)
          | Some(KeywordType::BoolLiteral)
          | Some(KeywordType::NullLiteral)
          | None => None,
        }
      }

      // Skip punctuation, whitespace, unknown, GLSL code
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
