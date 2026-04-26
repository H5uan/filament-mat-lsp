use regex::Regex;
use std::sync::OnceLock;

use lsp_types::{Color, ColorInformation, ColorPresentation, Position, Range};

/// Regex patterns for matching color values in GLSL code.
static COLOR_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn get_color_patterns() -> &'static Vec<Regex> {
  COLOR_PATTERNS.get_or_init(|| {
    vec![
      // vec3(r, g, b) - supports negative values
      Regex::new(r"vec3\s*\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)").unwrap(),
      // vec4(r, g, b, a)
      Regex::new(
        r"vec4\s*\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)",
      )
      .unwrap(),
      // float3(r, g, b)
      Regex::new(r"float3\s*\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)").unwrap(),
      // float4(r, g, b, a)
      Regex::new(
        r"float4\s*\(\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*,\s*(-?[0-9.]+)\s*\)",
      )
      .unwrap(),
    ]
  })
}

/// Find all color values in the given text.
/// Returns a list of ColorInformation with their ranges.
pub fn find_colors(text: &str) -> Vec<ColorInformation> {
  let mut colors = Vec::new();
  let patterns = get_color_patterns();

  for (line_idx, line) in text.lines().enumerate() {
    for pattern in patterns {
      for cap in pattern.captures_iter(line) {
        let r: f32 = cap[1].parse().unwrap_or(0.0);
        let g: f32 = cap[2].parse().unwrap_or(0.0);
        let b: f32 = cap[3].parse().unwrap_or(0.0);
        let a: f32 = cap
          .get(4)
          .map_or(1.0, |m| m.as_str().parse().unwrap_or(1.0));

        // Clamp values to [0.0, 1.0]
        let r = r.clamp(0.0, 1.0);
        let g = g.clamp(0.0, 1.0);
        let b = b.clamp(0.0, 1.0);
        let a = a.clamp(0.0, 1.0);

        let start_char = cap.get(0).unwrap().start() as u32;
        let end_char = cap.get(0).unwrap().end() as u32;

        colors.push(ColorInformation {
          range: Range {
            start: Position {
              line: line_idx as u32,
              character: start_char,
            },
            end: Position {
              line: line_idx as u32,
              character: end_char,
            },
          },
          color: Color {
            red: r,
            green: g,
            blue: b,
            alpha: a,
          },
        });
      }
    }
  }

  colors
}

/// Generate color presentations for a color at a given range.
/// Returns different ways to represent the color in code.
pub fn get_color_presentations(_color: Color, _range: Range) -> Vec<ColorPresentation> {
  // For now, return a simple presentation
  // In the future, this could offer different formats (vec3, vec4, hex, etc.)
  vec![
    ColorPresentation {
      label: format!(
        "vec4({}, {}, {}, {})",
        _color.red, _color.green, _color.blue, _color.alpha
      ),
      text_edit: Some(lsp_types::TextEdit {
        range: _range,
        new_text: format!(
          "vec4({:.2}, {:.2}, {:.2}, {:.2})",
          _color.red, _color.green, _color.blue, _color.alpha
        ),
      }),
      additional_text_edits: None,
    },
    ColorPresentation {
      label: format!("vec3({}, {}, {})", _color.red, _color.green, _color.blue),
      text_edit: Some(lsp_types::TextEdit {
        range: _range,
        new_text: format!(
          "vec3({:.2}, {:.2}, {:.2})",
          _color.red, _color.green, _color.blue
        ),
      }),
      additional_text_edits: None,
    },
  ]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_find_colors_vec3() {
    let text = "vec3(1.0, 0.0, 0.0)";
    let colors = find_colors(text);
    assert_eq!(colors.len(), 1);
    assert!((colors[0].color.red - 1.0).abs() < 0.01);
    assert!((colors[0].color.green - 0.0).abs() < 0.01);
    assert!((colors[0].color.blue - 0.0).abs() < 0.01);
    assert!((colors[0].color.alpha - 1.0).abs() < 0.01);
  }

  #[test]
  fn test_find_colors_vec4() {
    let text = "vec4(0.0, 1.0, 0.0, 0.5)";
    let colors = find_colors(text);
    assert_eq!(colors.len(), 1);
    assert!((colors[0].color.red - 0.0).abs() < 0.01);
    assert!((colors[0].color.green - 1.0).abs() < 0.01);
    assert!((colors[0].color.blue - 0.0).abs() < 0.01);
    assert!((colors[0].color.alpha - 0.5).abs() < 0.01);
  }

  #[test]
  fn test_find_colors_multiple() {
    let text = r#"vec3(1.0, 0.0, 0.0)
vec3(0.0, 1.0, 0.0)
vec4(0.0, 0.0, 1.0, 0.8)"#;
    let colors = find_colors(text);
    assert_eq!(colors.len(), 3);
  }

  #[test]
  fn test_find_colors_clamping() {
    let text = "vec3(1.5, -0.2, 2.0)";
    let colors = find_colors(text);
    assert_eq!(colors.len(), 1);
    assert!((colors[0].color.red - 1.0).abs() < 0.01);
    assert!((colors[0].color.green - 0.0).abs() < 0.01);
    assert!((colors[0].color.blue - 1.0).abs() < 0.01);
  }
}
