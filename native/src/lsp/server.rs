use crossbeam_channel::Sender;
use lsp_server::Message;
use lsp_types::Uri;
use std::collections::HashMap;
use std::time::Instant;

use filament_mat_lsp::block_cache::BlockCacheManager;
use filament_mat_lsp::parser::{Material, ParseError};
use lsp_types::SemanticToken;

#[derive(Debug, Clone)]
pub struct Document {
  pub text: String,
  pub version: i32,
  line_offsets: Vec<usize>,
}

impl Document {
  pub fn new(text: String, version: i32) -> Self {
    let line_offsets = compute_line_offsets(&text);
    Self {
      text,
      version,
      line_offsets,
    }
  }

  fn recompute_offsets(&mut self) {
    self.line_offsets = compute_line_offsets(&self.text);
  }

  pub fn position_to_offset(&self, position: lsp_types::Position) -> usize {
    let line = position.line as usize;
    if line >= self.line_offsets.len() {
      return self.text.len();
    }

    let line_start = self.line_offsets[line];
    let line_end = if line + 1 < self.line_offsets.len() {
      self.line_offsets[line + 1]
    } else {
      self.text.len()
    };

    let line_text = &self.text[line_start..line_end];
    let mut byte_offset = line_start;
    let mut utf16_count = 0;

    for c in line_text.chars() {
      if utf16_count >= position.character as usize {
        break;
      }
      if c == '\n' || c == '\r' {
        break;
      }
      utf16_count += c.len_utf16();
      byte_offset += c.len_utf8();
    }

    byte_offset
  }
}

fn compute_line_offsets(text: &str) -> Vec<usize> {
  let mut offsets = vec![0];
  let mut offset = 0;

  for c in text.chars() {
    offset += c.len_utf8();
    if c == '\n' {
      offsets.push(offset);
    }
  }

  offsets
}

/// Cache entry for semantic tokens.
#[derive(Debug, Clone)]
pub struct SemanticTokensCache {
  pub version: i32,
  pub result_id: String,
  pub data: Vec<SemanticToken>,
}

pub struct ServerState {
  pub documents: HashMap<Uri, Document>,
  sender: Sender<Message>,
  /// Block-level cache for parsed ASTs.
  block_cache_manager: BlockCacheManager,
  /// Cache for semantic tokens to support incremental updates.
  semantic_tokens_cache: HashMap<Uri, SemanticTokensCache>,
  /// Documents with pending diagnostic computation.
  /// Key: document URI
  /// Value: (version, last_change_instant)
  pub pending_diagnostics: HashMap<Uri, (i32, Instant)>,
}

impl ServerState {
  pub fn new(sender: Sender<Message>) -> Self {
    Self {
      documents: HashMap::new(),
      sender,
      block_cache_manager: BlockCacheManager::new(),
      semantic_tokens_cache: HashMap::new(),
      pending_diagnostics: HashMap::new(),
    }
  }

  pub fn get_document(&self, uri: &Uri) -> Option<&Document> {
    self.documents.get(uri)
  }

  pub fn insert_document(&mut self, uri: Uri, document: Document) {
    self.documents.insert(uri, document);
  }

  pub fn remove_document(&mut self, uri: &Uri) {
    self.documents.remove(uri);
    self.block_cache_manager.remove(uri);
  }

  pub fn apply_change(
    &mut self,
    uri: &Uri,
    change: lsp_types::TextDocumentContentChangeEvent,
    version: i32,
  ) {
    if let Some(doc) = self.documents.get_mut(uri) {
      let change_start_line: u32;
      let change_end_line: u32;

      if let Some(range) = change.range {
        // Incremental change
        let start_offset = doc.position_to_offset(range.start);
        let end_offset = doc.position_to_offset(range.end);
        let mut new_text = doc.text[..start_offset].to_string();
        new_text.push_str(&change.text);
        new_text.push_str(&doc.text[end_offset..]);
        doc.text = new_text;

        change_start_line = range.start.line;
        change_end_line = range.end.line;
      } else {
        // Full document replacement
        doc.text = change.text;
        change_start_line = 0;
        change_end_line = u32::MAX;
      }
      doc.version = version;
      doc.recompute_offsets();

      // Block-level cache invalidation
      self
        .block_cache_manager
        .handle_change(uri, version, change_start_line, change_end_line);
    }
  }

  pub fn send(&self, msg: Message) -> Result<(), crossbeam_channel::SendError<Message>> {
    self.sender.send(msg)
  }

  pub fn parse_document(&mut self, uri: &Uri) -> Option<Result<Material, ParseError>> {
    let doc = self.documents.get(uri)?;
    let version = doc.version;

    // Try block cache first
    if let Some(cache) = self.block_cache_manager.get(uri)
      && cache.version == version
      && cache.material_valid
    {
      return cache.material.as_ref().ok().map(|m| Ok(m.clone()));
    }

    // Fall back to full re-parse
    let text = doc.text.clone();
    let result = self.parse_text(&text);

    // Update block cache
    let matfile = self.parse_full_text(&text);
    let block_cache = filament_mat_lsp::block_cache::BlockCache::from_matfile(version, matfile);
    self.block_cache_manager.insert(uri.clone(), block_cache);

    Some(result)
  }

  pub fn parse_full_document(&mut self, uri: &Uri) -> Option<filament_mat_lsp::parser::MatFile> {
    let doc = self.documents.get(uri)?;
    let version = doc.version;

    // Try block cache first
    if let Some(cache) = self.block_cache_manager.get(uri)
      && cache.version == version
      && cache.is_fully_valid()
    {
      return cache.to_matfile();
    }

    // Fall back to full re-parse
    let text = doc.text.clone();
    let matfile = self.parse_full_text(&text);

    // Update block cache
    let block_cache =
      filament_mat_lsp::block_cache::BlockCache::from_matfile(version, matfile.clone());
    self.block_cache_manager.insert(uri.clone(), block_cache);

    Some(matfile)
  }

