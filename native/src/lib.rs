pub mod completion;
pub mod diagnostics;
pub mod hover;
pub mod lexer;
pub mod parser;
pub mod schema;
pub mod token;

// Re-export commonly used types
pub use completion::{CompletionContext, CompletionEngine, CompletionItem, CompletionItemKind};
pub use diagnostics::{Diagnostic, DiagnosticSeverity, TextPosition, TextRange, Validator};
pub use lexer::Lexer;
pub use parser::{MatFile, Material, Parameter, ShaderBlock, ShaderBlockType, Value};
pub use token::{Token, TokenExt, TokenType};
