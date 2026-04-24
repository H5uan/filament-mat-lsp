use crossbeam_channel::Sender;
use lsp_server::Message;
use lsp_types::Uri;
use std::collections::HashMap;

use filament_mat_lsp::parser::{Material, ParseError};

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

    for (char_count, c) in line_text.chars().enumerate() {
      if char_count >= position.character as usize {
        break;
      }
      if c == '\n' || c == '\r' {
        break;
      }
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

pub struct ServerState {
  pub documents: HashMap<Uri, Document>,
  sender: Sender<Message>,
  ast_cache: HashMap<Uri, (i32, Result<Material, ParseError>)>,
}

impl ServerState {
  pub fn new(sender: Sender<Message>) -> Self {
    Self {
      documents: HashMap::new(),
      sender,
      ast_cache: HashMap::new(),
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
    self.ast_cache.remove(uri);
  }

  pub fn apply_change(
    &mut self,
    uri: &Uri,
    change: lsp_types::TextDocumentContentChangeEvent,
    version: i32,
  ) {
    if let Some(doc) = self.documents.get_mut(uri) {
      if let Some(range) = change.range {
        // Incremental change
        let start_offset = doc.position_to_offset(range.start);
        let end_offset = doc.position_to_offset(range.end);
        let mut new_text = doc.text[..start_offset].to_string();
        new_text.push_str(&change.text);
        new_text.push_str(&doc.text[end_offset..]);
        doc.text = new_text;
      } else {
        // Full document replacement
        doc.text = change.text;
      }
      doc.version = version;
      doc.recompute_offsets();
      self.ast_cache.remove(uri);
    }
  }

  pub fn send(&self, msg: Message) -> Result<(), crossbeam_channel::SendError<Message>> {
    self.sender.send(msg)
  }

  pub fn parse_document(&mut self, uri: &Uri) -> Option<Result<Material, ParseError>> {
    let doc = self.documents.get(uri)?;
    let version = doc.version;

    if let Some((cached_version, cached_ast)) = self.ast_cache.get(uri)
      && *cached_version == version
    {
      return Some(cached_ast.clone());
    }

    let text = doc.text.clone();
    let result = self.parse_text(&text);
    self
      .ast_cache
      .insert(uri.clone(), (version, result.clone()));
    Some(result)
  }

  fn parse_text(&self, text: &str) -> Result<Material, ParseError> {
    use filament_mat_lsp::lexer::JsonishLexer;
    use filament_mat_lsp::parser::Parser;

    let mut lexer = JsonishLexer::new(text);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse_material()
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
