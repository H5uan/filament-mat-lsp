//! Lightweight GLSL symbol extraction from shader block code.
//!
//! This module performs simple text-based extraction of:
//! - Function declarations
//! - Uniform/varying declarations
//! - `materialParams.xxx` and `material.xxx` references

use crate::parser::ShaderBlock;

/// Kinds of symbols that can be extracted from shader code.
#[derive(Debug, Clone, PartialEq)]
pub enum ShaderSymbolKind {
  Function,
  Uniform,
  Varying,
  MaterialParamRef,
  MaterialFieldRef,
}

/// A symbol extracted from shader code, with its byte-offset range within `code`.
#[derive(Debug, Clone, PartialEq)]
pub struct ShaderSymbol {
  pub kind: ShaderSymbolKind,
  pub name: String,
  /// Byte offset of the symbol name within the shader code string.
  pub byte_start: usize,
  pub byte_end: usize,
}

/// Extract symbols from a shader block's code string.
pub fn extract_symbols(shader: &ShaderBlock) -> Vec<ShaderSymbol> {
  let mut symbols = Vec::new();
  let code = &shader.code;

  // 1. materialParams.xxx references
  for (idx, _) in code.match_indices("materialParams.") {
    let name_start = idx + "materialParams.".len();
    if let Some(name_end) = code[name_start..].find(|c: char| !c.is_alphanumeric() && c != '_') {
      let name = &code[name_start..name_start + name_end];
      if !name.is_empty() {
        symbols.push(ShaderSymbol {
          kind: ShaderSymbolKind::MaterialParamRef,
          name: name.to_string(),
          byte_start: name_start,
          byte_end: name_start + name_end,
        });
      }
    } else {
      // Runs to end of string
      let name = &code[name_start..];
      if !name.is_empty() {
        symbols.push(ShaderSymbol {
          kind: ShaderSymbolKind::MaterialParamRef,
          name: name.to_string(),
          byte_start: name_start,
          byte_end: code.len(),
        });
      }
    }
  }

  // 2. material.xxx field references (but not materialParams)
  for (idx, _) in code.match_indices("material.") {
    // Skip if this is actually "materialParams."
    if idx > 0 && code[..idx].ends_with("materialParams") {
      continue;
    }
    let name_start = idx + "material.".len();
    if let Some(name_end) = code[name_start..].find(|c: char| !c.is_alphanumeric() && c != '_') {
      let name = &code[name_start..name_start + name_end];
      if !name.is_empty() {
        symbols.push(ShaderSymbol {
          kind: ShaderSymbolKind::MaterialFieldRef,
          name: name.to_string(),
          byte_start: name_start,
          byte_end: name_start + name_end,
        });
      }
    }
  }

  // 3. Uniform declarations: "uniform type name;"
  for (idx, _) in code.match_indices("uniform ") {
    let after_uniform = &code[idx + 8..]; // skip "uniform "
    let rest = after_uniform.trim_start();
    let type_end = rest.find(|c: char| c.is_whitespace() || c == ';' || c == '[');
    if let Some(te) = type_end {
      let after_type = rest[te..].trim_start();
      if let Some(name_end) = after_type.find(|c: char| c.is_whitespace() || c == ';' || c == '=') {
        let name = &after_type[..name_end];
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
          // Find the actual byte offset of the name in the original code
          let name_in_code = format!(" {} ", name);
          if let Some(offset_in_rest) = after_type.find(&name_in_code) {
            let name_byte_start =
              idx + 8 + (rest.len() - after_uniform.trim_start().len()) + te + rest[te..].len()
                - after_type.len()
                + offset_in_rest
                + 1;
            symbols.push(ShaderSymbol {
              kind: ShaderSymbolKind::Uniform,
              name: name.to_string(),
              byte_start: name_byte_start,
              byte_end: name_byte_start + name.len(),
            });
          } else if let Some(offset_in_rest) = after_type.find(name) {
            let name_byte_start =
              idx + 8 + (rest.len() - after_uniform.trim_start().len()) + te + rest[te..].len()
                - after_type.len()
                + offset_in_rest;
            symbols.push(ShaderSymbol {
              kind: ShaderSymbolKind::Uniform,
              name: name.to_string(),
              byte_start: name_byte_start,
              byte_end: name_byte_start + name.len(),
            });
          }
        }
      }
    }
  }

  // 4. Varying declarations: "varying type name;"
  for (idx, _) in code.match_indices("varying ") {
    let after_varying = &code[idx + 8..]; // skip "varying "
    let rest = after_varying.trim_start();
    let type_end = rest.find(|c: char| c.is_whitespace() || c == ';');
    if let Some(te) = type_end {
      let after_type = rest[te..].trim_start();
      if let Some(name_end) = after_type.find(|c: char| c.is_whitespace() || c == ';' || c == '=') {
        let name = &after_type[..name_end];
        if !name.is_empty()
          && name.chars().all(|c| c.is_alphanumeric() || c == '_')
          && let Some(offset_in_rest) = after_type.find(name)
        {
          let name_byte_start =
            idx + 8 + (rest.len() - after_varying.trim_start().len()) + te + rest[te..].len()
              - after_type.len()
              + offset_in_rest;
          symbols.push(ShaderSymbol {
            kind: ShaderSymbolKind::Varying,
            name: name.to_string(),
            byte_start: name_byte_start,
            byte_end: name_byte_start + name.len(),
          });
        }
      }
    }
  }

  // 5. Function declarations: look for "type name(" at line start (after optional whitespace)
  let lines: Vec<&str> = code.lines().collect();
  let mut line_offset = 0usize;
  for line in &lines {
    let trimmed = line.trim();
    // Skip lines that start with keywords that aren't functions
    if !trimmed.starts_with("void ")
      && !trimmed.starts_with("float")
      && !trimmed.starts_with("int")
      && !trimmed.starts_with("bool")
      && !trimmed.starts_with("vec")
      && !trimmed.starts_with("mat")
      && !trimmed.starts_with("double")
      && !trimmed.starts_with("uint")
      && !trimmed.starts_with("ivec")
      && !trimmed.starts_with("uvec")
      && !trimmed.starts_with("bvec")
    {
      line_offset += line.len() + 1; // +1 for newline
      continue;
    }

    // Check if this line has a function declaration: "returnType name("
    if let Some(paren_idx) = trimmed.find('(') {
      // Find the last word before '('
      let before_paren = &trimmed[..paren_idx].trim();
      if let Some(space_idx) = before_paren.rfind(|c: char| c.is_whitespace()) {
        let name = before_paren[space_idx + 1..].trim();
        if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
          // Find byte offset of name within the line
          let line_trimmed = line.trim_start();
          let trimmed_len = line.len() - line_trimmed.len();
          if let Some(name_in_line) = line_trimmed.find(name) {
            let name_byte_start = line_offset + trimmed_len + name_in_line;
            symbols.push(ShaderSymbol {
              kind: ShaderSymbolKind::Function,
              name: name.to_string(),
              byte_start: name_byte_start,
              byte_end: name_byte_start + name.len(),
            });
          }
        }
      }
    }

    line_offset += line.len() + 1; // +1 for newline
  }

  symbols
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::{TextPosition, TextRange};
  use crate::parser::{ShaderBlock, ShaderBlockType};

  fn make_shader(code: &str) -> ShaderBlock {
    ShaderBlock {
      block_type: ShaderBlockType::Fragment,
      code: code.to_string(),
      range: TextRange {
        start: TextPosition {
          line: 0,
          character: 0,
        },
        end: TextPosition {
          line: 0,
          character: 0,
        },
      },
      symbols: vec![],
    }
  }

  fn test_extract_material_params() {
    let shader = make_shader(
      "material.baseColor = materialParams.color;\n    material.roughness = materialParams.roughness;",
    );
    let symbols = extract_symbols(&shader);
    let param_refs: Vec<&str> = symbols
      .iter()
      .filter(|s| s.kind == ShaderSymbolKind::MaterialParamRef)
      .map(|s| s.name.as_str())
      .collect();
    assert!(
      param_refs.contains(&"color"),
      "Should find materialParams.color"
    );
    assert!(
      param_refs.contains(&"roughness"),
      "Should find materialParams.roughness"
    );
  }

  #[test]
  fn test_extract_material_fields() {
    let shader = make_shader("material.baseColor = vec4(1.0);\n    material.roughness = 0.5;");
    let symbols = extract_symbols(&shader);
    let field_refs: Vec<&str> = symbols
      .iter()
      .filter(|s| s.kind == ShaderSymbolKind::MaterialFieldRef)
      .map(|s| s.name.as_str())
      .collect();
    assert!(
      field_refs.contains(&"baseColor"),
      "Should find material.baseColor"
    );
    assert!(
      field_refs.contains(&"roughness"),
      "Should find material.roughness"
    );
  }

  #[test]
  fn test_extract_uniform() {
    let shader = make_shader("uniform vec4 color;\nuniform sampler2d albedo;");
    let symbols = extract_symbols(&shader);
    let uniforms: Vec<&str> = symbols
      .iter()
      .filter(|s| s.kind == ShaderSymbolKind::Uniform)
      .map(|s| s.name.as_str())
      .collect();
    assert!(uniforms.contains(&"color"), "Should find uniform color");
    assert!(uniforms.contains(&"albedo"), "Should find uniform albedo");
  }

  #[test]
  fn test_extract_varying() {
    let shader = make_shader("varying vec2 uv0;\nvarying vec3 worldNormal;");
    let symbols = extract_symbols(&shader);
    let varyings: Vec<&str> = symbols
      .iter()
      .filter(|s| s.kind == ShaderSymbolKind::Varying)
      .map(|s| s.name.as_str())
      .collect();
    assert!(varyings.contains(&"uv0"), "Should find varying uv0");
    assert!(
      varyings.contains(&"worldNormal"),
      "Should find varying worldNormal"
    );
  }

  #[test]
  fn test_extract_function() {
    let shader = make_shader(
      "void material(inout MaterialInputs material) {\n  prepareMaterial(material);\n}\nfloat calcLighting(vec3 normal) {\n  return 1.0;\n}",
    );
    let symbols = extract_symbols(&shader);
    let funcs: Vec<&str> = symbols
      .iter()
      .filter(|s| s.kind == ShaderSymbolKind::Function)
      .map(|s| s.name.as_str())
      .collect();
    assert!(funcs.contains(&"material"), "Should find function material");
    assert!(
      funcs.contains(&"calcLighting"),
      "Should find function calcLighting"
    );
  }

  #[test]
  fn test_no_material_params_confusion() {
    // materialParams.xxx should NOT be extracted as material.xxx fields
    let shader = make_shader("materialParams.color;\n    material.color;");
    let symbols = extract_symbols(&shader);
    let field_refs: Vec<&str> = symbols
      .iter()
      .filter(|s| s.kind == ShaderSymbolKind::MaterialFieldRef)
      .map(|s| s.name.as_str())
      .collect();
    assert_eq!(
      field_refs,
      vec!["color"],
      "materialParams should not be extracted as material field"
    );
  }
}
