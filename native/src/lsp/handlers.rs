use lsp_server::{Message, Notification, Request, Response};
use lsp_types::*;

use filament_mat_lsp::completion::{
  CompletionContext as InternalCompletionContext, CompletionEngine,
};
use filament_mat_lsp::diagnostics::Validator;
use filament_mat_lsp::hover::HoverEngine;

use super::conv;
use super::server::ServerState;

fn send_response<T: serde::Serialize>(
  sender: &crossbeam_channel::Sender<Message>,
  id: lsp_server::RequestId,
  result: T,
) -> Result<(), Box<dyn std::error::Error>> {
  let result = serde_json::to_value(&result)?;
  let resp = Response {
    id,
    result: Some(result),
    error: None,
  };
  sender.send(resp.into())?;
  Ok(())
}

fn send_error(
  sender: &crossbeam_channel::Sender<Message>,
  id: lsp_server::RequestId,
  code: i32,
  message: String,
) -> Result<(), Box<dyn std::error::Error>> {
  let resp = Response::new_err(id, code, message);
  sender.send(resp.into())?;
  Ok(())
}

pub fn handle_request(
  server: &mut ServerState,
  req: Request,
  sender: &crossbeam_channel::Sender<Message>,
) -> Result<(), Box<dyn std::error::Error>> {
  match req.method.as_str() {
    "textDocument/completion" => {
      let params: CompletionParams = serde_json::from_value(req.params)?;
      let completions = handle_completion(server, params);
      send_response(sender, req.id, completions)?;
    }
    "textDocument/hover" => {
      let params: HoverParams = serde_json::from_value(req.params)?;
      let hover = handle_hover(server, params);
      send_response(sender, req.id, hover)?;
    }
    "textDocument/definition" => {
      let params: GotoDefinitionParams = serde_json::from_value(req.params)?;
      let locations = handle_definition(server, params);
      send_response(sender, req.id, locations)?;
    }
    "textDocument/documentSymbol" => {
      let params: DocumentSymbolParams = serde_json::from_value(req.params)?;
      let symbols = handle_document_symbol(server, params);
      send_response(sender, req.id, symbols)?;
    }
    "textDocument/diagnostic" => {
      let params: DocumentDiagnosticParams = serde_json::from_value(req.params)?;
      let diagnostics = handle_diagnostic(server, params);
      send_response(sender, req.id, diagnostics)?;
    }
    _ => {
      send_error(
        sender,
        req.id,
        lsp_server::ErrorCode::MethodNotFound as i32,
        format!("Method {} not found", req.method),
      )?;
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
        uri.clone(),
        super::server::Document::new(text.clone(), version),
      );
      // Trigger diagnostics
      let diagnostics = compute_diagnostics(server, &uri);
      publish_diagnostics(&uri, diagnostics, server)?;
    }
    "textDocument/didChange" => {
      let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
      let uri = params.text_document.uri;
      let version = params.text_document.version;
      for change in params.content_changes {
        server.apply_change(&uri, change, version);
      }
      let diagnostics = compute_diagnostics(server, &uri);
      publish_diagnostics(&uri, diagnostics, server)?;
    }
    "textDocument/didClose" => {
      let params: DidCloseTextDocumentParams = serde_json::from_value(not.params)?;
      server.remove_document(&params.text_document.uri);
    }
    _ => {}
  }
  Ok(())
}

fn handle_completion(server: &ServerState, params: CompletionParams) -> CompletionList {
  let uri = &params.text_document_position.text_document.uri;
  let position = params.text_document_position.position;

  let context = if let Some(doc) = server.get_document(uri) {
    detect_completion_context(doc, position)
  } else {
    InternalCompletionContext::MaterialBlock
  };

  let engine = CompletionEngine::new();
  let items = engine.get_completions(context);
  let completion_items: Vec<CompletionItem> = items
    .into_iter()
    .map(conv::to_lsp_completion_item)
    .collect();

  CompletionList {
    is_incomplete: false,
    items: completion_items,
  }
}

fn detect_completion_context(
  doc: &super::server::Document,
  position: Position,
) -> InternalCompletionContext {
  let offset = doc.position_to_offset(position);
  let text = &doc.text[..offset];

  // Look backwards for the last property name before a colon
  if let Some(colon_idx) = text.rfind(':') {
    let before = &text[..colon_idx];
    let word_start = before
      .rfind(|c: char| !c.is_alphanumeric() && c != '_')
      .map(|i| i + 1)
      .unwrap_or(0);
    let word = before[word_start..].trim();

    match word {
      "requires" => return InternalCompletionContext::RequiresValue,
      "type" => return InternalCompletionContext::ParameterType,
      _ => {
        // Check if it's a known material property with enum values
        if filament_mat_lsp::schema::get_enum_values(word).is_some() {
          return InternalCompletionContext::PropertyValue(word.to_string());
        }
      }
    }
  }

  InternalCompletionContext::MaterialBlock
}

