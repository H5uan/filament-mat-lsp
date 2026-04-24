use lsp_server::{Message, Notification, Request, Response};
use lsp_types::*;

use filament_mat_lsp::completion::{CompletionContext as InternalCompletionContext, CompletionEngine};
use filament_mat_lsp::diagnostics::Validator;
use filament_mat_lsp::hover::HoverEngine;
use filament_mat_lsp::lexer::JsonishLexer;
use filament_mat_lsp::parser::Parser;

use super::conv;
use super::server::ServerState;

pub fn handle_request(
    server: &mut ServerState,
    req: Request,
    sender: &crossbeam_channel::Sender<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
    match req.method.as_str() {
        "textDocument/completion" => {
            let params: CompletionParams = serde_json::from_value(req.params)?;
            let completions = handle_completion(server, params);
            let result = serde_json::to_value(&completions)?;
            let resp = Response {
                id: req.id,
                result: Some(result),
                error: None,
            };
            sender.send(resp.into())?;
        }
        "textDocument/hover" => {
            let params: HoverParams = serde_json::from_value(req.params)?;
            let hover = handle_hover(server, params);
            let result = serde_json::to_value(&hover)?;
            let resp = Response {
                id: req.id,
                result: Some(result),
                error: None,
            };
            sender.send(resp.into())?;
        }
        "textDocument/definition" => {
            let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
            let locations = handle_definition(server, params);
            let result = serde_json::to_value(&locations)?;
            let resp = Response {
                id: req.id,
                result: Some(result),
                error: None,
            };
            sender.send(resp.into())?;
        }
        "textDocument/documentSymbol" => {
            let params: DocumentSymbolParams = serde_json::from_value(req.params)?;
            let symbols = handle_document_symbol(server, params);
            let result = serde_json::to_value(&symbols)?;
            let resp = Response {
                id: req.id,
                result: Some(result),
                error: None,
            };
            sender.send(resp.into())?;
        }
        "textDocument/diagnostic" => {
            let params: DocumentDiagnosticParams = serde_json::from_value(req.params)?;
            let diagnostics = handle_diagnostic(server, params);
            let result = serde_json::to_value(&diagnostics)?;
            let resp = Response {
                id: req.id,
                result: Some(result),
                error: None,
            };
            sender.send(resp.into())?;
        }
        _ => {
            // Method not found - return error
            let resp = Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("Method {} not found", req.method),
            );
            sender.send(resp.into())?;
        }
    }
    Ok(())
}

pub fn handle_notification(
    server: &mut ServerState,
    not: Notification,
) -> Result<(), Box<dyn std::error::Error>> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            let text = params.text_document.text;
            let version = params.text_document.version;
            server.insert_document(
                uri.clone(), super::server::Document { text: text.clone(), version });
            // Trigger diagnostics
            let diagnostics = compute_diagnostics(&text);
            publish_diagnostics(&uri, diagnostics, server)?;
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
            let uri = params.text_document.uri;
            let version = params.text_document.version;
            for change in params.content_changes {
                server.apply_change(&uri, change, version);
            }
            if let Some(doc) = server.get_document(&uri) {
                let diagnostics = compute_diagnostics(&doc.text);
                publish_diagnostics(&uri, diagnostics, server)?;
            }
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
            server.remove_document(&params.text_document.uri);
        }
        _ => {}
    }
    Ok(())
}

fn handle_completion(
    _server: &ServerState,
    params: CompletionParams,
) -> CompletionList {
    let engine = CompletionEngine::new();
    
    // Simple context detection based on trigger character
    let trigger = params.context.as_ref().and_then(|c| c.trigger_character.as_ref());
    
    let context = match trigger.map(|s| s.as_str()) {
        Some(":") => {
            // Check what's before the colon to determine context
            InternalCompletionContext::MaterialBlock
        }
        _ => InternalCompletionContext::MaterialBlock,
    };
    
    let items = engine.get_completions(context);
    let completion_items: Vec<CompletionItem> = items.into_iter().map(conv::to_lsp_completion_item).collect();
    
    CompletionList {
        is_incomplete: false,
        items: completion_items,
    }
}

fn handle_hover(
    server: &ServerState,
    params: HoverParams,
) -> Option<Hover> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;
    
    let doc = server.get_document(uri)?;
    let word = extract_word_at_position(&doc.text, position)?;
    
    let engine = HoverEngine::new();
    engine.get_hover(&word).map(|doc| Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.clone(),
        }),
        range: None,
    })
}

