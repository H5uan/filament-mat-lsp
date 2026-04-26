use crate::diagnostics::TextRange;
use crate::parser::{MatFile, Material, Value};
use lsp_types::{Position, Range, SelectionRange};

/// Build selection ranges for a position in a .mat file.
pub fn build_selection_ranges(matfile: &MatFile, position: Position) -> Vec<SelectionRange> {
  // Collect all ranges that contain the position, from smallest to largest
  let mut candidates: Vec<(Range, u32)> = Vec::new(); // (range, depth)

  // Check material block
  let material_range = to_lsp_range(&matfile.material.range);
  if position_in_range(position, material_range) {
    candidates.push((material_range, 0));
  }

  // Check parameter definitions
  for param in &matfile.material.parameters {
    let param_range = to_lsp_range(&param.range);
    if position_in_range(position, param_range) {
      candidates.push((param_range, 1));

      // Parameter name is a sub-range
      // We approximate: name field is usually near the start of the parameter object
      // For simplicity, we just use the parameter range itself
    }
  }

  // Check property values in other_properties
  for (key, value) in &matfile.material.other_properties {
    let value_range = to_lsp_range(&value.range);
    if position_in_range(position, value_range) {
      candidates.push((value_range, 1));

      // If the value is an identifier, add a sub-range for just the identifier
      if matches!(value.value, Value::Identifier(_) | Value::String(_)) {
        let inner_range = narrow_to_value_range(&matfile.material, key, &value.value, position);
        if let Some(r) = inner_range {
          candidates.push((r, 2));
        }
      }
    }
  }

  // Check name property
  if let Some(name) = &matfile.material.name {
    let name_range = to_lsp_range(&name.range);
    if position_in_range(position, name_range) {
      candidates.push((name_range, 1));
    }
  }

  // Check shading_model property
  if let Some(sm) = &matfile.material.shading_model {
    let sm_range = to_lsp_range(&sm.range);
    if position_in_range(position, sm_range) {
      candidates.push((sm_range, 1));

      // Add sub-range for just the value
      let value_start = Position {
        line: sm.range.start.line,
        character: sm.range.start.character + "shadingModel : ".len() as u32,
      };
      let value_end = Position {
        line: sm.range.end.line,
        character: sm.range.end.character,
      };
      let value_range = Range {
        start: value_start,
        end: value_end,
      };
      if position_in_range(position, value_range) {
        candidates.push((value_range, 2));
      }
    }
  }

  // Sort by "depth" (descending) so smallest ranges come first
  candidates.sort_by_key(|(_, depth)| std::cmp::Reverse(*depth));

  // Remove duplicates
  candidates.dedup_by(|a, b| a.0 == b.0);

  // Build SelectionRange chain from largest to smallest
  let mut parent: Option<Box<SelectionRange>> = None;

  for (range, _) in candidates.into_iter().rev() {
    let current = SelectionRange { range, parent };
    parent = Some(Box::new(current));
  }

  // Unpack the chain into a vector (smallest first)
  let mut unpacked = Vec::new();
  let mut current = parent;
  while let Some(node) = current {
    unpacked.push(*node.clone());
    current = node.parent;
  }

  unpacked
}

/// Convert internal TextRange to LSP Range.
fn to_lsp_range(range: &TextRange) -> Range {
  Range {
    start: Position {
      line: range.start.line,
      character: range.start.character,
    },
    end: Position {
      line: range.end.line,
      character: range.end.character,
    },
  }
}

/// Check if a position is within a range.
fn position_in_range(position: Position, range: Range) -> bool {
  (position.line > range.start.line
    || (position.line == range.start.line && position.character >= range.start.character))
    && (position.line < range.end.line
      || (position.line == range.end.line && position.character <= range.end.character))
}

/// Try to narrow down to just the value part of a property.
fn narrow_to_value_range(
  _material: &Material,
  _key: &str,
  _value: &Value,
  _position: Position,
) -> Option<Range> {
  // For now, return None - we could implement more precise narrowing
  // by analyzing the text around the value
  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::{TextPosition, TextRange};
  use crate::parser::Located;

  fn make_range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> TextRange {
    TextRange {
      start: TextPosition {
        line: start_line,
        character: start_char,
      },
      end: TextPosition {
        line: end_line,
        character: end_char,
      },
    }
  }

  #[test]
  fn test_build_selection_ranges() {
    let matfile = MatFile {
      material: Material {
        range: make_range(0, 0, 5, 1),
        name: Some(Located::new("TestMat".to_string(), make_range(1, 4, 1, 20))),
        shading_model: Some(Located::new("lit".to_string(), make_range(2, 4, 2, 24))),
        requires: Located::new(vec![], make_range(0, 0, 0, 0)),
        parameters: vec![],
        other_properties: vec![],
      },
      shaders: vec![],
      errors: vec![],
    };

    // Cursor on "lit"
    let ranges = build_selection_ranges(
      &matfile,
      Position {
        line: 2,
        character: 18,
      },
    );

    assert!(!ranges.is_empty());
    // Should have at least: value range, property range, material block
    assert!(ranges.len() >= 2);
  }

  #[test]
  fn test_position_in_range() {
    let range = Range {
      start: Position {
        line: 0,
        character: 0,
      },
      end: Position {
        line: 0,
        character: 10,
      },
    };

    assert!(position_in_range(
      Position {
        line: 0,
        character: 5
      },
      range
    ));
    assert!(!position_in_range(
      Position {
        line: 0,
        character: 15
      },
      range
    ));
  }
}
