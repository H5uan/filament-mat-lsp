use crate::parser::MatFile;
use lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, InlayHintTooltip, Position, Range};

/// Generate inlay hints for a .mat file within the given range.
pub fn generate_inlay_hints(matfile: &MatFile, range: Range) -> Vec<InlayHint> {
  let mut hints = Vec::new();

  // Type hints for parameters
  for param in &matfile.material.parameters {
    let param_range = Range {
      start: Position {
        line: param.range.start.line,
        character: param.range.start.character,
      },
      end: Position {
        line: param.range.end.line,
        character: param.range.end.character,
      },
    };

    // Only add hint if parameter range overlaps with requested range
    if ranges_overlap(range, param_range) {
      // Add type hint at the end of the parameter name
      // We approximate the position after "name : xxx"
      hints.push(InlayHint {
        position: Position {
          line: param.range.start.line,
          character: param.range.end.character,
        },
        label: InlayHintLabel::String(format!(": {}", param.param_type)),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: Some(InlayHintTooltip::String("Parameter type".to_string())),
        padding_left: Some(false),
        padding_right: Some(true),
        data: None,
      });
    }
  }

  // Type hints for properties that might benefit from them
  // (e.g., showing enum type for shadingModel)
  if let Some(sm) = &matfile.material.shading_model {
    let sm_range = Range {
      start: Position {
        line: sm.range.start.line,
        character: sm.range.start.character,
      },
      end: Position {
        line: sm.range.end.line,
        character: sm.range.end.character,
      },
    };

    if ranges_overlap(range, sm_range) {
      hints.push(InlayHint {
        position: Position {
          line: sm.range.end.line,
          character: sm.range.end.character,
        },
        label: InlayHintLabel::String(": ShadingModel".to_string()),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: Some(InlayHintTooltip::String("Shading model type".to_string())),
        padding_left: Some(false),
        padding_right: Some(true),
        data: None,
      });
    }
  }

  hints
}

/// Check if two ranges overlap.
fn ranges_overlap(a: Range, b: Range) -> bool {
  !(b.end.line < a.start.line
    || (b.end.line == a.start.line && b.end.character < a.start.character)
    || b.start.line > a.end.line
    || (b.start.line == a.end.line && b.start.character > a.end.character))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::{TextPosition, TextRange};
  use crate::parser::{Located, Material, Parameter};

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
  fn test_generate_inlay_hints() {
    let matfile = MatFile {
      material: Material {
        range: make_range(0, 0, 5, 1),
        name: None,
        shading_model: None,
        requires: Located::new(vec![], make_range(0, 0, 0, 0)),
        parameters: vec![Parameter {
          param_type: "float4".to_string(),
          name: "color".to_string(),
          other_fields: vec![],
          range: make_range(1, 4, 3, 5),
        }],
        other_properties: vec![],
      },
      shaders: vec![],
      errors: vec![],
    };

    let range = Range {
      start: Position {
        line: 0,
        character: 0,
      },
      end: Position {
        line: 10,
        character: 0,
      },
    };

    let hints = generate_inlay_hints(&matfile, range);

    assert_eq!(hints.len(), 1);
    assert!(matches!(&hints[0].label,
      InlayHintLabel::String(s) if s.contains("float4")
    ));
  }

  #[test]
  fn test_ranges_overlap() {
    let a = Range {
      start: Position {
        line: 0,
        character: 0,
      },
      end: Position {
        line: 5,
        character: 0,
      },
    };
    let b = Range {
      start: Position {
        line: 2,
        character: 0,
      },
      end: Position {
        line: 3,
        character: 0,
      },
    };
    assert!(ranges_overlap(a, b));

    let c = Range {
      start: Position {
        line: 10,
        character: 0,
      },
      end: Position {
        line: 15,
        character: 0,
      },
    };
    assert!(!ranges_overlap(a, c));
  }
}
