use crossbeam_channel::Sender;
use lsp_server::Message;
use lsp_types::Url;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Document {
  pub text: String,
  pub version: i32,
}

pub struct ServerState {
  pub documents: HashMap<Url, Document>,
  sender: Sender<Message>,
}

impl ServerState {
  pub fn new(sender: Sender<Message>) -> Self {
    Self {
      documents: HashMap::new(),
      sender,
    }
  }

  pub fn get_document(&self, uri: &Url) -> Option<&Document> {
    self.documents.get(uri)
  }

  pub fn insert_document(&mut self, uri: Url, document: Document) {
    self.documents.insert(uri, document);
  }

  pub fn remove_document(&mut self, uri: &Url) {
    self.documents.remove(uri);
  }

  pub fn apply_change(
    &mut self,
    uri: &Url,
    change: lsp_types::TextDocumentContentChangeEvent,
    version: i32,
  ) {
    if let Some(doc) = self.documents.get_mut(uri) {
      if let Some(range) = change.range {
        // Incremental change
        let start_offset = position_to_offset(&doc.text, range.start);
        let end_offset = position_to_offset(&doc.text, range.end);
        let mut new_text = doc.text[..start_offset].to_string();
        new_text.push_str(&change.text);
        new_text.push_str(&doc.text[end_offset..]);
        doc.text = new_text;
      } else {
        // Full document replacement
        doc.text = change.text;
      }
      doc.version = version;
    }
  }

  pub fn send(&self, msg: Message) -> Result<(), crossbeam_channel::SendError<Message>> {
    self.sender.send(msg)
  }
}

fn position_to_offset(text: &str, position: lsp_types::Position) -> usize {
  let mut offset = 0;
  let mut line = 0;
  let mut character = 0;

  for c in text.chars() {
    if line == position.line && character == position.character {
      break;
    }
    if c == '\n' {
      line += 1;
      character = 0;
    } else {
      character += 1;
    }
    offset += c.len_utf8();
  }

  offset
}
