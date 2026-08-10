use lsp_server::{Message, Notification, Request, Response};
use lsp_types::*;

use filament_mat_lsp::completion::{
  CompletionContext as InternalCompletionContext, CompletionEngine,
};
use filament_mat_lsp::diagnostics::Validator;
use filament_mat_lsp::hover::HoverEngine;
use filament_mat_lsp::matc;

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
    "textDocument/references" => {
      let params: ReferenceParams = serde_json::from_value(req.params)?;
      let locations = handle_references(server, params);
      send_response(sender, req.id, locations)?;
    }
    "textDocument/prepareRename" => {
      let params: TextDocumentPositionParams = serde_json::from_value(req.params)?;
      let range = handle_prepare_rename(server, params);
      send_response(sender, req.id, range)?;
    }
    "textDocument/rename" => {
      let params: RenameParams = serde_json::from_value(req.params)?;
      let edit = handle_rename(server, params);
      send_response(sender, req.id, edit)?;
    }
    "textDocument/documentHighlight" => {
      let params: DocumentHighlightParams = serde_json::from_value(req.params)?;
      let highlights = handle_document_highlight(server, params);
      send_response(sender, req.id, highlights)?;
    }
    "textDocument/foldingRange" => {
      let params: FoldingRangeParams = serde_json::from_value(req.params)?;
      let ranges = handle_folding_range(server, params);
      send_response(sender, req.id, ranges)?;
    }
    "textDocument/signatureHelp" => {
      let params: SignatureHelpParams = serde_json::from_value(req.params)?;
      let help = handle_signature_help(server, params);
      send_response(sender, req.id, help)?;
    }
    "textDocument/selectionRange" => {
      let params: SelectionRangeParams = serde_json::from_value(req.params)?;
      let ranges = handle_selection_range(server, params);
      send_response(sender, req.id, ranges)?;
    }
    "textDocument/inlayHint" => {
      let params: InlayHintParams = serde_json::from_value(req.params)?;
      let hints = handle_inlay_hints(server, params);
      send_response(sender, req.id, hints)?;
    }
    "workspace/symbol" => {
      let params: WorkspaceSymbolParams = serde_json::from_value(req.params)?;
      let symbols = handle_workspace_symbol(server, params);
      send_response(sender, req.id, symbols)?;
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
    "textDocument/didSave" => {
      let params: DidSaveTextDocumentParams = serde_json::from_value(not.params)?;
      let uri = params.text_document.uri;
      if let Some(doc) = server.get_document(&uri) {
        let text = doc.text.clone();
        let mut all_diags = compute_diagnostics(server, &uri);
        if let Some(matc_diags) = compute_matc_diagnostics(server, &uri, &text) {
          all_diags.extend(matc_diags);
        }
        if let Err(e) = publish_diagnostics(&uri, all_diags, server) {
          eprintln!("Error publishing diagnostics: {}", e);
        }
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

fn handle_completion(server: &ServerState, params: CompletionParams) -> CompletionList {
  let uri = &params.text_document_position.text_document.uri;
  let position = params.text_document_position.position;

  // TODO: detect shading model from parsed material for smart field filtering.
  // Currently requires &mut ServerState which isn't available in handle_completion.
  let shading_model: Option<String> = None;

  let context = if let Some(doc) = server.get_document(uri) {
    detect_completion_context(doc, position, shading_model)
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
  shading_model: Option<String>,
) -> InternalCompletionContext {
  let offset = doc.position_to_offset(position);
  let text = &doc.text[..offset];

  // Detect "material." prefix for material field completion
  // Look for "material." in the text before the cursor
  if let Some(dot_idx) = text.rfind("material.") {
    // Check that this is the start of a word or follows a non-alphanumeric char
    let is_word_start = dot_idx == 0
      || !text[..dot_idx]
        .chars()
        .last()
        .map(|c| c.is_alphanumeric() || c == '_')
        .unwrap_or(false);
    if is_word_start {
      // Determine if we're in a vertex shader scope
      // Look backwards for the nearest shader block keyword
      let vertex_scope = text[..dot_idx]
        .lines()
        .last()
        .map(|line| line.trim().starts_with("vertex"))
        .unwrap_or(false);

      return InternalCompletionContext::MaterialField {
        shading_model,
        vertex_scope,
      };
    }
  }

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

fn handle_hover(server: &mut ServerState, params: HoverParams) -> Option<Hover> {
  let uri = &params.text_document_position_params.text_document.uri;
  let position = params.text_document_position_params.position;

  let doc = server.get_document(uri)?;
  let word = extract_word_at_position(doc, position)?;

  // User-defined material parameters take precedence: hovering on a parameter
  // (declaration or a `materialParams.X` / `materialParams_X` usage) shows its
  // declared type.
  if let Some(Ok(material)) = server.parse_document(uri)
    && let Some(param) = material
      .parameters
      .iter()
      .find(|p| p.name == word || param_name_from_word(&word) == Some(p.name.as_str()))
  {
    let value = format!(
      "**Parameter `{}`**\n\nType: `{}`\n\nDeclared in the `material` block and accessed from the shader as `materialParams.{}`.",
      param.name, param.param_type, param.name
    );
    return Some(Hover {
      contents: HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value,
      }),
      range: None,
    });
  }

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

/// Extract the word at `position` together with its byte range in the document.
fn extract_word_range_at_position(
  doc: &super::server::Document,
  position: Position,
) -> Option<(String, Range)> {
  let offset = doc.position_to_offset(position);
  let text = &doc.text;

  let mut start = offset;
  let mut end = offset;

  while start > 0 {
    let prev = text[..start].chars().last()?;
    if !is_word_char(prev) {
      break;
    }
    start -= prev.len_utf8();
  }

  while end < text.len() {
    let next = text[end..].chars().next()?;
    if !is_word_char(next) {
      break;
    }
    end += next.len_utf8();
  }

  if start < end {
    Some((
      text[start..end].to_string(),
      Range {
        start: offset_to_line_col(text, start, 0),
        end: offset_to_line_col(text, end, 0),
      },
    ))
  } else {
    None
  }
}

fn is_word_char(c: char) -> bool {
  c.is_alphanumeric() || c == '_'
}

/// When a shader references a parameter via the underscore form
/// (`materialParams_<name>`), the whole token is extracted as a single word
/// because `_` is a word character. Strip the prefix so the name can be matched
/// against declared parameters.
fn param_name_from_word(word: &str) -> Option<&str> {
  word
    .strip_prefix("materialParams_")
    .filter(|r| !r.is_empty())
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
  let matfile = server.parse_full_document(uri)?;

  let material = &matfile.material;
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

  // Add shader block symbols (functions, uniforms, varyings)
  use filament_mat_lsp::shader_symbols::ShaderSymbolKind;
  for shader in &matfile.shaders {
    let shader_label = format!("{:?} shader", shader.block_type);
    let shader_range = conv::to_lsp_range(&shader.range);
    let mut shader_children = Vec::new();

    for sym in &shader.symbols {
      let (kind, detail) = match sym.kind {
        ShaderSymbolKind::Function => (SymbolKind::FUNCTION, None),
        ShaderSymbolKind::Uniform => (SymbolKind::VARIABLE, Some("uniform".to_string())),
        ShaderSymbolKind::Varying => (SymbolKind::VARIABLE, Some("varying".to_string())),
        ShaderSymbolKind::MaterialParamRef => continue, // Skip references in outline
        ShaderSymbolKind::MaterialFieldRef => continue,
      };

      // Convert byte offset within shader code to line/col in document
      let code_before = &shader.code[..sym.byte_start];
      let newlines = code_before.matches('\n').count() as u32;
      let last_newline = code_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
      let sym_line = shader.range.start.line + 1 + newlines;
      let sym_col = (sym.byte_start - last_newline) as u32;

      let sym_range = Range {
        start: Position {
          line: sym_line,
          character: sym_col,
        },
        end: Position {
          line: sym_line,
          character: sym_col + (sym.byte_end - sym.byte_start) as u32,
        },
      };

      shader_children.push(DocumentSymbol {
        name: sym.name.clone(),
        detail,
        kind,
        tags: None,
        deprecated: None,
        range: sym_range,
        selection_range: sym_range,
        children: None,
      });
    }

    if !shader_children.is_empty() {
      symbols.push(DocumentSymbol {
        name: shader_label,
        detail: None,
        kind: SymbolKind::NAMESPACE,
        tags: None,
        deprecated: None,
        range: shader_range,
        selection_range: shader_range,
        children: Some(shader_children),
      });
    }
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
        source: Some("filament-mat-lsp".to_string()),
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

fn compute_matc_diagnostics(
  server: &ServerState,
  _uri: &Uri,
  text: &str,
) -> Option<Vec<lsp_types::Diagnostic>> {
  let config = server.matc_config.as_ref()?;
  // Write text to temp file for matc
  let temp_path = std::env::temp_dir().join("filament_mat_temp.mat");
  std::fs::write(&temp_path, text).ok()?;
  let matc_diags = matc::run_matc(config, &temp_path).ok()?;
  Some(
    matc_diags
      .into_iter()
      .map(|d| lsp_types::Diagnostic {
        range: lsp_types::Range {
          start: lsp_types::Position {
            line: d.line.unwrap_or(0),
            character: 0,
          },
          end: lsp_types::Position {
            line: d.line.unwrap_or(0),
            character: 1,
          },
        },
        severity: Some(match d.severity {
          filament_mat_lsp::diagnostics::DiagnosticSeverity::Error => {
            lsp_types::DiagnosticSeverity::ERROR
          }
          filament_mat_lsp::diagnostics::DiagnosticSeverity::Warning => {
            lsp_types::DiagnosticSeverity::WARNING
          }
        }),
        code: None,
        code_description: None,
        source: Some("matc".to_string()),
        message: d.message,
        related_information: None,
        tags: None,
        data: None,
      })
      .collect(),
  )
}

pub fn publish_diagnostics(
  uri: &Uri,
  diagnostics: Vec<lsp_types::Diagnostic>,
  server: &ServerState,
) -> Result<(), Box<dyn std::error::Error>> {
  let params = PublishDiagnosticsParams {
    uri: uri.clone(),
    diagnostics,
    version: server.get_document(uri).map(|doc| doc.version),
  };
  let not = lsp_server::Notification::new(
    "textDocument/publishDiagnostics".to_string(),
    serde_json::to_value(params)?,
  );
  server.send(not.into())?;
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
    // Material symbols
    if let Some(Ok(material)) = server.parse_document(&uri) {
      let name = material
        .name
        .as_ref()
        .map(|n| n.value.clone())
        .unwrap_or_else(|| "Material".to_string());
      let range = conv::to_lsp_range(&material.range);

      let material_name = name.clone();
      symbols.push(SymbolInformation {
        name,
        kind: SymbolKind::OBJECT,
        location: Location {
          uri: uri.clone(),
          range,
        },
        container_name: None,
        deprecated: None,
        tags: None,
      });

      // Parameter symbols
      for param in &material.parameters {
        symbols.push(SymbolInformation {
          name: param.name.clone(),
          kind: SymbolKind::PROPERTY,
          location: Location {
            uri: uri.clone(),
            range: conv::to_lsp_range(&param.range),
          },
          container_name: Some(material_name.clone()),
          deprecated: None,
          tags: None,
        });
      }
    }

    // Shader function/uniform symbols from full parse
    if let Some(matfile) = server.parse_full_document(&uri) {
      for shader in &matfile.shaders {
        use filament_mat_lsp::shader_symbols::ShaderSymbolKind;
        let container = format!("{:?} shader", shader.block_type);

        for sym in &shader.symbols {
          let (kind, label) = match sym.kind {
            ShaderSymbolKind::Function => (SymbolKind::FUNCTION, sym.name.clone()),
            ShaderSymbolKind::Uniform => (SymbolKind::VARIABLE, format!("uniform {}", sym.name)),
            ShaderSymbolKind::Varying => (SymbolKind::VARIABLE, format!("varying {}", sym.name)),
            ShaderSymbolKind::MaterialParamRef | ShaderSymbolKind::MaterialFieldRef => continue,
          };

          let code_before = &shader.code[..sym.byte_start];
          let newlines = code_before.matches('\n').count() as u32;
          let last_newline = code_before.rfind('\n').map(|i| i + 1).unwrap_or(0);
          let sym_line = shader.range.start.line + 1 + newlines;
          let sym_col = (sym.byte_start - last_newline) as u32;

          symbols.push(SymbolInformation {
            name: label,
            kind,
            location: Location {
              uri: uri.clone(),
              range: Range {
                start: Position {
                  line: sym_line,
                  character: sym_col,
                },
                end: Position {
                  line: sym_line,
                  character: sym_col + (sym.byte_end - sym.byte_start) as u32,
                },
              },
            },
            container_name: Some(container.clone()),
            deprecated: None,
            tags: None,
          });
        }
      }
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

  // Check if position is on a parameter name (declaration).
  for param in &material.parameters {
    let range = conv::to_lsp_range(&param.range);
    if position_in_range(position, range) {
      return Some(PrepareRenameResponse::Range(range));
    }
  }

  // Fall back to a parameter referenced from the shader body (materialParams.X
  // or materialParams_X). Extract the word; if it matches a parameter name it is
  // a valid rename target, and we highlight the word itself.
  let doc = server.get_document(uri)?;
  let (word, word_range) = extract_word_range_at_position(doc, position)?;
  if material
    .parameters
    .iter()
    .any(|p| p.name == word || param_name_from_word(&word) == Some(p.name.as_str()))
  {
    return Some(PrepareRenameResponse::Range(word_range));
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

  // Find which parameter is being renamed. This works both when the cursor is on
  // the parameter declaration and when it is on a shader usage (materialParams.X
  // or materialParams_X), since both resolve to a parameter name.
  let mut target_name: Option<String> = None;
  for param in &material.parameters {
    let range = conv::to_lsp_range(&param.range);
    if position_in_range(position, range) {
      target_name = Some(param.name.clone());
      break;
    }
  }

  if target_name.is_none()
    && let Some(doc) = server.get_document(uri)
    && let Some((word, _)) = extract_word_range_at_position(doc, position)
    && let Some(name) = material
      .parameters
      .iter()
      .find(|p| p.name == word || param_name_from_word(&word) == Some(p.name.as_str()))
      .map(|p| p.name.clone())
  {
    target_name = Some(name);
  }

  let old_name = &target_name?;

  // Edit 1: parameter definition name field
  // Search the whole document for "name : <old_name>" / "name:<old_name>".
  // Searching the full text (rather than a single parameter's range) lets the
  // rename work both when triggered on the declaration and on a shader usage.
  if let Some(doc) = server.get_document(uri) {
    let text = doc.text.as_str();
    let search_pattern = format!("name : {}", old_name);
    let search_pattern2 = format!("name:{}", old_name);
    let mut offset = 0usize;
    while let Some(idx) = text[offset..].find(&search_pattern) {
      let abs_idx = offset + idx;
      let name_start = abs_idx + search_pattern.len() - old_name.len();
      let start = offset_to_line_col(text, name_start, 0);
      edits.push(TextEdit {
        range: Range {
          start,
          end: Position {
            line: start.line,
            character: start.character + old_name.len() as u32,
          },
        },
        new_text: new_name.clone(),
      });
      offset = abs_idx + search_pattern.len();
    }
    offset = 0;
    while let Some(idx) = text[offset..].find(&search_pattern2) {
      let abs_idx = offset + idx;
      let name_start = abs_idx + search_pattern2.len() - old_name.len();
      let start = offset_to_line_col(text, name_start, 0);
      edits.push(TextEdit {
        range: Range {
          start,
          end: Position {
            line: start.line,
            character: start.character + old_name.len() as u32,
          },
        },
        new_text: new_name.clone(),
      });
      offset = abs_idx + search_pattern2.len();
    }
  }

  // Edit 2: shader references materialParams_xxx and materialParams.xxx
  // The shader code starts at the line after the opening brace.
  for shader in &matfile.shaders {
    let search_old = format!("materialParams_{}", old_name);
    let search_dot = format!("materialParams.{}", old_name);
    let replace_old = format!("materialParams_{}", new_name);
    let replace_dot = format!("materialParams.{}", new_name);

    let base_line = shader.range.start.line + 1;

    // Find all occurrences in shader code
    let mut offset = 0usize;
    while let Some(idx) = shader.code[offset..].find(&search_old) {
      let abs_idx = offset + idx;
      let pos = offset_to_line_col(&shader.code, abs_idx, base_line);
      edits.push(TextEdit {
        range: Range {
          start: pos,
          end: Position {
            line: pos.line,
            character: pos.character + search_old.len() as u32,
          },
        },
        new_text: replace_old.clone(),
      });
      offset = abs_idx + search_old.len();
    }

    offset = 0;
    while let Some(idx) = shader.code[offset..].find(&search_dot) {
      let abs_idx = offset + idx;
      let pos = offset_to_line_col(&shader.code, abs_idx, base_line);
      edits.push(TextEdit {
        range: Range {
          start: pos,
          end: Position {
            line: pos.line,
            character: pos.character + search_dot.len() as u32,
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

/// Convert a byte offset within a multi-line string to a Position (line, column),
/// relative to the given base line.
fn offset_to_line_col(code: &str, offset: usize, base_line: u32) -> Position {
  let before = &code[..offset];
  let newlines = before.matches('\n').count() as u32;
  let last_newline = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
  Position {
    line: base_line + newlines,
    character: (offset - last_newline) as u32,
  }
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

  // Map of property names → documentation URL fragments
  let doc_links: [(&str, &str); 4] = [
    ("shadingModel", "shadingmodel"),
    ("blending", "blending"),
    ("culling", "culling"),
    ("transparency", "transparency"),
  ];

  for (line_idx, line) in text.lines().enumerate() {
    let trimmed = line.trim();

    for (prop_name, doc_fragment) in &doc_links {
      if trimmed.contains(prop_name)
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
                format!(
                  "https://google.github.io/filament/Materials.html#{}",
                  doc_fragment
                )
                .parse::<Uri>()
                .unwrap(),
              ),
              tooltip: Some(format!("Open Filament {} documentation", prop_name)),
              data: None,
            });
          }
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

  fn test_uri() -> Uri {
    "file:///test.mat".parse().unwrap()
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
    let text = "material {\n    blending : opaque,\n}";
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

  // ---------------------------------------------------------------------------
  // Integration tests — full handler pipeline
  // ---------------------------------------------------------------------------

  #[test]
  fn test_integration_completion_material_block() {
    let text = "material {\n    \n}";
    let server = create_test_server(text);
    let uri = test_uri();

    let params = CompletionParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: 1,
          character: 4,
        },
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
      context: None,
    };

    let result = handle_completion(&server, params);
    assert!(result.items.len() > 10, "Expected many completion items");
    // Should include common material properties
    let labels: Vec<&str> = result.items.iter().map(|i| i.label.as_str()).collect();
    assert!(
      labels.contains(&"shadingModel"),
      "shadingModel should be in completion"
    );
    assert!(
      labels.contains(&"blending"),
      "blending should be in completion"
    );
    assert!(labels.contains(&"name"), "name should be in completion");
    assert!(
      labels.contains(&"parameters"),
      "parameters should be in completion"
    );
  }

  #[test]
  fn test_integration_hover_known_keyword() {
    let text = "material {\n    shadingModel : lit\n}";
    let server = create_test_server(text);
    let uri = test_uri();

    let params = HoverParams {
      text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: 1,
          character: 6,
        }, // on "shadingModel"
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let mut server = server;
    let result = handle_hover(&mut server, params);
    assert!(
      result.is_some(),
      "Hover should return content for 'shadingModel'"
    );
    let hover = result.unwrap();
    match hover.contents {
      HoverContents::Markup(content) => {
        assert!(
          content.value.to_lowercase().contains("shading"),
          "Hover should mention shading, got: {}",
          content.value
        );
      }
      _ => panic!("Expected Markup content"),
    }
  }

  #[test]
  fn test_integration_diagnostics_missing_name() {
    let text = "material {\n    shadingModel : lit\n}";
    let mut server = create_test_server(text);
    let uri = test_uri();

    let params = DocumentDiagnosticParams {
      text_document: TextDocumentIdentifier { uri },
      identifier: None,
      previous_result_id: None,
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
    };

    let result = handle_diagnostic(&mut server, params);
    match result {
      DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
        let has_missing_name = report
          .full_document_diagnostic_report
          .items
          .iter()
          .any(|d| d.message.contains("name"));
        assert!(has_missing_name, "Should report missing 'name' property");
      }
      _ => panic!("Expected Full report"),
    }
  }

  #[test]
  fn test_integration_diagnostics_valid_material() {
    let text = "material {\n    name : Test,\n    shadingModel : lit\n}";
    let mut server = create_test_server(text);
    let uri = test_uri();

    let params = DocumentDiagnosticParams {
      text_document: TextDocumentIdentifier { uri },
      identifier: None,
      previous_result_id: None,
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
    };

    let result = handle_diagnostic(&mut server, params);
    match result {
      DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) => {
        let errors: Vec<_> = report
          .full_document_diagnostic_report
          .items
          .iter()
          .filter(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
          .collect();
        assert!(
          errors.is_empty(),
          "Valid material should have no errors: {:?}",
          errors
        );
      }
      _ => panic!("Expected Full report"),
    }
  }

  #[test]
  fn test_integration_definition_parameter() {
    let text = "material {\n    name : Test,\n    shadingModel : lit,\n    parameters : [\n        { type : float, name : roughness }\n    ]\n}";
    let mut server = create_test_server(text);
    let uri = test_uri();

    // Definition uses `extract_word_at_position` on the raw text, then
    // matches against parameter names. Position must be on the parameter name.
    let params = GotoDefinitionParams {
      text_document_position_params: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position {
          line: 4,
          character: 35,
        }, // 0-based: middle of "roughness"
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
    };

    let result = handle_definition(&mut server, params);
    assert!(
      result.is_some(),
      "Definition should find parameter 'roughness'"
    );
  }

  #[test]
  fn test_integration_rename_parameter() {
    let text = "material {\n    name : Test,\n    shadingModel : lit,\n    parameters : [\n        { type : float, name : roughness }\n    ]\n}\nfragment {\n    materialParams.roughness;\n}";
    let mut server = create_test_server(text);
    let uri = test_uri();

    let params = RenameParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position {
          line: 5,
          character: 35,
        },
      },
      new_name: "roughness2".to_string(),
      work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = handle_rename(&mut server, params);
    assert!(result.is_some(), "Rename should produce edits");
    let edit = result.unwrap();
    let changes = edit.changes.expect("Should have changes");
    let edits = changes
      .get(&uri)
      .expect("Should have edits for the document");
    assert!(!edits.is_empty(), "Should have at least one edit");
  }

  #[test]
  fn test_integration_references_parameter() {
    let text = "material {\n    name : Test,\n    shadingModel : lit,\n    parameters : [\n        { type : float, name : roughness }\n    ]\n}\nfragment {\n    materialParams.roughness;\n}";
    let mut server = create_test_server(text);
    let uri = test_uri();

    // References uses `extract_word_at_position` (0-based) then
    // `parse_full_document` for the full MatFile.
    let params = ReferenceParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        position: Position {
          line: 4,
          character: 35,
        }, // 0-based: middle of "roughness"
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
      context: ReferenceContext {
        include_declaration: true,
      },
    };

    let result = handle_references(&mut server, params);
    assert!(result.is_some(), "References should find locations");
    let locations = result.unwrap();
    assert!(
      locations.len() >= 1,
      "Should find at least one reference, got {:?}",
      locations
    );
  }

  #[test]
  fn test_integration_document_symbol() {
    let text = "material {\n    name : TestMat,\n    shadingModel : lit,\n    parameters : [\n        { type : float, name : roughness }\n    ]\n}";
    let mut server = create_test_server(text);
    let uri = test_uri();

    let params = DocumentSymbolParams {
      text_document: TextDocumentIdentifier { uri },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
    };

    let result = handle_document_symbol(&mut server, params);
    assert!(result.is_some(), "Document symbols should be returned");
    match result.unwrap() {
      DocumentSymbolResponse::Nested(symbols) => {
        assert!(symbols.len() >= 1, "Should have at least material symbol");
        assert!(
          symbols[0].name.contains("TestMat"),
          "Symbol name should contain material name"
        );
      }
      _ => panic!("Expected Nested document symbols"),
    }
  }

  #[test]
  fn test_integration_formatting() {
    let text = "material {\nname : Test,\nshadingModel : lit\n}";
    let server = create_test_server(text);
    let uri = test_uri();

    let params = DocumentFormattingParams {
      text_document: TextDocumentIdentifier { uri },
      options: FormattingOptions::default(),
      work_done_progress_params: WorkDoneProgressParams::default(),
    };

    let result = handle_formatting(&server, params);
    assert!(result.is_some(), "Formatting should produce edits");
    let edits = result.unwrap();
    assert!(
      !edits.is_empty(),
      "Should have at least one formatting edit"
    );
    // The formatted text should have indentation
    assert!(
      edits[0].new_text.contains("    name"),
      "Formatted text should have indented properties"
    );
  }
}
