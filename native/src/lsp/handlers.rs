#![allow(dead_code)]

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
    "textDocument/codeAction" => {
      let params: CodeActionParams = serde_json::from_value(req.params)?;
      let actions = handle_code_action(server, params);
      send_response(sender, req.id, actions)?;
    }
    "textDocument/semanticTokens/full" => {
      let params: SemanticTokensParams = serde_json::from_value(req.params)?;
      let tokens = handle_semantic_tokens(server, params);
      send_response(sender, req.id, tokens)?;
    }
    "textDocument/semanticTokens/full/delta" => {
      let params: SemanticTokensDeltaParams = serde_json::from_value(req.params)?;
      let delta = handle_semantic_tokens_delta(server, params);
      send_response(sender, req.id, delta)?;
    }
    "textDocument/formatting" => {
      let params: DocumentFormattingParams = serde_json::from_value(req.params)?;
      let edits = handle_formatting(server, params);
      send_response(sender, req.id, edits)?;
    }
    "textDocument/rangeFormatting" => {
      let params: DocumentRangeFormattingParams = serde_json::from_value(req.params)?;
      let edits = handle_range_formatting(server, params);
      send_response(sender, req.id, edits)?;
    }
    "textDocument/documentColor" => {
      let params: DocumentColorParams = serde_json::from_value(req.params)?;
      let colors = handle_document_color(server, params);
      send_response(sender, req.id, colors)?;
    }
    "textDocument/colorPresentation" => {
      let params: ColorPresentationParams = serde_json::from_value(req.params)?;
      let presentations = handle_color_presentation(params);
      send_response(sender, req.id, presentations)?;
    }
    "textDocument/onTypeFormatting" => {
      let params: DocumentOnTypeFormattingParams = serde_json::from_value(req.params)?;
      let edits = handle_on_type_formatting(server, params);
      send_response(sender, req.id, edits)?;
    }
    "textDocument/documentLink" => {
      let params: DocumentLinkParams = serde_json::from_value(req.params)?;
      let links = handle_document_link(server, params);
      send_response(sender, req.id, links)?;
    }
    "textDocument/codeLens" => {
      let params: CodeLensParams = serde_json::from_value(req.params)?;
      let lenses = handle_code_lens(server, params);
      send_response(sender, req.id, lenses)?;
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
      // Mark for debounced diagnostics
      server
        .pending_diagnostics
        .insert(uri.clone(), (version, std::time::Instant::now()));
    }
    "textDocument/didChange" => {
      let params: DidChangeTextDocumentParams = serde_json::from_value(not.params)?;
      let uri = params.text_document.uri;
      let version = params.text_document.version;
      for change in params.content_changes {
        server.apply_change(&uri, change, version);
      }
      // Mark for debounced diagnostics instead of computing immediately
      server
        .pending_diagnostics
        .insert(uri.clone(), (version, std::time::Instant::now()));
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

  // Extract prefix for filtering
  let prefix = if let Some(doc) = server.get_document(uri) {
    extract_completion_prefix(doc, position)
  } else {
    String::new()
  };

  let engine = CompletionEngine::new();
  let items = engine.get_completions(context);
  let completion_items: Vec<CompletionItem> = items
    .into_iter()
    .filter(|item| {
      let text_to_match = item.filter_text.as_ref().unwrap_or(&item.label);
      text_to_match
        .to_lowercase()
        .starts_with(&prefix.to_lowercase())
    })
    .map(conv::to_lsp_completion_item)
    .collect();

  CompletionList {
    is_incomplete: !prefix.is_empty(),
    items: completion_items,
  }
}

/// Extract the text prefix before the cursor for completion filtering.
fn extract_completion_prefix(doc: &super::server::Document, position: Position) -> String {
  let offset = doc.position_to_offset(position);
  let text = &doc.text[..offset];

  // Find the start of the current word
  let word_start = text
    .rfind(|c: char| !c.is_alphanumeric() && c != '_')
    .map(|i| i + 1)
    .unwrap_or(0);

  text[word_start..].to_string()
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
      locations.push(Location {
        uri: uri.clone(),
        range: conv::to_lsp_range(&param.range),
      });
    }
  }

  if locations.is_empty() {
    None
  } else {
    Some(GotoDefinitionResponse::Array(locations))
  }
}

