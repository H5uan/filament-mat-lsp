# Filament Material LSP

VS Code Language Server Protocol support for Google Filament material files (`.mat`).

Provides intelligent editing features including completions, hover documentation, diagnostics, semantic highlighting, formatting, and more.

---

## Features

### Core LSP Features

- ✅ **Syntax Highlighting** — TextMate grammar for `.mat` files
- ✅ **Code Completion** — Context-aware suggestions with **snippet support** for:
  - 40+ material properties (`shadingModel`, `blending`, `parameters`, etc.)
  - 60+ enum values (`lit`, `unlit`, `opaque`, `transparent`, etc.)
  - Parameter types (`float4`, `sampler2d`, `mat4`, etc.)
  - Vertex attributes (`position`, `uv0`, `normal`, etc.)
- ✅ **Hover Documentation** — Docs for properties, enum values, and Filament shader API
- ✅ **Diagnostics** — Validation with **300ms debounce** for smooth typing:
  - Missing required properties (`name`, `shadingModel`)
  - Invalid enum values
  - Unknown material properties
  - Invalid parameter types
- ✅ **Code Actions** — Quick fixes for missing required properties
- ✅ **Go to Definition** — Navigate to parameter definitions
- ✅ **Document Symbols** — Outline view with material and parameter structure
- ✅ **Workspace Symbols** — Search across all materials in workspace
- ✅ **Rename Refactoring** — Rename parameters and sync shader references (`materialParams_xxx`)

### Navigation & Intelligence

