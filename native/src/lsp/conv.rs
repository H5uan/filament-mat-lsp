use lsp_types::{self, CompletionItemKind, DiagnosticSeverity, Position, Range};

use filament_mat_lsp::completion::{CompletionItem, CompletionItemKind as InternalCompletionItemKind};
use filament_mat_lsp::diagnostics::{Diagnostic, DiagnosticSeverity as InternalDiagnosticSeverity, TextRange};

pub fn to_lsp_range(range: &TextRange) -> Range {
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

pub fn to_lsp_completion_item(item: CompletionItem) -> lsp_types::CompletionItem {
    lsp_types::CompletionItem {
        label: item.label,
        kind: Some(match item.kind {
            InternalCompletionItemKind::Property => CompletionItemKind::PROPERTY,
            InternalCompletionItemKind::EnumValue => CompletionItemKind::ENUM_MEMBER,
            InternalCompletionItemKind::Type => CompletionItemKind::TYPE_PARAMETER,
        }),
        documentation: item.documentation.map(lsp_types::Documentation::String),
        ..Default::default()
    }
}

pub fn to_lsp_diagnostic(diagnostic: Diagnostic) -> lsp_types::Diagnostic {
    let range = diagnostic.range.map_or(
        Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        },
        |r| Range {
            start: Position {
                line: r.start.line,
                character: r.start.character,
            },
            end: Position {
                line: r.end.line,
                character: r.end.character,
            },
        },
    );

    lsp_types::Diagnostic {
        range,
        severity: Some(match diagnostic.severity {
            InternalDiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            InternalDiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
        }),
        code: None,
        code_description: None,
        source: Some("filament-mat".to_string()),
        message: diagnostic.message,
        related_information: None,
        tags: None,
        data: None,
    }
}