fn extract_word_at_position(text: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let line = *lines.get(position.line as usize)?;
    
    let mut start = position.character as usize;
    let mut end = position.character as usize;
    
    // Find word boundaries
    while start > 0 && is_word_char(line.chars().nth(start - 1)?) {
        start -= 1;
    }
    while end < line.len() && is_word_char(line.chars().nth(end)?) {
        end += 1;
    }
    
    if start < end {
        Some(line[start..end].to_string())
    } else {
        None
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn handle_definition(
    server: &ServerState,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;
    
    let doc = server.get_document(uri)?;
    let word = extract_word_at_position(&doc.text, position)?;
    
    let mut lexer = JsonishLexer::new(&doc.text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    
    let material = parser.parse_material()?;
    
    // Check if word is a material property key
    let mut locations = Vec::new();
    
    // Check name property
    if word == "name" {
        if let Some(name) = &material.name {
            locations.push(Location {
                uri: uri.clone(),
                range: conv::to_lsp_range(&name.range),
            });
        }
    }
    
    // Check shadingModel property
    if word == "shadingModel" {
        if let Some(sm) = &material.shading_model {
            locations.push(Location {
                uri: uri.clone(),
                range: conv::to_lsp_range(&sm.range),
            });
        }
    }
    
    // Check parameters
    for param in &material.parameters {
        if param.name == word {
            // For parameters, we don't have position info yet, so return None
            // This is a simplified implementation
        }
    }
    
    if locations.is_empty() {
        None
    } else {
        Some(GotoDefinitionResponse::Array(locations))
    }
}

fn handle_diagnostic(
    server: &ServerState,
    params: DocumentDiagnosticParams,
) -> DocumentDiagnosticReportResult {
    let uri = params.text_document.uri;
    
    let diagnostics = if let Some(doc) = server.get_document(&uri) {
        compute_diagnostics(&doc.text)
    } else {
        vec![]
    };
    
    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
        related_documents: None,
        full_document_diagnostic_report: FullDocumentDiagnosticReport {
            result_id: None,
            items: diagnostics,
        },
    }))
}

fn handle_document_symbol(
    server: &ServerState,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = &params.text_document.uri;
    let doc = server.get_document(uri)?;
    
    let mut lexer = JsonishLexer::new(&doc.text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    
    let material = parser.parse_material()?;
    
    let mut symbols = Vec::new();
    
    // Material root symbol
    let material_symbol = DocumentSymbol {
        name: material.name.as_ref()
            .map(|n| n.value.clone())
            .unwrap_or_else(|| "Material".to_string()),
        detail: material.shading_model.as_ref()
            .map(|s| format!("shadingModel: {}", s.value)),
        kind: SymbolKind::OBJECT,
        tags: None,
        deprecated: None,
        range: conv::to_lsp_range(&material.range),
        selection_range: conv::to_lsp_range(&material.range),
        children: Some(Vec::new()),
    };
    
    symbols.push(material_symbol);
    
    // Add parameter symbols
    for param in &material.parameters {
        let param_symbol = DocumentSymbol {
            name: param.name.clone(),
            detail: Some(format!("type: {}", param.param_type)),
            kind: SymbolKind::PROPERTY,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            selection_range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            children: None,
        };
        symbols.push(param_symbol);
    }
    
    Some(DocumentSymbolResponse::Nested(symbols))
}

fn compute_diagnostics(text: &str) -> Vec<lsp_types::Diagnostic> {
    let mut lexer = JsonishLexer::new(text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    
    let mut diagnostics = Vec::new();
    
    if let Some(material) = parser.parse_material() {
        let validator = Validator::new();
        let internal_diagnostics = validator.validate_material(&material);
        diagnostics = internal_diagnostics
            .into_iter()
            .map(conv::to_lsp_diagnostic)
            .collect();
    }
    
    diagnostics
}

fn publish_diagnostics(
    uri: &Url,
    diagnostics: Vec<lsp_types::Diagnostic>,
    server: &ServerState,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let notification = Notification {
        method: "textDocument/publishDiagnostics".to_string(),
        params: serde_json::to_value(params)?,
    };
    server.send(notification.into())?;
    Ok(())
}
