use crate::parser::{Material, Parameter};

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
      diagnostics.push(Self::error("Material is missing 'name' property"));
    }

    if material.shading_model.is_none() {
      diagnostics.push(Self::error("Material is missing 'shadingModel' property"));
    }

    for param in &material.parameters {
      diagnostics.extend(self.validate_parameter(param));
    }

    diagnostics
  }

  fn validate_parameter(&self, param: &Parameter) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if param.name.is_empty() {
      diagnostics.push(Self::error("Parameter is missing a name"));
    }

    if param.param_type.is_empty() {
      diagnostics.push(Self::error(format!(
        "Parameter '{}' is missing a type",
        param.name
      )));
    }

    if !Self::is_valid_parameter_type(&param.param_type) {
      diagnostics.push(Self::warning(format!(
        "Parameter type '{}' is not a standard Filament type",
        param.param_type
      )));
    }

    diagnostics
  }

  fn is_valid_parameter_type(ty: &str) -> bool {
    matches!(
      ty,
      "bool"
        | "bool2"
        | "bool3"
        | "bool4"
        | "int"
        | "int2"
        | "int3"
        | "int4"
        | "uint"
        | "uint2"
        | "uint3"
        | "uint4"
        | "float"
        | "float2"
        | "float3"
        | "float4"
        | "mat3"
        | "mat4"
        | "sampler2d"
        | "sampler3d"
        | "samplerCubemap"
        | "samplerExternal"
    )
  }

  fn error(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
      message: message.into(),
      severity: DiagnosticSeverity::Error,
      range: None,
    }
  }

  fn warning(message: impl Into<String>) -> Diagnostic {
    Diagnostic {
      message: message.into(),
      severity: DiagnosticSeverity::Warning,
      range: None,
    }
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
  use crate::parser::{Material, Parameter};

  #[test]
  fn test_validate_valid_material() {
    let material = Material {
      name: Some("TestMat".to_string()),
      shading_model: Some("lit".to_string()),
      parameters: vec![],
      requires: vec![],
      other_properties: vec![],
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_material(&material);
    assert!(diagnostics.is_empty());
  }

  #[test]
  fn test_validate_missing_name() {
    let material = Material {
      name: None,
      shading_model: Some("lit".to_string()),
      parameters: vec![],
      requires: vec![],
      other_properties: vec![],
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
    };

    let validator = Validator::new();
    let diagnostics = validator.validate_parameter(&invalid_param);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Warning);
  }
}
