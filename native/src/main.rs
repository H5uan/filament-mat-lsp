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
        full: Some(SemanticTokensFullOptions::Delta { delta: Some(true) }),
      },
    )),
    document_formatting_provider: Some(OneOf::Left(true)),
    document_range_formatting_provider: Some(OneOf::Left(true)),
    workspace_symbol_provider: Some(OneOf::Left(true)),
    rename_provider: Some(OneOf::Right(RenameOptions {
      prepare_provider: Some(true),
      work_done_progress_options: WorkDoneProgressOptions::default(),
    })),
    document_highlight_provider: Some(OneOf::Left(true)),
    folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
    references_provider: Some(OneOf::Left(true)),
    signature_help_provider: Some(SignatureHelpOptions {
      trigger_characters: Some(vec!["(".into()]),
      retrigger_characters: Some(vec![",".into()]),
      work_done_progress_options: WorkDoneProgressOptions::default(),
    }),
    selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
    inlay_hint_provider: Some(OneOf::Right(InlayHintServerCapabilities::Options(
      InlayHintOptions {
        resolve_provider: Some(false),
        work_done_progress_options: WorkDoneProgressOptions::default(),
      },
    ))),
    color_provider: Some(ColorProviderCapability::Simple(true)),
    document_on_type_formatting_provider: Some(DocumentOnTypeFormattingOptions {
      first_trigger_character: "}".to_string(),
      more_trigger_character: Some(vec![";".to_string(), ",".to_string(), ":".to_string()]),
    }),
    document_link_provider: Some(DocumentLinkOptions {
      resolve_provider: Some(false),
      work_done_progress_options: WorkDoneProgressOptions::default(),
    }),
    code_lens_provider: Some(CodeLensOptions {
      resolve_provider: Some(false),
    }),
    ..Default::default()
  })?;

  let _init_params = connection.initialize(server_capabilities)?;
  eprintln!("Server initialized successfully");

  let mut server = lsp::server::ServerState::new(connection.sender.clone());

  const DIAGNOSTICS_DEBOUNCE_MS: u64 = 300;
  const POLL_INTERVAL_MS: u64 = 50;

  loop {
    // Try to receive a message with a timeout so we can periodically check
    // for pending diagnostics.
    let msg = match connection
      .receiver
      .recv_timeout(std::time::Duration::from_millis(POLL_INTERVAL_MS))
    {
      Ok(msg) => msg,
      Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
        // No message received within the poll interval.
        // Check for pending diagnostics that have exceeded the debounce delay.
        let now = std::time::Instant::now();
        let debounce = std::time::Duration::from_millis(DIAGNOSTICS_DEBOUNCE_MS);
        let mut ready = Vec::new();

        for (uri, (version, instant)) in &server.pending_diagnostics {
          if now.duration_since(*instant) >= debounce {
            // Verify the document version hasn't changed since we queued it.
            if let Some(doc) = server.get_document(uri)
              && doc.version == *version
            {
              ready.push(uri.clone());
            }
          }
        }

        for uri in ready {
          server.pending_diagnostics.remove(&uri);
          let diagnostics = lsp::handlers::compute_diagnostics(&mut server, &uri);
          if let Err(e) = lsp::handlers::publish_diagnostics(&uri, diagnostics, &server) {
            eprintln!("Error publishing diagnostics: {}", e);
          }
        }

        continue;
      }
      Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
        eprintln!("Channel disconnected, shutting down...");
        break;
      }
    };

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