- ✅ **Find All References** — Locate all usages of a symbol across shader blocks
- ✅ **Document Highlight** — Highlight all occurrences of the same symbol in the editor
- ✅ **Document Links** — Clickable links from `shadingModel`/`blendMode` values to [Filament documentation](https://google.github.io/filament/Materials.html)
- ✅ **Code Lens** — Reference counts displayed above parameter definitions (click to find references)
- ✅ **Selection Ranges** — Smart expand/shrink selection with semantic awareness
- ✅ **Folding Ranges** — Collapse/expand material blocks, shader blocks, and parameter arrays

### Editor Assistance

- ✅ **Signature Help** — Inline function signatures for Filament shader APIs (`prepareMaterial`, `getUV0`, etc.)
- ✅ **Inlay Hints** — Inline type annotations inferred from parameter usage
- ✅ **Semantic Highlighting** — Token-based syntax coloring with **delta updates** for performance
- ✅ **Color Preview** — Inline color swatches for `vec3`/`vec4`/`float3`/`float4` values in shaders
- ✅ **On-Type Formatting** — Auto-indent on `}`, `;`, `,`, `:` for real-time formatting

### Formatting

- ✅ **Document Formatting** — Full-file auto-indent with GLSL block preservation
- ✅ **Range Formatting** — Format selected lines only
- ✅ **On-Type Formatting** — Indentation adjusts as you type closing braces

### Custom Commands

- ✅ **Compile Material** (`Filament Material: Compile Material`) — Run `matc` on the current `.mat` file (configurable path)
- ✅ **Show Documentation** (`Filament Material: Show Documentation`) — Open Filament docs for the symbol under cursor

### Performance

- ✅ **AST Block Cache** — Per-block invalidation for fast editing in large files (>1000 lines)
- ✅ **Debounced Diagnostics** — 300ms delay to avoid blocking the UI while typing
- ✅ **Error Recovery** — Parser continues after syntax errors to provide partial LSP support

---

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (nightly toolchain)
- VS Code 1.75+

### Build

```bash
# Install dependencies
npm install

# Build everything (Rust + TypeScript)
npm run compile

# Or build separately
npm run compile:rust   # Build LSP server binary
npm run compile:ts     # Compile VS Code client
```

### Development

```bash
# Watch TypeScript files
npm run watch

# Run Rust tests
cd native && cargo test --all

# Run full CI checks locally
npm run check
```

### Launch Extension

1. Open `filament-mat-lsp/` in VS Code
2. Press `F5` to launch Extension Development Host
3. Open any `.mat` file (see `test/` for examples)
4. Try completions, hover, diagnostics, and formatting

---

## Project Structure

```
filament-mat-lsp/
├── .github/workflows/     # CI: Rust fmt/clippy/test + TypeScript check
├── client/
│   ├── src/
│   │   ├── extension.ts   # VS Code extension entry point
│   │   └── test/          # E2E test suite (@vscode/test-electron)
│   └── package.json
├── native/                # Rust LSP server (standalone stdio binary)
│   ├── src/
│   │   ├── main.rs              # LSP server entry point
│   │   ├── lib.rs               # Public API exports
│   │   ├── lexer.rs             # Two-tier lexer (Material + Shader blocks)
│   │   ├── parser.rs            # AST parser with error recovery
│   │   ├── schema.rs            # Central schema (properties, enums, types)
│   │   ├── completion.rs        # Completion engine with snippets
│   │   ├── diagnostics.rs       # Validation engine
│   │   ├── hover.rs             # Hover documentation engine
│   │   ├── references.rs        # Find references + document highlight
│   │   ├── signature_help.rs    # Function signature help
│   │   ├── selection_range.rs   # Smart selection ranges
│   │   ├── inlay_hints.rs       # Inline type hints
│   │   ├── block_cache.rs       # AST block-level cache
│   │   ├── color_provider.rs    # Color preview in shaders
│   │   └── lsp/
│   │       ├── server.rs              # Document storage + block cache + debounce
│   │       ├── handlers.rs            # LSP request handlers
│   │       ├── semantic_tokens.rs     # Semantic highlighting with delta
│   │       └── conv.rs                # Type conversions
│   └── Cargo.toml
├── syntaxes/              # TextMate grammar for .mat files
├── test/                  # Sample .mat files for testing
└── package.json
```

---

## Example Material File

```mat
material {
    name : TexturedLit,
    requires : [position, normal, uv0],
    shadingModel : lit,
    blending : opaque,
    culling : back,

    parameters : [
        { type : float4, name : baseColor },
        { type : sampler2d, name : albedoMap },
        { type : sampler2d, name : normalMap },
        { type : float, name : roughness },
        { type : float, name : metallic }
    ]
}

vertex {
    void materialVertex(inout MaterialVertexInputs material) {
        // Vertex shader code
    }
}

fragment {
    void material(inout MaterialInputs material) {
        prepareMaterial(material);
        material.baseColor = texture(materialParams_albedoMap, getUV0());
        material.roughness = materialParams.roughness;
        material.metallic = materialParams.metallic;
    }
}
```

---

## Supported Material Properties

The LSP recognizes all properties defined in the [Filament Materials documentation](https://google.github.io/filament/Materials.html):

| Category | Properties |
|----------|-----------|
| **Core** | `name`, `shadingModel`, `requires`, `parameters`, `constants`, `variables` |
| **Rendering** | `blending`, `postLightingBlending`, `transparency`, `maskThreshold`, `alphaToCoverage` |
| **Geometry** | `vertexDomain`, `culling`, `doubleSided`, `instanced` |
| **Depth** | `colorWrite`, `depthWrite`, `depthCulling` |
| **Lighting** | `refractionMode`, `refractionType`, `reflections`, `shadowMultiplier` |
| **Quality** | `quality`, `specularAmbientOcclusion`, `specularAntiAliasing`, `variantFilter` |
| **Advanced** | `customSurfaceShading`, `flipUV`, `framebufferFetch`, `stereoscopicType` |

---

## Configuration

The extension contributes the following VS Code settings:

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `filamentMat.matcPath` | `string` | `"matc"` | Path to the Filament material compiler (`matc`) |

## Commands

Open the Command Palette (`Ctrl+Shift+P`) and type:

- **Filament Material: Compile Material** — Compiles the current `.mat` file using `matc`
- **Filament Material: Show Documentation** — Opens Filament documentation for the symbol under cursor

## CI / CD

GitHub Actions workflow (`.github/workflows/ci.yml`):

- **Rust checks**: `cargo fmt`, `cargo clippy`, `cargo test` on Windows
- **TypeScript check**: `tsc --noEmit`
- **Release builds**: Cross-platform binaries for Windows, macOS, Linux on tag push

---

## License

MIT
