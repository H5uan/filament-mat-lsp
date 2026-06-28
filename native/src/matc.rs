use std::path::Path;
use std::process::Command;

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, TextPosition, TextRange};

pub struct MatcConfig {
  pub matc_path: String,
  pub platform: String,
  pub api: String,
}

pub struct MatcDiagnostic {
  pub severity: DiagnosticSeverity,
  pub message: String,
  pub line: Option<u32>,
  pub file: Option<String>,
}

pub fn run_matc(config: &MatcConfig, file_path: &Path) -> Result<Vec<MatcDiagnostic>, String> {
  let output = Command::new(&config.matc_path)
    .args(["-o", "NUL", "-p", &config.platform, "-a", &config.api])
    .arg(file_path)
    .output()
    .map_err(|e| format!("Failed to spawn matc: {}", e))?;

  let stderr = String::from_utf8_lossy(&output.stderr);
  parse_matc_output(&stderr)
}

fn parse_matc_output(stderr: &str) -> Result<Vec<MatcDiagnostic>, String> {
  let mut diags = Vec::new();
  for line in stderr.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }

    if let Some(rest) = line.strip_prefix("ERROR: ") {
      diags.push(parse_matc_line(rest, DiagnosticSeverity::Error));
    } else if let Some(rest) = line.strip_prefix("WARNING: ") {
      diags.push(parse_matc_line(rest, DiagnosticSeverity::Warning));
    } else if line.contains("error") || line.contains("Error") {
      diags.push(MatcDiagnostic {
        severity: DiagnosticSeverity::Error,
        message: line.to_string(),
        line: None,
        file: None,
      });
    } else if line.contains("warning") || line.contains("Warning") {
      diags.push(MatcDiagnostic {
        severity: DiagnosticSeverity::Warning,
        message: line.to_string(),
        line: None,
        file: None,
      });
    } else {
      // Treat any other stderr output as error (compilation failure)
      diags.push(MatcDiagnostic {
        severity: DiagnosticSeverity::Error,
        message: line.to_string(),
        line: None,
        file: None,
      });
    }
  }
  Ok(diags)
}

fn parse_matc_line(rest: &str, severity: DiagnosticSeverity) -> MatcDiagnostic {
  // Try to parse "file:line: message" or "file:line:message"
  if let Some((file_part, msg)) = rest.split_once(": ") {
    if let Some((file, line_str)) = file_part.rsplit_once(':')
      && let Ok(line_num) = line_str.parse::<u32>()
    {
      return MatcDiagnostic {
        severity,
        message: msg.to_string(),
        line: Some(line_num),
        file: Some(file.to_string()),
      };
    }
    // No line number found, but has "file: message" format
    return MatcDiagnostic {
      severity,
      message: msg.to_string(),
      line: None,
      file: Some(file_part.to_string()),
    };
  }

  // Fallback: no structured format
  MatcDiagnostic {
    severity,
    message: rest.to_string(),
    line: None,
    file: None,
  }
}

pub fn matc_diagnostic_to_lsp(diag: MatcDiagnostic) -> Diagnostic {
  let range = diag
    .line
    .map(|line| TextRange {
      start: TextPosition {
        line: line.saturating_sub(1),
        character: 0,
      },
      end: TextPosition {
        line: line.saturating_sub(1),
        character: 999,
      },
    })
    .unwrap_or_else(|| TextRange {
      start: TextPosition {
        line: 0,
        character: 0,
      },
      end: TextPosition {
        line: 0,
        character: 0,
      },
    });

  Diagnostic {
    message: diag.message,
    severity: diag.severity,
    range: Some(range),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_matc_error_with_line() {
    let stderr = "ERROR: test.mat:15: 'shadingModel' : unknown property";
    let diags = parse_matc_output(stderr).unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
    assert_eq!(diags[0].line, Some(15));
    assert_eq!(diags[0].message, "'shadingModel' : unknown property");
    assert_eq!(diags[0].file.as_ref().unwrap(), "test.mat");
  }

  #[test]
  fn test_parse_matc_warning_with_line() {
    let stderr =
      "WARNING: test.mat:23: 'maskThreshold' has no effect when blending is not 'masked'";
    let diags = parse_matc_output(stderr).unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagnosticSeverity::Warning);
    assert_eq!(diags[0].line, Some(23));
    assert_eq!(
      diags[0].message,
      "'maskThreshold' has no effect when blending is not 'masked'"
    );
  }

  #[test]
  fn test_parse_matc_error_without_line() {
    let stderr = "ERROR: matc failed to compile";
    let diags = parse_matc_output(stderr).unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
    assert_eq!(diags[0].line, None);
    assert_eq!(diags[0].message, "matc failed to compile");
  }

  #[test]
  fn test_parse_matc_generic_error() {
    let stderr = "Could not compile material test.mat";
    let diags = parse_matc_output(stderr).unwrap();
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].severity, DiagnosticSeverity::Error);
    assert_eq!(diags[0].message, "Could not compile material test.mat");
  }

  #[test]
  fn test_parse_matc_empty_stderr() {
    let diags = parse_matc_output("").unwrap();
    assert!(diags.is_empty());
  }

  #[test]
  fn test_matc_diagnostic_to_lsp_with_line() {
    let diag = MatcDiagnostic {
      severity: DiagnosticSeverity::Error,
      message: "test error".to_string(),
      line: Some(5),
      file: None,
    };
    let lsp = matc_diagnostic_to_lsp(diag);
    assert_eq!(lsp.severity, DiagnosticSeverity::Error);
    assert_eq!(lsp.message, "test error");
    assert!(lsp.range.is_some());
    let range = lsp.range.unwrap();
    assert_eq!(range.start.line, 4); // 0-indexed
    assert_eq!(range.start.character, 0);
  }

  #[test]
  fn test_matc_diagnostic_to_lsp_without_line() {
    let diag = MatcDiagnostic {
      severity: DiagnosticSeverity::Warning,
      message: "test warning".to_string(),
      line: None,
      file: None,
    };
    let lsp = matc_diagnostic_to_lsp(diag);
    assert_eq!(lsp.severity, DiagnosticSeverity::Warning);
    assert_eq!(lsp.message, "test warning");
    assert!(lsp.range.is_some());
  }
}
