use lsp_server::{Connection, Message};
use lsp_types::*;

mod lsp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Set up logging
  eprintln!("Starting Filament Material LSP Server");

  let (connection, io_threads) = Connection::stdio();

  let server_capabilities = serde_json::to_value(&ServerCapabilities {
    text_document_sync: Some(TextDocumentSyncCapability::Options(
      TextDocumentSyncOptions {
        open_close: Some(true),
        change: Some(TextDocumentSyncKind::INCREMENTAL),
        ..Default::default()
      },
    )),
    completion_provider: Some(CompletionOptions {
      trigger_characters: Some(vec![":".into(), ",".into(), "{".into()]),
      ..Default::default()
    }),
    hover_provider: Some(HoverProviderCapability::Simple(true)),
    definition_provider: Some(OneOf::Left(true)),
    document_symbol_provider: Some(OneOf::Left(true)),
    diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
      identifier: Some("filament-mat".to_string()),
      inter_file_dependencies: false,
      workspace_diagnostics: false,
      work_done_progress_options: WorkDoneProgressOptions::default(),
    })),
    code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
    semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
      SemanticTokensOptions {
        work_done_progress_options: WorkDoneProgressOptions::default(),
        legend: SemanticTokensLegend {
          token_types: lsp::semantic_tokens::token_types()
            .into_iter()
            .map(SemanticTokenType::new)
            .collect(),
          token_modifiers: vec![],
        },
        range: Some(false),
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(false) }),
      },
    )),
    document_formatting_provider: Some(OneOf::Left(true)),
    workspace_symbol_provider: Some(OneOf::Left(true)),
    rename_provider: Some(OneOf::Right(RenameOptions {
      prepare_provider: Some(true),
      work_done_progress_options: WorkDoneProgressOptions::default(),
    })),
    ..Default::default()
  })?;

  let _init_params = connection.initialize(server_capabilities)?;
  eprintln!("Server initialized successfully");

  let mut server = lsp::server::ServerState::new(connection.sender.clone());

  for msg in &connection.receiver {
    match msg {
      Message::Request(req) => {
        if connection.handle_shutdown(&req)? {
          eprintln!("Shutting down...");
          break;
        }
        if let Err(e) = lsp::handlers::handle_request(&mut server, req, &connection.sender) {
          eprintln!("Error handling request: {}", e);
        }
      }
      Message::Notification(not) => {
        if let Err(e) = lsp::handlers::handle_notification(&mut server, not) {
          eprintln!("Error handling notification: {}", e);
        }
      }
      _ => {}
    }
  }

  io_threads.join()?;
  eprintln!("Server stopped");
  Ok(())
}
