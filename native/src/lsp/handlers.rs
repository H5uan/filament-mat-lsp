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
    "textDocument/formatting" => {
      let params: DocumentFormattingParams = serde_json::from_value(req.params)?;
      let edits = handle_formatting(server, params);
      send_response(sender, req.id, edits)?;
    }
    "workspace/symbol" => {
      let params: WorkspaceSymbolParams = serde_json::from_value(req.params)?;
      let symbols = handle_workspace_symbol(server, params);
      send_response(sender, req.id, symbols)?;
    }
    "textDocument/rename" => {
      let params: RenameParams = serde_json::from_value(req.params)?;
      let edit = handle_rename(server, params);
      send_response(sender, req.id, edit)?;
    }
    "textDocument/prepareRename" => {
      let params: TextDocumentPositionParams = serde_json::from_value(req.params)?;
      let range = handle_prepare_rename(server, params);
      send_response(sender, req.id, range)?;
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
  server: &ServerState,
  params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
  let uri = &params.text_document.uri;
  let doc = server.get_document(uri)?;
  let data = super::semantic_tokens::generate_semantic_tokens(&doc.text);
  Some(SemanticTokensResult::Tokens(SemanticTokens {
    result_id: None,
    data,
  }))
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