  fn parse_text(&self, text: &str) -> Result<Material, ParseError> {
    use filament_mat_lsp::lexer::Lexer;
    use filament_mat_lsp::parser::Parser;

    let mut lexer = Lexer::new(text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_material()
  }

  fn parse_full_text(&self, text: &str) -> filament_mat_lsp::parser::MatFile {
    use filament_mat_lsp::lexer::Lexer;
    use filament_mat_lsp::parser::Parser;

    let mut lexer = Lexer::new(text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse()
  }

  /// Get semantic tokens for a document, using cache if available.
  pub fn get_semantic_tokens(&mut self, uri: &Uri) -> Option<(String, Vec<SemanticToken>)> {
    let doc = self.documents.get(uri)?;
    let version = doc.version;

    // Check cache
    if let Some(cache) = self.semantic_tokens_cache.get(uri)
      && cache.version == version
    {
      return Some((cache.result_id.clone(), cache.data.clone()));
    }

    // Generate new tokens
    let data = super::semantic_tokens::generate_semantic_tokens(&doc.text);
    let result_id = format!("{}", version);

    // Update cache
    self.semantic_tokens_cache.insert(
      uri.clone(),
      SemanticTokensCache {
        version,
        result_id: result_id.clone(),
        data: data.clone(),
      },
    );

    Some((result_id, data))
  }

  /// Get semantic tokens delta for a document.
  /// Returns (result_id, is_delta, data_or_edits).
  pub fn get_semantic_tokens_delta(
    &mut self,
    uri: &Uri,
    previous_result_id: &str,
  ) -> Option<(
    String,
    bool,
    Vec<SemanticToken>,
    Vec<lsp_types::SemanticTokensEdit>,
  )> {
    let doc = self.documents.get(uri)?;
    let version = doc.version;

    // Check if previous result is still valid
    if let Some(cache) = self.semantic_tokens_cache.get(uri)
      && cache.result_id == previous_result_id
      && cache.version == version
    {
      // No changes, return empty delta
      return Some((previous_result_id.to_string(), true, vec![], vec![]));
    }

    // Generate new tokens
    let new_data = super::semantic_tokens::generate_semantic_tokens(&doc.text);
    let result_id = format!("{}", version);

    // For simplicity, we return full data as a single edit
    // Client can handle receiving full data in delta response
    let edits = vec![lsp_types::SemanticTokensEdit {
      start: 0,
      delete_count: 0,
      data: Some(new_data.clone()),
    }];

    // Update cache
    self.semantic_tokens_cache.insert(
      uri.clone(),
      SemanticTokensCache {
        version,
        result_id: result_id.clone(),
        data: new_data.clone(),
      },
    );

    Some((result_id, true, new_data, edits))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::str::FromStr;

  #[test]
  fn test_document_new() {
    let doc = Document::new("hello\nworld".to_string(), 1);
    assert_eq!(doc.text, "hello\nworld");
    assert_eq!(doc.version, 1);
    assert_eq!(doc.line_offsets, vec![0, 6]);
  }

  #[test]
  fn test_position_to_offset() {
    let doc = Document::new("hello\nworld".to_string(), 1);
    // Line 0, char 0 -> offset 0
    assert_eq!(
      doc.position_to_offset(lsp_types::Position {
        line: 0,
        character: 0
      }),
      0
    );
    // Line 0, char 3 -> offset 3
    assert_eq!(
      doc.position_to_offset(lsp_types::Position {
        line: 0,
        character: 3
      }),
      3
    );
    // Line 1, char 0 -> offset 6
    assert_eq!(
      doc.position_to_offset(lsp_types::Position {
        line: 1,
        character: 0
      }),
      6
    );
    // Line 1, char 2 -> offset 8
    assert_eq!(
      doc.position_to_offset(lsp_types::Position {
        line: 1,
        character: 2
      }),
      8
    );
  }

  #[test]
  fn test_apply_change_full() {
    let (sender, _) = crossbeam_channel::unbounded();
    let mut server = ServerState::new(sender);
    let uri = lsp_types::Uri::from_str("file:///test.mat").unwrap();

    server.insert_document(uri.clone(), Document::new("hello".to_string(), 1));
    server.apply_change(
      &uri,
      lsp_types::TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "world".to_string(),
      },
      2,
    );

    let doc = server.get_document(&uri).unwrap();
    assert_eq!(doc.text, "world");
    assert_eq!(doc.version, 2);
  }

  #[test]
  fn test_apply_change_incremental() {
    let (sender, _) = crossbeam_channel::unbounded();
    let mut server = ServerState::new(sender);
    let uri = lsp_types::Uri::from_str("file:///test.mat").unwrap();

    server.insert_document(uri.clone(), Document::new("hello world".to_string(), 1));
    server.apply_change(
      &uri,
      lsp_types::TextDocumentContentChangeEvent {
        range: Some(lsp_types::Range {
          start: lsp_types::Position {
            line: 0,
            character: 6,
          },
          end: lsp_types::Position {
            line: 0,
            character: 11,
          },
        }),
        range_length: None,
        text: "rust".to_string(),
      },
      2,
    );

    let doc = server.get_document(&uri).unwrap();
    assert_eq!(doc.text, "hello rust");
    assert_eq!(doc.version, 2);
  }
}
