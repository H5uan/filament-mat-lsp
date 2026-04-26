use std::collections::HashMap;

use crate::parser::{MatFile, Material, ParseError, ShaderBlock};
use lsp_types::Uri;

/// Block-level cache for a parsed .mat file.
#[derive(Debug, Clone)]
pub struct BlockCache {
  pub version: i32,
  pub material: Result<Material, ParseError>,
  pub material_valid: bool,
  pub shaders: Vec<(ShaderBlock, bool)>,
  /// (start_line, end_line) for material block
  pub material_range: (u32, u32),
  /// (start_line, end_line) for each shader block
  pub shader_ranges: Vec<(u32, u32)>,
}

impl BlockCache {
  pub fn from_matfile(version: i32, matfile: MatFile) -> Self {
    let material_range = (
      matfile.material.range.start.line,
      matfile.material.range.end.line,
    );
    let shader_ranges = matfile
      .shaders
      .iter()
      .map(|s| (s.range.start.line, s.range.end.line))
      .collect();

    Self {
      version,
      material: Ok(matfile.material),
      material_valid: true,
      shaders: matfile.shaders.into_iter().map(|s| (s, true)).collect(),
      material_range,
      shader_ranges,
    }
  }

  /// Invalidate blocks affected by a change in the given line range.
  pub fn invalidate_blocks(&mut self, change_start_line: u32, change_end_line: u32) {
    // Check material block
    if ranges_overlap(
      change_start_line,
      change_end_line,
      self.material_range.0,
      self.material_range.1,
    ) {
      self.material_valid = false;
    }

    // Check shader blocks
    // If a shader block before the change is affected, it only invalidates that block
    // If a change spans multiple blocks or goes beyond a block, invalidate all subsequent blocks too
    let mut first_affected_shader: Option<usize> = None;

    for (i, (start, end)) in self.shader_ranges.iter().enumerate() {
      if ranges_overlap(change_start_line, change_end_line, *start, *end) {
        first_affected_shader = Some(i);
        break;
      }
    }

    if let Some(first) = first_affected_shader {
      // Invalidate this shader and all subsequent shaders
      // (because line numbers after the change may have shifted)
      for i in first..self.shaders.len() {
        self.shaders[i].1 = false;
      }
    }
  }

  /// Check if all blocks are valid.
  pub fn is_fully_valid(&self) -> bool {
    if !self.material_valid {
      return false;
    }
    for (_, valid) in &self.shaders {
      if !valid {
        return false;
      }
    }
    true
  }

  /// Check if only the material block is valid (no shaders or all shaders invalid).
  pub fn is_material_only_valid(&self) -> bool {
    self.material_valid && self.shaders.iter().all(|(_, v)| !v)
  }

  /// Reconstruct a MatFile from the cache.
  /// Returns None if not all blocks are valid.
  pub fn to_matfile(&self) -> Option<MatFile> {
    if !self.is_fully_valid() {
      return None;
    }

    let material: Material = match &self.material {
      Ok(m) => m.clone(),
      Err(_) => return None,
    };

    let shaders: Vec<ShaderBlock> = self.shaders.iter().map(|(s, _)| s.clone()).collect();

    Some(MatFile {
      material,
      shaders,
      errors: vec![],
    })
  }
}

fn ranges_overlap(a_start: u32, a_end: u32, b_start: u32, b_end: u32) -> bool {
  !(a_end < b_start || a_start > b_end)
}

/// Manager for block-level caches across all documents.
pub struct BlockCacheManager {
  caches: HashMap<Uri, BlockCache>,
}

impl BlockCacheManager {
  pub fn new() -> Self {
    Self {
      caches: HashMap::new(),
    }
  }

  pub fn get(&self, uri: &Uri) -> Option<&BlockCache> {
    self.caches.get(uri)
  }

  pub fn insert(&mut self, uri: Uri, cache: BlockCache) {
    self.caches.insert(uri, cache);
  }

  pub fn remove(&mut self, uri: &Uri) {
    self.caches.remove(uri);
  }