#[allow(clippy::mutable_key_type)]
fn handle_code_action(
  server: &mut ServerState,
  params: CodeActionParams,
) -> Option<Vec<CodeActionOrCommand>> {
  let uri = &params.text_document.uri;
  let material = match server.parse_document(uri)? {
    Ok(m) => m,
    Err(_) => return None,
  };

  let mut actions = Vec::new();

  // Check for missing name
  if material.name.is_none() {
    let insert_pos = Position {
      line: material.range.start.line,
      character: material.range.start.character + 1,
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
      title: "Add missing 'name' property".to_string(),
      kind: Some(CodeActionKind::QUICKFIX),
      diagnostics: None,
      disabled: None,
      edit: Some(WorkspaceEdit {
        changes: Some({
          let mut map = std::collections::HashMap::new();
          map.insert(
            uri.clone(),
            vec![TextEdit {
              range: Range {
                start: insert_pos,
                end: insert_pos,
              },
              new_text: "\n    name : MyMaterial,".to_string(),
            }],
          );
          map
        }),
        document_changes: None,
        change_annotations: None,
      }),
      command: None,
      is_preferred: Some(true),
      data: None,
    }));
  }

  // Check for missing shadingModel
  if material.shading_model.is_none() {
    let insert_pos = Position {
      line: material.range.start.line,
      character: material.range.start.character + 1,
    };
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
      title: "Add missing 'shadingModel' property".to_string(),
      kind: Some(CodeActionKind::QUICKFIX),
      diagnostics: None,
      disabled: None,
      edit: Some(WorkspaceEdit {
        changes: Some({
          let mut map = std::collections::HashMap::new();
          map.insert(
            uri.clone(),
            vec![TextEdit {
              range: Range {
                start: insert_pos,
                end: insert_pos,
              },
              new_text: "\n    shadingModel : lit,".to_string(),
            }],
          );
          map
        }),
        document_changes: None,
        change_annotations: None,
      }),
      command: None,
      is_preferred: Some(true),
      data: None,
    }));
  }

  if actions.is_empty() {
    None
  } else {
    Some(actions)
  }
}

fn handle_semantic_tokens(
  server: &mut ServerState,
  params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
  let uri = &params.text_document.uri;
  let (result_id, data) = server.get_semantic_tokens(uri)?;
  Some(SemanticTokensResult::Tokens(SemanticTokens {
    result_id: Some(result_id),
    data,
  }))
}

fn handle_semantic_tokens_delta(
  server: &mut ServerState,
  params: SemanticTokensDeltaParams,
) -> Option<SemanticTokensFullDeltaResult> {
  let uri = &params.text_document.uri;
  let previous_result_id = params.previous_result_id.as_str();

  let (result_id, _is_delta, _data, edits) =
    server.get_semantic_tokens_delta(uri, previous_result_id)?;

  Some(SemanticTokensFullDeltaResult::TokensDelta(
    SemanticTokensDelta {
      result_id: Some(result_id),
      edits,
    },
  ))
}

fn handle_formatting(
  server: &ServerState,
  params: DocumentFormattingParams,
) -> Option<Vec<TextEdit>> {
  let uri = &params.text_document.uri;
  let doc = server.get_document(uri)?;
  let formatted = format_mat_text(&doc.text);
  if formatted == doc.text {
    return Some(Vec::new());
  }
  Some(vec![TextEdit {
    range: Range {
      start: Position {
        line: 0,
        character: 0,
      },
      end: Position {
        line: u32::MAX,
        character: u32::MAX,
      },
    },
    new_text: formatted,
  }])
}

