# Changelog

All notable changes to the Filament Material LSP extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-06-28

### Added

Phase 1 quick wins:

- **Debounced diagnostics**: diagnostics are now computed 300 ms after the last edit instead of on every keystroke.
- **Completion filtering + snippets**: completions are filtered by the typed prefix; properties expand into snippets with tab stops and enum choices.
- **Document highlight**: cursor on a parameter highlights all `materialParams.name` and `materialParams_name` occurrences.
- **Folding ranges**: top-level blocks (`material`, `vertex`, `fragment`, `compute`, `tool`) and parameter arrays can be folded.

Phase 2 core navigation features:

- **Find all references**: lists every use of a parameter across material and shader blocks.
- **Signature help**: shows signatures for common Filament shader functions such as `prepareMaterial` and `getUV0`.
- **Selection ranges**: smart expand selection from identifier → value → key/value pair → block.
- **Inlay hints**: parameter definitions show inferred type hints.

Phase 3 performance & visuals:

- **Block-level AST cache**: editing a shader block no longer invalidates the parsed material block.
- **Incremental semantic tokens**: `textDocument/semanticTokens/full/delta` is supported and cached.
- **Range formatting**: format only the selected range while preserving GLSL shader code.
- **Color provider**: color squares and picker support for `vec3`/`vec4`/`float3`/`float4` color values in shader code.

Phase 4 polish:

- **On-type formatting**: closing brace `}` is automatically dedented.
- **Document links**: `shadingModel` and `blendMode` values link to the official Filament documentation.
- **Code lens**: parameter definitions show reference counts.
- **Custom VS Code commands**: `filamentMat.compile` invokes `matc`, and `filamentMat.showDocumentation` opens the Filament docs.

### Configuration

- Added `filamentMat.diagnosticsDelay` (default: 300 ms).
- Added `filamentMat.matcPath` (default: `matc`).

## [0.0.1] - 2026-04-26

### Added

- Initial LSP support for Filament `.mat` files.
- Completions, hover, go-to-definition, diagnostics, document symbols, workspace symbols, rename, code actions, formatting, and semantic tokens.
