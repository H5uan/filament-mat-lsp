use std::collections::HashMap;
use std::sync::OnceLock;

/// Information about a function signature.
pub struct SignatureInfo {
  pub label: String,
  pub documentation: Option<String>,
  pub parameters: Vec<ParameterInfo>,
}

/// Information about a single parameter.
pub struct ParameterInfo {
  pub label: String,
  pub documentation: Option<String>,
}

static SIGNATURES: OnceLock<HashMap<&'static str, SignatureInfo>> = OnceLock::new();

fn init_signatures() -> HashMap<&'static str, SignatureInfo> {
  let mut map = HashMap::new();

  map.insert(
    "prepareMaterial",
    SignatureInfo {
      label: "prepareMaterial(inout MaterialInputs material)".to_string(),
      documentation: Some(
        "Must be called before modifying MaterialInputs in the fragment shader.".to_string(),
      ),
      parameters: vec![ParameterInfo {
        label: "material".to_string(),
        documentation: Some("The material inputs structure to initialize.".to_string()),
      }],
    },
  );

  map.insert(
    "getUV0",
    SignatureInfo {
      label: "getUV0() -> vec2".to_string(),
      documentation: Some(
        "Returns the first UV coordinate set (uv0) for the current fragment.".to_string(),
      ),
      parameters: vec![],
    },
  );

  map.insert(
    "getUV1",
    SignatureInfo {
      label: "getUV1() -> vec2".to_string(),
      documentation: Some(
        "Returns the second UV coordinate set (uv1) for the current fragment.".to_string(),
      ),
      parameters: vec![],
    },
  );

  map.insert(
    "getWorldPosition",
    SignatureInfo {
      label: "getWorldPosition() -> vec3".to_string(),
      documentation: Some("Returns the world-space position of the current fragment.".to_string()),
      parameters: vec![],
    },
  );

  map.insert(
    "getWorldNormal",
    SignatureInfo {
      label: "getWorldNormal() -> vec3".to_string(),
      documentation: Some("Returns the world-space normal of the current fragment.".to_string()),
      parameters: vec![],
    },
  );

  map.insert(
    "texture",
    SignatureInfo {
      label: "texture(sampler2D sampler, vec2 coord) -> vec4".to_string(),
      documentation: Some("GLSL built-in function for sampling textures.".to_string()),
      parameters: vec![
        ParameterInfo {
          label: "sampler".to_string(),
          documentation: Some("The sampler to read from.".to_string()),
        },
        ParameterInfo {
          label: "coord".to_string(),
          documentation: Some("Texture coordinates.".to_string()),
        },
      ],
    },
  );

  map
}

/// Get signature information for a function by name.
pub fn get_signature(name: &str) -> Option<&SignatureInfo> {
  SIGNATURES.get_or_init(init_signatures).get(name)
}

/// Find the function name at the given position in the text.
/// This looks backwards from the cursor to find the identifier before the opening parenthesis.
pub fn find_function_name(text: &str, cursor_offset: usize) -> Option<String> {
  if cursor_offset == 0 || cursor_offset > text.len() {
    return None;
  }

  // Look backwards from cursor to find the '('
  let before = &text[..cursor_offset];
  let paren_idx = before.rfind('(')?;

  // Extract the word before '('
  let before_paren = &before[..paren_idx];
  let word_start = before_paren
    .rfind(|c: char| !c.is_alphanumeric() && c != '_')
    .map(|i| i + 1)
    .unwrap_or(0);

  let name = before_paren[word_start..].trim();
  if name.is_empty() {
    return None;
  }

  Some(name.to_string())
}

/// Compute which parameter is active based on comma count.
pub fn compute_active_parameter(text: &str, cursor_offset: usize) -> u32 {
  if cursor_offset == 0 || cursor_offset > text.len() {
    return 0;
  }

  let before = &text[..cursor_offset];
  let paren_idx = match before.rfind('(') {
    Some(idx) => idx,
    None => return 0,
  };

  let inside_parens = &before[paren_idx + 1..];

  // Count commas that are not inside nested parentheses
  let mut comma_count = 0u32;
  let mut paren_depth = 0i32;

  for c in inside_parens.chars() {
    match c {
      '(' => paren_depth += 1,
      ')' => paren_depth -= 1,
      ',' if paren_depth == 0 => comma_count += 1,
      _ => {}
    }
  }

  comma_count
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_get_signature_prepare_material() {
    let sig = get_signature("prepareMaterial").unwrap();
    assert_eq!(sig.label, "prepareMaterial(inout MaterialInputs material)");
    assert_eq!(sig.parameters.len(), 1);
  }

  #[test]
  fn test_get_signature_get_uv0() {
    let sig = get_signature("getUV0").unwrap();
    assert_eq!(sig.label, "getUV0() -> vec2");
    assert!(sig.parameters.is_empty());
  }

  #[test]
  fn test_find_function_name() {
    let text = "prepareMaterial(material);";
    let offset = text.find('(').unwrap() + 1; // cursor inside parens
    assert_eq!(
      find_function_name(text, offset),
      Some("prepareMaterial".to_string())
    );
  }

  #[test]
  fn test_find_function_name_nested() {
    let text = "material.baseColor = texture(materialParams.color, getUV0());";
    // Cursor inside texture()
    let offset = text.find("materialParams").unwrap();
    assert_eq!(
      find_function_name(text, offset),
      Some("texture".to_string())
    );
  }

  #[test]
  fn test_compute_active_parameter() {
    let text = "texture(sampler, coord);";
    // Cursor after first comma
    let offset = text.find(',').unwrap() + 1;
    assert_eq!(compute_active_parameter(text, offset), 1);

    // Cursor before first comma
    let offset = text.find("sampler").unwrap() + 1;
    assert_eq!(compute_active_parameter(text, offset), 0);
  }
}
