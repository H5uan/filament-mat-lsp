use crate::parser::MatFile;
use lsp_types::{DocumentHighlight, DocumentHighlightKind, Location, Position, Range, Uri};

/// Internal representation of a reference found in the document.
struct Reference {
  range: Range,
  is_write: bool,
}

/// Find all references to a symbol within a .mat file.
fn find_all_refs(matfile: &MatFile, symbol: &str) -> Vec<Reference> {
  let mut refs = Vec::new();

  // 1. Parameter definition (Write)
  for param in &matfile.material.parameters {
    if param.name == symbol {
      refs.push(Reference {
        range: Range {
          start: Position {
            line: param.range.start.line,
            character: param.range.start.character,
          },
          end: Position {
            line: param.range.end.line,
            character: param.range.end.character,
          },
        },
        is_write: true,
      });
    }
  }

  // 2. Shader code references (Read)
  for shader in &matfile.shaders {
    let code = &shader.code;

    // Search for materialParams.symbol
    let search_dot = format!("materialParams.{}", symbol);
    for (idx, _) in code.match_indices(&search_dot) {
      let start_char = idx as u32 + "materialParams.".len() as u32;
      let end_char = start_char + symbol.len() as u32;

      refs.push(Reference {
        range: Range {
          start: Position {
            line: shader.range.start.line + 1, // approximate line within shader
            character: start_char,
          },
          end: Position {
            line: shader.range.start.line + 1,
            character: end_char,
          },
        },
        is_write: false,
      });
    }

    // Search for materialParams_symbol
    let search_underscore = format!("materialParams_{}", symbol);
    for (idx, _) in code.match_indices(&search_underscore) {
      let start_char = idx as u32 + "materialParams_".len() as u32;
      let end_char = start_char + symbol.len() as u32;

      refs.push(Reference {
        range: Range {
          start: Position {
            line: shader.range.start.line + 1,
            character: start_char,
          },
          end: Position {
            line: shader.range.start.line + 1,
            character: end_char,
          },
        },
        is_write: false,
      });
    }
  }

  refs
}

/// Find references as DocumentHighlights (for textDocument/documentHighlight).
pub fn find_references(matfile: &MatFile, symbol: &str, _uri: &Uri) -> Vec<DocumentHighlight> {
  find_all_refs(matfile, symbol)
    .into_iter()
    .map(|r| DocumentHighlight {
      range: r.range,
      kind: Some(if r.is_write {
        DocumentHighlightKind::WRITE
      } else {
        DocumentHighlightKind::READ
      }),
    })
    .collect()
}

/// Find references as Locations (for textDocument/references).
pub fn find_reference_locations(matfile: &MatFile, symbol: &str, uri: &Uri) -> Vec<Location> {
  find_all_refs(matfile, symbol)
    .into_iter()
    .map(|r| Location {
      uri: uri.clone(),
      range: r.range,
    })
    .collect()
}

/// Extract the word at a given position in the document text.
pub fn extract_word_at_position(
  text: &str,
  position: Position,
  line_offsets: &[usize],
) -> Option<String> {
  let line = position.line as usize;
  if line >= line_offsets.len() {
    return None;
  }

  let line_start = line_offsets[line];
  let line_end = if line + 1 < line_offsets.len() {
    line_offsets[line + 1]
  } else {
    text.len()
  };

  let line_text = &text[line_start..line_end];
  let mut byte_offset = 0usize;
  let mut utf16_count = 0u32;

  // Find byte offset from character count
  for c in line_text.chars() {
    if utf16_count >= position.character {
      break;
    }
    utf16_count += c.len_utf16() as u32;
    byte_offset += c.len_utf8();
  }

  let abs_offset = line_start + byte_offset;
  if abs_offset >= text.len() {
    return None;
  }

  let mut start = abs_offset;
  let mut end = abs_offset;

  // Go backwards to find word start
  while start > 0 {
    let prev = text[..start].chars().last()?;
    if !is_word_char(prev) {
      break;
    }
    start -= prev.len_utf8();
  }

  // Go forwards to find word end
  while end < text.len() {
    let next = text[end..].chars().next()?;
    if !is_word_char(next) {
      break;
    }
    end += next.len_utf8();
  }

  if start < end {
    Some(text[start..end].to_string())
  } else {
    None
  }
}

fn is_word_char(c: char) -> bool {
  c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::{TextPosition, TextRange};
  use crate::parser::{MatFile, Material, Parameter, ShaderBlock, ShaderBlockType};
  use std::str::FromStr;

  fn dummy_range() -> TextRange {
    TextRange {
      start: TextPosition {
        line: 0,
        character: 0,
      },
      end: TextPosition {
        line: 0,
        character: 0,
      },
    }
  }

  #[test]
  fn test_find_references() {
    let matfile = MatFile {
      material: Material {
        range: dummy_range(),
        name: None,
        shading_model: None,
        requires: crate::parser::Located::new(vec![], dummy_range()),
        parameters: vec![Parameter {
          param_type: "float4".to_string(),
          name: "color".to_string(),
          other_fields: vec![],
          range: dummy_range(),
        }],
        other_properties: vec![],
      },
      shaders: vec![ShaderBlock {
        block_type: ShaderBlockType::Fragment,
        code: "material.baseColor = materialParams.color;".to_string(),
        range: TextRange {
          start: TextPosition {
            line: 10,
            character: 0,
          },
          end: TextPosition {
            line: 15,
            character: 1,
          },
        },
      }],
      errors: vec![],
    };

    let uri = Uri::from_str("file:///test.mat").unwrap();
    let refs = find_references(&matfile, "color", &uri);

    // Should find: 1 parameter definition + 1 shader reference
    assert_eq!(refs.len(), 2);
    assert!(
      refs
        .iter()
        .any(|r| r.kind == Some(DocumentHighlightKind::WRITE))
    );
    assert!(
      refs
        .iter()
        .any(|r| r.kind == Some(DocumentHighlightKind::READ))
    );
  }

  #[test]
  fn test_extract_word_at_position() {
    let text = "material { name : Test }";
    let offsets = vec![0, text.len()];

    let word = extract_word_at_position(
      text,
      Position {
        line: 0,
        character: 12,
      },
      &offsets,
    );
    assert_eq!(word, Some("name".to_string()));
  }
}