fn handle_hover(server: &ServerState, params: HoverParams) -> Option<Hover> {
  let uri = &params.text_document_position_params.text_document.uri;
  let position = params.text_document_position_params.position;

  let doc = server.get_document(uri)?;
  let word = extract_word_at_position(doc, position)?;

  let engine = HoverEngine::new();
  engine.get_hover(&word).map(|doc| Hover {
    contents: HoverContents::Markup(MarkupContent {
      kind: MarkupKind::Markdown,
      value: doc.clone(),
    }),
    range: None,
  })
}

fn extract_word_at_position(doc: &super::server::Document, position: Position) -> Option<String> {
  let offset = doc.position_to_offset(position);
  let text = &doc.text;

  let mut start = offset;
  let mut end = offset;

  // Go backwards to find word start
  while start > 0 {
    let prev = text[..start].chars().last()?;
    if !is_word_char(prev) {
      break;
    }
    start -= prev.len_utf8();
  }

  // Go forwards to find word end
  while end < text.len() {
    let next = text[end..].chars().next()?;
    if !is_word_char(next) {
      break;
    }
    end += next.len_utf8();
  }

  if start < end {
    Some(text[start..end].to_string())
  } else {
    None
  }
}

fn is_word_char(c: char) -> bool {
  c.is_alphanumeric() || c == '_'
}

fn handle_definition(
  server: &mut ServerState,
  params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
  let uri = &params.text_document_position_params.text_document.uri;
  let position = params.text_document_position_params.position;

  let doc = server.get_document(uri)?;
  let word = extract_word_at_position(doc, position)?;

  let material = match server.parse_document(uri)? {
    Ok(m) => m,
    Err(_) => return None,
  };

  // Check if word is a material property key
  let mut locations = Vec::new();

  // Check name property
  if word == "name"
    && let Some(name) = &material.name
  {
    locations.push(Location {
      uri: uri.clone(),
      range: conv::to_lsp_range(&name.range),
    });
  }

  // Check shadingModel property
  if word == "shadingModel"
    && let Some(sm) = &material.shading_model
  {
    locations.push(Location {
      uri: uri.clone(),
      range: conv::to_lsp_range(&sm.range),
    });
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
  server: &mut ServerState,
  params: DocumentDiagnosticParams,
) -> DocumentDiagnosticReportResult {
  let uri = params.text_document.uri;

  let diagnostics = compute_diagnostics(server, &uri);

  DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
    RelatedFullDocumentDiagnosticReport {
      related_documents: None,
      full_document_diagnostic_report: FullDocumentDiagnosticReport {
        result_id: None,
        items: diagnostics,
      },
    },
  ))
}

#[allow(deprecated)]
fn handle_document_symbol(
  server: &mut ServerState,
  params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
  let uri = &params.text_document.uri;
  let material = match server.parse_document(uri)? {
    Ok(m) => m,
    Err(_) => return None,
  };

  let mut symbols = Vec::new();

  // Material root symbol
  let material_symbol = DocumentSymbol {
    name: material
      .name
      .as_ref()
      .map(|n| n.value.clone())
      .unwrap_or_else(|| "Material".to_string()),
    detail: material
      .shading_model
      .as_ref()
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
        start: Position {
          line: 0,
          character: 0,
        },
        end: Position {
          line: 0,
          character: 0,
        },
      },
      selection_range: Range {
        start: Position {
          line: 0,
          character: 0,
        },
        end: Position {
          line: 0,
          character: 0,
        },
      },
      children: None,
    };
    symbols.push(param_symbol);
  }

  Some(DocumentSymbolResponse::Nested(symbols))
}

fn compute_diagnostics(server: &mut ServerState, uri: &Uri) -> Vec<lsp_types::Diagnostic> {
  let mut diagnostics = Vec::new();

  match server.parse_document(uri) {
    Some(Ok(material)) => {
      let validator = Validator::new();
      let internal_diagnostics = validator.validate_material(&material);
      diagnostics = internal_diagnostics
        .into_iter()
        .map(conv::to_lsp_diagnostic)
        .collect();
    }
    Some(Err(parse_err)) => {
      diagnostics.push(lsp_types::Diagnostic {
        range: conv::to_lsp_range(&parse_err.range),
        severity: Some(lsp_types::DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("filament-mat".to_string()),
        message: parse_err.message,
        related_information: None,
        tags: None,
        data: None,
      });
    }
    None => {}
  }

  diagnostics
}

fn publish_diagnostics(
  uri: &Uri,
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
