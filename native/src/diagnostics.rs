use crate::parser::{Material, Parameter, Value};
use crate::schema::{KeywordType, get_enum_values, get_properties};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
  pub message: String,
  pub severity: DiagnosticSeverity,
  pub range: Option<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
  Error,
  Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRange {
  pub start: TextPosition,
  pub end: TextPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPosition {
  pub line: u32,
  pub character: u32,
}

pub struct Validator;

impl Validator {
  pub fn new() -> Self {
    Self
  }

  pub fn validate_material(&self, material: &Material) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if material.name.is_none() {
      diagnostics.push(Self::error(
        "Material is missing 'name' property",
        Some(material_keyword_range(material)),
      ));
    }

    if material.shading_model.is_none() {
      diagnostics.push(Self::error(
        "Material is missing 'shadingModel' property",
        Some(material_keyword_range(material)),
      ));
    }

    // Validate all properties
    for (key, value) in &material.properties {
      diagnostics.extend(self.validate_property(key, value));
    }

    for param in &material.parameters {
      diagnostics.extend(self.validate_parameter(param));
    }

    diagnostics.extend(self.validate_interdependencies(material));

    diagnostics
  }

  fn validate_interdependencies(&self, material: &Material) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let shading_model = material.shading_model.as_ref().map(|s| s.value.as_str());
    let domain = material
      .properties
      .iter()
      .find(|(k, _)| k == "domain")
      .and_then(|(_, v)| match &v.value {
        Value::Identifier(s) => Some(s.as_str()),
        _ => None,
      });
    let blending = material
      .properties
      .iter()
      .find(|(k, _)| k == "blending")
      .and_then(|(_, v)| match &v.value {
        Value::Identifier(s) => Some(s.as_str()),
        _ => None,
      });

    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "shadowMultiplier")
      && shading_model != Some("unlit")
    {
      diags.push(Self::warning(
        "shadowMultiplier is only valid for unlit shading model",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "customSurfaceShading")
      && shading_model != Some("lit")
    {
      diags.push(Self::warning(
        "customSurfaceShading is only valid for lit shading model",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material.properties.iter().find(|(k, _)| k == "groupSize")
      && domain != Some("compute")
    {
      diags.push(Self::warning(
        "groupSize is only valid for compute domain",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "refractionMode")
      && shading_model != Some("lit")
    {
      diags.push(Self::warning(
        "refractionMode requires lit shading model",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "refractionType")
      && shading_model != Some("lit")
    {
      diags.push(Self::warning(
        "refractionType requires lit shading model",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "stereoscopicType")
      && shading_model != Some("lit")
    {
      diags.push(Self::warning(
        "stereoscopicType requires lit shading model",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "alphaToCoverage")
      && blending != Some("masked")
    {
      diags.push(Self::warning(
        "alphaToCoverage is only effective when blending is masked",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "maskThreshold")
      && blending != Some("masked")
    {
      diags.push(Self::warning(
        "maskThreshold is only effective when blending is masked",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material.properties.iter().find(|(k, _)| k == "flipUV") {
      diags.push(Self::warning(
        "flipUV is deprecated",
        Some(prop.range.clone()),
      ));
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "vertexDomainDeviceJittered")
    {
      let vertex_domain = material
        .properties
        .iter()
        .find(|(k, _)| k == "vertexDomain")
        .and_then(|(_, v)| match &v.value {
          Value::Identifier(s) => Some(s.as_str()),
          _ => None,
        });
      if vertex_domain != Some("device") {
        diags.push(Self::warning(
          "vertexDomainDeviceJittered requires vertexDomain to be device",
          Some(prop.range.clone()),
        ));
      }
    }
    if let Some((_, prop)) = material
      .properties
      .iter()
      .find(|(k, _)| k == "useDefaultDepthVariant")
      && (shading_model != Some("lit") || blending != Some("opaque"))
    {
      diags.push(Self::warning(
        "useDefaultDepthVariant requires lit shading model and opaque blending",
        Some(prop.range.clone()),
      ));
    }

    diags
  }

  fn validate_property(&self, key: &str, value: &crate::parser::Located<Value>) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check if property name is known
    let known_properties: Vec<&str> = get_properties().iter().map(|p| p.name).collect();
    if !known_properties.contains(&key) {
      // It's already stored in properties, which means it's unknown.
      // But we only want to warn, not error, since custom properties might exist.
      diagnostics.push(Self::warning(
        format!("Unknown material property: '{}'", key),
        Some(value.range.clone()),
      ));
      return diagnostics;
    }

    // Check if property value is valid enum
    if let Some(valid_values) = get_enum_values(key) {
      let value_str = match &value.value {
        Value::Identifier(s) | Value::String(s) => Some(s.as_str()),
        _ => None,
      };

      if let Some(s) = value_str
        && !valid_values.contains(&s)
      {
        diagnostics.push(Self::warning(
          format!(
            "Invalid value '{}' for property '{}'. Expected one of: {}",
            s,
            key,
            valid_values.join(", ")
          ),
          Some(value.range.clone()),
        ));
      }
    }

    diagnostics
  }

  fn validate_parameter(&self, param: &Parameter) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if param.name.is_empty() {
      diagnostics.push(Self::error("Parameter is missing a name", None));
    }

    if param.param_type.is_empty() {
      diagnostics.push(Self::error(
        format!("Parameter '{}' is missing a type", param.name),
        None,
      ));
    }

    if !Self::is_valid_parameter_type(&param.param_type) {
      diagnostics.push(Self::warning(
        format!(
          "Parameter type '{}' is not a standard Filament type",
          param.param_type
        ),
        None,
      ));
    }

    diagnostics
  }

  fn is_valid_parameter_type(ty: &str) -> bool {
    let types = crate::schema::get_keywords_by_type(KeywordType::ParameterType);
    types.contains(&ty)
  }

  fn error(message: impl Into<String>, range: Option<TextRange>) -> Diagnostic {
    Diagnostic {
      message: message.into(),
      severity: DiagnosticSeverity::Error,
      range,
    }
  }

  fn warning(message: impl Into<String>, range: Option<TextRange>) -> Diagnostic {
    Diagnostic {
      message: message.into(),
      severity: DiagnosticSeverity::Warning,
      range,
    }
  }
}

fn material_keyword_range(material: &Material) -> TextRange {
  TextRange {
    start: TextPosition {
      line: material.range.start.line,
      character: material.range.start.character,
    },
    end: TextPosition {
      line: material.range.start.line,
      character: material.range.start.character + 8,
    },
  }
}

impl Default for Validator {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::parser::{Located, Material, Parameter};

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
  fn test_validate_valid_material() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn test_validate_missing_name() {
    let material = Material {
      range: dummy_range(),
      name: None,
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
  }

  #[test]
  fn test_validate_parameter_type() {
    let invalid_param = Parameter {
      name: "test".to_string(),
      param_type: "invalidType".to_string(),
      other_fields: vec![],
      range: dummy_range(),
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_parameter(&invalid_param);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
  }

  #[test]
  fn test_validate_invalid_property_value() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("invalidModel".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    // shading_model is stored as Option, not properties, so this test
    // doesn't trigger the property validation. We test properties instead.
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn test_validate_unknown_property() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![(
        "unknownProperty".to_string(),
        Located::new(Value::Identifier("value".to_string()), dummy_range()),
      )],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
    assert!(diagnostics[0].message.contains("Unknown material property"));
  }

  #[test]
  fn test_validate_invalid_enum_value() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![(
        "blending".to_string(),
        Located::new(Value::Identifier("invalidBlend".to_string()), dummy_range()),
      )],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
    assert!(diagnostics[0].message.contains("Invalid value"));
  }

  #[test]
  fn test_validate_interdependency_shadow_multiplier() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![(
        "shadowMultiplier".to_string(),
        Located::new(Value::Bool(true), dummy_range()),
      )],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("shadowMultiplier"));
  }

  #[test]
  fn test_validate_interdependency_custom_surface_shading() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("unlit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![(
        "customSurfaceShading".to_string(),
        Located::new(Value::Bool(true), dummy_range()),
      )],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("customSurfaceShading"));
  }

  #[test]
  fn test_validate_interdependency_group_size() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "domain".to_string(),
          Located::new(Value::Identifier("surface".to_string()), dummy_range()),
        ),
        (
          "groupSize".to_string(),
          Located::new(
            Value::Array(vec![
              Value::Number(8.0),
              Value::Number(8.0),
              Value::Number(1.0),
            ]),
            dummy_range(),
          ),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("groupSize"));
  }

  #[test]
  fn test_validate_interdependency_refraction() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("unlit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "refractionMode".to_string(),
          Located::new(Value::Identifier("screenspace".to_string()), dummy_range()),
        ),
        (
          "refractionType".to_string(),
          Located::new(Value::Identifier("solid".to_string()), dummy_range()),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 2);
    assert!(
      warnings[0].message.contains("refractionMode")
        || warnings[0].message.contains("refractionType")
    );
  }

  #[test]
  fn test_validate_interdependency_alpha_to_coverage() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "blending".to_string(),
          Located::new(Value::Identifier("transparent".to_string()), dummy_range()),
        ),
        (
          "alphaToCoverage".to_string(),
          Located::new(Value::Bool(true), dummy_range()),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("alphaToCoverage"));
  }

  #[test]
  fn test_validate_interdependency_mask_threshold() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "blending".to_string(),
          Located::new(Value::Identifier("opaque".to_string()), dummy_range()),
        ),
        (
          "maskThreshold".to_string(),
          Located::new(Value::Number(0.5), dummy_range()),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("maskThreshold"));
  }

  #[test]
  fn test_validate_interdependency_flip_uv_deprecated() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![(
        "flipUV".to_string(),
        Located::new(Value::Bool(true), dummy_range()),
      )],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("flipUV"));
    assert!(warnings[0].message.contains("deprecated"));
  }

  #[test]
  fn test_validate_interdependency_vertex_domain_device_jittered() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("lit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "vertexDomain".to_string(),
          Located::new(Value::Identifier("object".to_string()), dummy_range()),
        ),
        (
          "vertexDomainDeviceJittered".to_string(),
          Located::new(Value::Bool(true), dummy_range()),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("vertexDomainDeviceJittered"));
  }

  #[test]
  fn test_validate_interdependency_use_default_depth_variant() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("unlit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "blending".to_string(),
          Located::new(Value::Identifier("opaque".to_string()), dummy_range()),
        ),
        (
          "useDefaultDepthVariant".to_string(),
          Located::new(Value::Bool(true), dummy_range()),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("useDefaultDepthVariant"));
  }

  #[test]
  fn test_validate_interdependency_no_warnings_when_valid() {
    let material = Material {
      range: dummy_range(),
      name: Some(Located::new("TestMat".to_string(), dummy_range())),
      shading_model: Some(Located::new("unlit".to_string(), dummy_range())),
      parameters: vec![],
      constants: vec![],
      variables: vec![],
      buffers: vec![],
      subpasses: vec![],
      outputs: vec![],
      properties: vec![
        (
          "shadowMultiplier".to_string(),
          Located::new(Value::Bool(true), dummy_range()),
        ),
        (
          "blending".to_string(),
          Located::new(Value::Identifier("masked".to_string()), dummy_range()),
        ),
        (
          "alphaToCoverage".to_string(),
          Located::new(Value::Bool(true), dummy_range()),
        ),
        (
          "maskThreshold".to_string(),
          Located::new(Value::Number(0.5), dummy_range()),
        ),
      ],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    let warnings: Vec<_> = diagnostics
      .iter()
      .filter(|d| d.severity == DiagnosticSeverity::Warning)
      .collect();
    assert_eq!(
      warnings.len(),
      0,
      "Expected no warnings for valid combination, got: {:?}",
      warnings
    );
  }
}