fn handle_range_formatting(
  server: &ServerState,
  params: DocumentRangeFormattingParams,
) -> Option<Vec<TextEdit>> {
  let uri = &params.text_document.uri;
  let doc = server.get_document(uri)?;
  let range = params.range;

  // Extract lines in the range
  let lines: Vec<&str> = doc.text.lines().collect();
  let start_line = range.start.line as usize;
  let end_line = range.end.line as usize;

  if start_line >= lines.len() {
    return Some(Vec::new());
  }

  let end_line = end_line.min(lines.len() - 1);

  // Calculate initial indent level from lines before the range
  let mut indent_level = 0usize;
  for line in lines.iter().take(start_line) {
    let trimmed = line.trim();
    if trimmed.ends_with('{') {
      indent_level += 1;
    }
    if trimmed.starts_with('}') || trimmed == "}" {
      indent_level = indent_level.saturating_sub(1);
    }
  }

  // Format the selected lines
  let mut formatted_lines = Vec::new();
  let mut in_glsl = false;

  // Check if we're inside a shader block
  for line in lines.iter().take(start_line) {
    let trimmed = line.trim();
    if trimmed.starts_with("vertex ")
      || trimmed.starts_with("fragment ")
      || trimmed.starts_with("compute ")
      || trimmed.starts_with("tool ")
    {
      in_glsl = true;
    }
    if trimmed == "}" && in_glsl {
      in_glsl = false;
    }
  }

  for line in lines.iter().take(end_line + 1).skip(start_line) {
    let trimmed = line.trim();

    if trimmed.is_empty() {
      formatted_lines.push(String::new());
      continue;
    }

    // Detect shader block start/end
    if trimmed.starts_with("vertex ")
      || trimmed.starts_with("fragment ")
      || trimmed.starts_with("compute ")
      || trimmed.starts_with("tool ")
    {
      in_glsl = true;
    }
    if trimmed == "}" && in_glsl {
      in_glsl = false;
    }

    if in_glsl
      && !trimmed.starts_with("material ")
      && !trimmed.starts_with("vertex ")
      && !trimmed.starts_with("fragment ")
      && !trimmed.starts_with("compute ")
      && !trimmed.starts_with("tool ")
    {
      // Preserve GLSL code as-is
      formatted_lines.push(line.to_string());
      continue;
    }

    // Decrease indent before closing brace
    if trimmed.starts_with('}') {
      indent_level = indent_level.saturating_sub(1);
    }

    // Build formatted line
    let mut formatted = String::new();
    for _ in 0..indent_level {
      formatted.push_str("    ");
    }
    formatted.push_str(trimmed);
    formatted_lines.push(formatted);

    // Increase indent after opening brace
    if trimmed.ends_with('{') {
      indent_level += 1;
    }

    // Decrease indent for lone closing braces
    if trimmed == "}" {
      indent_level = indent_level.saturating_sub(1);
    }
  }

  // Join formatted lines
  let new_text = formatted_lines.join("\n") + "\n";

  // Build the original text in the range
  let original_text = lines[start_line..=end_line].join("\n") + "\n";

  if new_text == original_text {
    return Some(Vec::new());
  }

  Some(vec![TextEdit {
    range: Range {
      start: Position {
        line: start_line as u32,
        character: 0,
      },
      end: Position {
        line: end_line as u32,
        character: lines[end_line].len() as u32,
      },
    },
    new_text,
  }])
}