  /// Invalidate cache for a document if the change affects cached blocks.
  pub fn handle_change(
    &mut self,
    uri: &Uri,
    new_version: i32,
    change_start_line: u32,
    change_end_line: u32,
  ) {
    if let Some(cache) = self.caches.get_mut(uri) {
      cache.version = new_version;
      cache.invalidate_blocks(change_start_line, change_end_line);
    }
  }

  /// Clear all caches.
  pub fn clear_all(&mut self) {
    self.caches.clear();
  }
}

impl Default for BlockCacheManager {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostics::{TextPosition, TextRange};
  use crate::parser::{Located, Material, ShaderBlock, ShaderBlockType};

  fn make_range(start_line: u32, start_char: u32, end_line: u32, end_char: u32) -> TextRange {
    TextRange {
      start: TextPosition {
        line: start_line,
        character: start_char,
      },
      end: TextPosition {
        line: end_line,
        character: end_char,
      },
    }
  }

  fn create_test_matfile() -> MatFile {
    MatFile {
      material: Material {
        range: make_range(0, 0, 4, 1),
        name: Some(Located::new("Test".to_string(), make_range(1, 4, 1, 10))),
        shading_model: Some(Located::new("lit".to_string(), make_range(2, 4, 2, 10))),
        requires: Located::new(vec![], make_range(0, 0, 0, 0)),
        parameters: vec![],
        other_properties: vec![],
      },
      shaders: vec![
        ShaderBlock {
          block_type: ShaderBlockType::Vertex,
          code: "void main() {}".to_string(),
          range: make_range(5, 0, 8, 1),
        },
        ShaderBlock {
          block_type: ShaderBlockType::Fragment,
          code: "void material() {}".to_string(),
          range: make_range(9, 0, 12, 1),
        },
      ],
      errors: vec![],
    }
  }

  #[test]
  fn test_block_cache_creation() {
    let matfile = create_test_matfile();
    let cache = BlockCache::from_matfile(1, matfile);

    assert!(cache.material_valid);
    assert_eq!(cache.shaders.len(), 2);
    assert!(cache.shaders[0].1);
    assert!(cache.shaders[1].1);
    assert!(cache.is_fully_valid());
  }

  #[test]
  fn test_invalidate_material_block() {
    let matfile = create_test_matfile();
    let mut cache = BlockCache::from_matfile(1, matfile);

    // Change inside material block
    cache.invalidate_blocks(2, 2);

    assert!(!cache.material_valid);
    assert!(cache.shaders[0].1); // shaders unaffected
    assert!(cache.shaders[1].1);
  }

  #[test]
  fn test_invalidate_first_shader() {
    let matfile = create_test_matfile();
    let mut cache = BlockCache::from_matfile(1, matfile);

    // Change inside first shader block
    cache.invalidate_blocks(6, 6);

    assert!(cache.material_valid);
    assert!(!cache.shaders[0].1); // first shader invalidated
    assert!(!cache.shaders[1].1); // subsequent shaders also invalidated
  }

  #[test]
  fn test_invalidate_second_shader() {
    let matfile = create_test_matfile();
    let mut cache = BlockCache::from_matfile(1, matfile);

    // Change inside second shader block
    cache.invalidate_blocks(10, 10);

    assert!(cache.material_valid);
    assert!(cache.shaders[0].1); // first shader unaffected
    assert!(!cache.shaders[1].1); // second shader invalidated
  }

  #[test]
  fn test_to_matfile_fully_valid() {
    let matfile = create_test_matfile();
    let cache = BlockCache::from_matfile(1, matfile);

    let reconstructed = cache.to_matfile();
    assert!(reconstructed.is_some());
    let m = reconstructed.unwrap();
    assert_eq!(m.shaders.len(), 2);
  }

  #[test]
  fn test_to_matfile_partial() {
    let matfile = create_test_matfile();
    let mut cache = BlockCache::from_matfile(1, matfile);
    cache.invalidate_blocks(6, 6);

    // material is valid but shaders are not, so to_matfile returns None
    // because we can't reconstruct a partial MatFile
    let reconstructed = cache.to_matfile();
    assert!(reconstructed.is_none());
  }
}