fn format_mat_text(text: &str) -> String {
  let mut result = String::new();
  let mut indent_level = 0usize;
  let mut in_glsl = false;

  for line in text.lines() {
    let trimmed = line.trim();

    // Skip empty lines
    if trimmed.is_empty() {
      result.push('\n');
      continue;
    }

    // Detect shader block start/end for GLSL passthrough
    if trimmed.starts_with("vertex ")
      || trimmed.starts_with("fragment ")
      || trimmed.starts_with("compute ")
      || trimmed.starts_with("tool ")
    {
      in_glsl = true;
    }
    if trimmed == "}" && in_glsl {
      in_glsl = false;
    }

    if in_glsl
      && !trimmed.starts_with("material ")
      && !trimmed.starts_with("vertex ")
      && !trimmed.starts_with("fragment ")
      && !trimmed.starts_with("compute ")
      && !trimmed.starts_with("tool ")
    {
      // Preserve GLSL code as-is
      result.push_str(line);
      result.push('\n');
      continue;
    }

    // Decrease indent before closing brace
    if trimmed.starts_with('}') {
      indent_level = indent_level.saturating_sub(1);
    }

    // Add indentation
    for _ in 0..indent_level {
      result.push_str("    ");
    }

    // Add content
    result.push_str(trimmed);
    result.push('\n');

    // Increase indent after opening brace
    if trimmed.ends_with('{') {
      indent_level += 1;
    }

    // Decrease indent for lone closing braces
    if trimmed == "}" {
      indent_level = indent_level.saturating_sub(1);
    }
  }

  result
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
    let param_range = conv::to_lsp_range(&param.range);
    let param_symbol = DocumentSymbol {
      name: param.name.clone(),
      detail: Some(format!("type: {}", param.param_type)),
      kind: SymbolKind::PROPERTY,
      tags: None,
      deprecated: None,
      range: param_range,
      selection_range: param_range,
      children: None,
    };
    symbols.push(param_symbol);
  }

  Some(DocumentSymbolResponse::Nested(symbols))
}

pub fn compute_diagnostics(server: &mut ServerState, uri: &Uri) -> Vec<lsp_types::Diagnostic> {
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

pub fn publish_diagnostics(
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

#[allow(deprecated)]
fn handle_workspace_symbol(
  server: &mut ServerState,
  _params: WorkspaceSymbolParams,
) -> Option<Vec<SymbolInformation>> {
  let mut symbols = Vec::new();

  // Collect URIs first to avoid borrow issues
  let uris: Vec<Uri> = server.documents.keys().cloned().collect();

  // Collect from all open documents
  for uri in uris {
    if let Some(Ok(material)) = server.parse_document(&uri) {
      let name = material
        .name
        .as_ref()
        .map(|n| n.value.clone())
        .unwrap_or_else(|| "Material".to_string());
      let range = conv::to_lsp_range(&material.range);

      symbols.push(SymbolInformation {
        name,
        kind: SymbolKind::OBJECT,
        location: Location { uri, range },
        container_name: None,
        deprecated: None,
        tags: None,
      });
    }
  }

  if symbols.is_empty() {
    None
  } else {
    Some(symbols)
  }
}

fn handle_prepare_rename(
  server: &mut ServerState,
  params: TextDocumentPositionParams,
) -> Option<PrepareRenameResponse> {
  let uri = &params.text_document.uri;
  let position = params.position;

  let material = match server.parse_document(uri)? {
    Ok(m) => m,
    Err(_) => return None,
  };

  // Check if position is on a parameter name
  for param in &material.parameters {
    let range = conv::to_lsp_range(&param.range);
    if position_in_range(position, range) {
      return Some(PrepareRenameResponse::Range(range));
    }
  }

  None
}

fn position_in_range(position: Position, range: Range) -> bool {
  (position.line > range.start.line
    || (position.line == range.start.line && position.character >= range.start.character))
    && (position.line < range.end.line
      || (position.line == range.end.line && position.character <= range.end.character))
}

#[allow(clippy::mutable_key_type)]
fn handle_rename(server: &mut ServerState, params: RenameParams) -> Option<WorkspaceEdit> {
  let uri = &params.text_document_position.text_document.uri;
  let position = params.text_document_position.position;
  let new_name = &params.new_name;

  let matfile = server.parse_full_document(uri)?;

  let material = matfile.material;
  let mut changes: std::collections::HashMap<Uri, Vec<TextEdit>> = std::collections::HashMap::new();
  let mut edits = Vec::new();

  // Find which parameter is being renamed
  let mut target_param: Option<&filament_mat_lsp::parser::Parameter> = None;
  for param in &material.parameters {
    let range = conv::to_lsp_range(&param.range);
    if position_in_range(position, range) {
      target_param = Some(param);
      break;
    }
  }

  let target_param = target_param?;
  let old_name = &target_param.name;

  // Edit 1: parameter definition name field
  // We need to find the exact position of the name value inside the parameter object
  // Since we don't have sub-ranges, we'll search in the parameter range text
  if let Some(doc) = server.get_document(uri) {
    let param_range = &target_param.range;
    let param_text = &doc.text[param_range.start.line as usize..param_range.end.line as usize + 1];

    // Simple text search for name value
    let search_pattern = format!("name : {}", old_name);
    if let Some(idx) = param_text.find(&search_pattern) {
      let start_char = idx as u32 + 7; // skip "name : "
      let end_char = start_char + old_name.len() as u32;
      edits.push(TextEdit {
        range: Range {
          start: Position {
            line: param_range.start.line,
            character: start_char,
          },
          end: Position {
            line: param_range.start.line,
            character: end_char,
          },
        },
        new_text: new_name.clone(),
      });
    }
  }

  // Edit 2: shader references materialParams_xxx and materialParams.xxx
  for shader in &matfile.shaders {
    let search_old = format!("materialParams_{}", old_name);
    let search_dot = format!("materialParams.{}", old_name);
    let replace_old = format!("materialParams_{}", new_name);
    let replace_dot = format!("materialParams.{}", new_name);

    // Find all occurrences in shader code
    let mut offset = 0usize;
    while let Some(idx) = shader.code[offset..].find(&search_old) {
      let abs_idx = offset + idx;
      edits.push(TextEdit {
        range: Range {
          start: Position {
            line: shader.range.start.line + 1, // approximate
            character: abs_idx as u32,
          },
          end: Position {
            line: shader.range.start.line + 1,
            character: (abs_idx + search_old.len()) as u32,
          },
        },
        new_text: replace_old.clone(),
      });
      offset = abs_idx + search_old.len();
    }

    offset = 0;
    while let Some(idx) = shader.code[offset..].find(&search_dot) {
      let abs_idx = offset + idx;
      edits.push(TextEdit {
        range: Range {
          start: Position {
            line: shader.range.start.line + 1,
            character: abs_idx as u32,
          },
          end: Position {
            line: shader.range.start.line + 1,
            character: (abs_idx + search_dot.len()) as u32,
          },
        },
        new_text: replace_dot.clone(),
      });
      offset = abs_idx + search_dot.len();
    }
  }

  if edits.is_empty() {
    return None;
  }

  changes.insert(uri.clone(), edits);
  Some(WorkspaceEdit {
    changes: Some(changes),
    document_changes: None,
    change_annotations: None,
  })
}

fn handle_document_highlight(
  server: &mut ServerState,
  params: DocumentHighlightParams,
) -> Option<Vec<DocumentHighlight>> {
  let uri = &params.text_document_position_params.text_document.uri;
  let position = params.text_document_position_params.position;

  let doc = server.get_document(uri)?;
  let word = extract_word_at_position(doc, position)?;

  let matfile = server.parse_full_document(uri)?;

  let highlights = filament_mat_lsp::references::find_references(&matfile, &word, uri);

  if highlights.is_empty() {
    None
  } else {
    Some(highlights)
  }
}

fn handle_folding_range(
  server: &ServerState,
  params: FoldingRangeParams,
) -> Option<Vec<FoldingRange>> {
  let uri = &params.text_document.uri;
  let doc = server.get_document(uri)?;

  // Tokenize the document to find folding ranges
  use filament_mat_lsp::lexer::Lexer;
  use filament_mat_lsp::token::TokenType;

  let mut lexer = Lexer::new(&doc.text);
  let tokens = lexer.tokenize();

  let mut ranges = Vec::new();
  let mut stack: Vec<(u32, u32, TokenType)> = Vec::new();

  for token in tokens {
    match token.token_type {
      TokenType::LCurly | TokenType::LBracket => {
        stack.push((token.line, token.column, token.token_type.clone()));
      }
      TokenType::RCurly | TokenType::RBracket => {
        if let Some(start) = stack.pop() {
          // Only create folding ranges for meaningful blocks
          // Skip single-line ranges
          if token.line > start.0 {
            ranges.push(FoldingRange {
              start_line: start.0,
              start_character: Some(start.1),
              end_line: token.line,
              end_character: Some(token.column),
              kind: Some(FoldingRangeKind::Region),
              collapsed_text: None,
            });
          }
        }
      }
      _ => {}
    }
  }

  if ranges.is_empty() {
    None
  } else {
    Some(ranges)
  }
}

fn handle_references(server: &mut ServerState, params: ReferenceParams) -> Option<Vec<Location>> {
  let uri = &params.text_document_position.text_document.uri;
  let position = params.text_document_position.position;

  let doc = server.get_document(uri)?;
  let word = extract_word_at_position(doc, position)?;

  let matfile = server.parse_full_document(uri)?;

  let locations = filament_mat_lsp::references::find_reference_locations(&matfile, &word, uri);

  if locations.is_empty() {
    None
  } else {
    Some(locations)
  }
}

fn handle_signature_help(
  server: &ServerState,
  params: SignatureHelpParams,
) -> Option<SignatureHelp> {
  let uri = &params.text_document_position_params.text_document.uri;
  let position = params.text_document_position_params.position;

  let doc = server.get_document(uri)?;
  let offset = doc.position_to_offset(position);

  let function_name = filament_mat_lsp::signature_help::find_function_name(&doc.text, offset)?;

  let sig_info = filament_mat_lsp::signature_help::get_signature(&function_name)?;

  let active_parameter =
    filament_mat_lsp::signature_help::compute_active_parameter(&doc.text, offset);

  let parameters: Vec<lsp_types::ParameterInformation> = sig_info
    .parameters
    .iter()
    .map(|p| lsp_types::ParameterInformation {
      label: lsp_types::ParameterLabel::Simple(p.label.clone()),
      documentation: p
        .documentation
        .as_ref()
        .map(|d| lsp_types::Documentation::String(d.clone())),
    })
    .collect();

  let signature = lsp_types::SignatureInformation {
    label: sig_info.label.clone(),
    documentation: sig_info
      .documentation
      .as_ref()
      .map(|d| lsp_types::Documentation::String(d.clone())),
    parameters: Some(parameters),
    active_parameter: None,
  };

  Some(SignatureHelp {
    signatures: vec![signature],
    active_signature: Some(0),
    active_parameter: Some(active_parameter),
  })
}

fn handle_selection_range(
  server: &mut ServerState,
  params: SelectionRangeParams,
) -> Option<Vec<SelectionRange>> {
  let uri = &params.text_document.uri;

  let matfile = server.parse_full_document(uri)?;

  let mut result = Vec::new();
  for position in params.positions {
    let ranges = filament_mat_lsp::selection_range::build_selection_ranges(&matfile, position);
    if let Some(first) = ranges.first() {
      result.push(first.clone());
    }
  }

  if result.is_empty() {
    None
  } else {
    Some(result)
  }
}

fn handle_inlay_hints(server: &mut ServerState, params: InlayHintParams) -> Option<Vec<InlayHint>> {
  let uri = &params.text_document.uri;

  let matfile = server.parse_full_document(uri)?;

  let hints = filament_mat_lsp::inlay_hints::generate_inlay_hints(&matfile, params.range);

  if hints.is_empty() { None } else { Some(hints) }
}

fn handle_document_color(
  server: &ServerState,
  params: DocumentColorParams,
) -> Option<Vec<ColorInformation>> {
  let uri = &params.text_document.uri;
  let doc = server.get_document(uri)?;

  let colors = filament_mat_lsp::color_provider::find_colors(&doc.text);

  if colors.is_empty() {
    None
  } else {
    Some(colors)
  }
}

fn handle_color_presentation(params: ColorPresentationParams) -> Option<Vec<ColorPresentation>> {
  let presentations =
    filament_mat_lsp::color_provider::get_color_presentations(params.color, params.range);

  if presentations.is_empty() {
    None
  } else {
    Some(presentations)
  }
}

fn handle_on_type_formatting(
  server: &ServerState,
  params: DocumentOnTypeFormattingParams,
) -> Option<Vec<TextEdit>> {
  let uri = &params.text_document_position.text_document.uri;
  let doc = server.get_document(uri)?;
  let position = params.text_document_position.position;
  let ch = params.ch;

  // Only handle '}' for now - adjust indent of current line
  if ch != "}" {
    return Some(Vec::new());
  }

  let line_idx = position.line as usize;
  let lines: Vec<&str> = doc.text.lines().collect();

  if line_idx >= lines.len() {
    return Some(Vec::new());
  }

  let current_line = lines[line_idx];
  let trimmed = current_line.trim();

  // Only format if this line is just a closing brace
  if trimmed != "}" {
    return Some(Vec::new());
  }

  // Calculate expected indent level by looking at previous lines
  let mut indent_level = 0usize;
  for line in lines.iter().take(line_idx) {
    let t = line.trim();
    if t.ends_with('{') {
      indent_level += 1;
    }
    if t.starts_with('}') || t == "}" {
      indent_level = indent_level.saturating_sub(1);
    }
  }

  // Decrease indent for this closing brace
  indent_level = indent_level.saturating_sub(1);

  let mut expected = String::new();
  for _ in 0..indent_level {
    expected.push_str("    ");
  }
  expected.push('}');

  if current_line == expected {
    return Some(Vec::new());
  }

  Some(vec![TextEdit {
    range: Range {
      start: Position {
        line: line_idx as u32,
        character: 0,
      },
      end: Position {
        line: line_idx as u32,
        character: current_line.len() as u32,
      },
    },
    new_text: expected,
  }])
}

fn handle_document_link(
  server: &ServerState,
  params: DocumentLinkParams,
) -> Option<Vec<DocumentLink>> {
  let uri = &params.text_document.uri;
  let doc = server.get_document(uri)?;

  let mut links = Vec::new();
  let text = &doc.text;

  // Find material property lines and create links for known enum values
  for (line_idx, line) in text.lines().enumerate() {
    let trimmed = line.trim();

    // shadingModel: lit → link to shading model docs
    if trimmed.contains("shadingModel")
      && let Some(colon_idx) = trimmed.find(':')
    {
      let value_part = &trimmed[colon_idx + 1..].trim();
      if let Some(value) = value_part.split([',', '}']).next() {
        let value = value.trim();
        if !value.is_empty() {
          let value_start = line.find(value).unwrap_or(0) as u32;
          links.push(DocumentLink {
            range: Range {
              start: Position {
                line: line_idx as u32,
                character: value_start,
              },
              end: Position {
                line: line_idx as u32,
                character: value_start + value.len() as u32,
              },
            },
            target: Some(
              "https://google.github.io/filament/Materials.html#shadingmodel"
                .parse::<Uri>()
                .unwrap(),
            ),
            tooltip: Some("Open Filament shading model documentation".to_string()),
            data: None,
          });
        }
      }
    }

    // blendMode: opaque → link to blend mode docs
    if trimmed.contains("blendMode")
      && let Some(colon_idx) = trimmed.find(':')
    {
      let value_part = &trimmed[colon_idx + 1..].trim();
      if let Some(value) = value_part.split([',', '}']).next() {
        let value = value.trim();
        if !value.is_empty() {
          let value_start = line.find(value).unwrap_or(0) as u32;
          links.push(DocumentLink {
            range: Range {
              start: Position {
                line: line_idx as u32,
                character: value_start,
              },
              end: Position {
                line: line_idx as u32,
                character: value_start + value.len() as u32,
              },
            },
            target: Some(
              "https://google.github.io/filament/Materials.html#blendmode"
                .parse::<Uri>()
                .unwrap(),
            ),
            tooltip: Some("Open Filament blend mode documentation".to_string()),
            data: None,
          });
        }
      }
    }
  }

  if links.is_empty() { None } else { Some(links) }
}

fn handle_code_lens(server: &mut ServerState, params: CodeLensParams) -> Option<Vec<CodeLens>> {
  let uri = &params.text_document.uri;
  let matfile = server.parse_full_document(uri)?;

  let mut lenses = Vec::new();

  // Add code lens for each parameter showing reference count
  for param in &matfile.material.parameters {
    let name = &param.name;

    // Count references in shader blocks
    let mut ref_count = 0;
    let param_ref = format!("materialParams.{}", name);

    for shader in &matfile.shaders {
      ref_count += shader.code.matches(&param_ref).count();
    }

    if ref_count > 0 {
      let start = Position {
        line: param.range.start.line,
        character: param.range.start.character,
      };
      let end = Position {
        line: param.range.end.line,
        character: param.range.end.character,
      };
      lenses.push(CodeLens {
        range: Range { start, end },
        command: Some(Command {
          title: format!(
            "{} reference{}",
            ref_count,
            if ref_count == 1 { "" } else { "s" }
          ),
          command: "editor.action.showReferences".to_string(),
          arguments: Some(vec![
            serde_json::to_value(uri).unwrap(),
            serde_json::to_value(start).unwrap(),
            serde_json::to_value(Vec::new() as Vec<Location>).unwrap(),
          ]),
        }),
        data: None,
      });
    }
  }

  if lenses.is_empty() {
    None
  } else {
    Some(lenses)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::lsp::server::ServerState;

  fn create_test_server(text: &str) -> ServerState {
    let (sender, _) = crossbeam_channel::unbounded();
    let mut server = ServerState::new(sender);
    let uri: Uri = "file:///test.mat".parse().unwrap();
    server.insert_document(uri, crate::lsp::server::Document::new(text.to_string(), 1));
    server
  }

  #[test]
  fn test_on_type_formatting_brace() {
    // Note: the last line has wrong indentation (4 spaces instead of 0)
    let text = "material {\n    vertex {\n        void main() {\n        }\n    }\n    }";
    let server = create_test_server(text);
    let uri: Uri = "file:///test.mat".parse().unwrap();

    let params = DocumentOnTypeFormattingParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position {
          line: 5,
          character: 5,
        },
      },
      ch: "}".to_string(),
      options: FormattingOptions::default(),
    };

    let edits = handle_on_type_formatting(&server, params);
    assert!(edits.is_some());
    let edits = edits.unwrap();
    assert!(!edits.is_empty());
    // The closing brace should be dedented to match 'material {'
    assert_eq!(edits[0].new_text, "}");
  }

  #[test]
  fn test_on_type_formatting_not_brace() {
    let text = "material { }";
    let server = create_test_server(text);
    let uri: Uri = "file:///test.mat".parse().unwrap();

    let params = DocumentOnTypeFormattingParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position {
          line: 0,
          character: 10,
        },
      },
      ch: ";".to_string(),
      options: FormattingOptions::default(),
    };

    let edits = handle_on_type_formatting(&server, params);
    assert!(edits.is_some());
    assert!(edits.unwrap().is_empty());
  }

  #[test]
  fn test_document_link_shading_model() {
    let text = "material {\n    shadingModel : lit,\n}";
    let server = create_test_server(text);
    let uri: Uri = "file:///test.mat".parse().unwrap();

    let params = DocumentLinkParams {
      text_document: TextDocumentIdentifier { uri: uri.clone() },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
    };

    let links = handle_document_link(&server, params);
    assert!(links.is_some());
    let links = links.unwrap();
    assert_eq!(links.len(), 1);
    assert!(links[0].target.is_some());
  }

  #[test]
  fn test_document_link_blend_mode() {
    let text = "material {\n    blendMode : opaque,\n}";
    let server = create_test_server(text);
    let uri: Uri = "file:///test.mat".parse().unwrap();

    let params = DocumentLinkParams {
      text_document: TextDocumentIdentifier { uri: uri.clone() },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
    };

    let links = handle_document_link(&server, params);
    assert!(links.is_some());
    let links = links.unwrap();
    assert_eq!(links.len(), 1);
    assert!(links[0].target.is_some());
  }
}
