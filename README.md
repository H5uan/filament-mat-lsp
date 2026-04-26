# Filament Material LSP

VS Code Language Server Protocol support for Google Filament material files (`.mat`).

Provides intelligent editing features including completions, hover documentation, diagnostics, semantic highlighting, formatting, and more.

---

## Features

### Core LSP Features

- ✅ **Syntax Highlighting** — TextMate grammar for `.mat` files
- ✅ **Code Completion** — Context-aware suggestions for:
  - 40+ material properties (`shadingModel`, `blending`, `parameters`, etc.)
  - 60+ enum values (`lit`, `unlit`, `opaque`, `transparent`, etc.)
  - Parameter types (`float4`, `sampler2d`, `mat4`, etc.)
  - Vertex attributes (`position`, `uv0`, `normal`, etc.)
- ✅ **Hover Documentation** — Docs for properties, enum values, and Filament shader API
- ✅ **Diagnostics** — Validation for:
  - Missing required properties (`name`, `shadingModel`)
  - Invalid enum values
  - Unknown material properties
  - Invalid parameter types
- ✅ **Code Actions** — Quick fixes for missing required properties
- ✅ **Go to Definition** — Navigate to parameter definitions
- ✅ **Document Symbols** — Outline view with material and parameter structure
- ✅ **Workspace Symbols** — Search across all materials in workspace
- ✅ **Rename Refactoring** — Rename parameters and sync shader references (`materialParams_xxx`)

### Advanced Features

- ✅ **Semantic Highlighting** — Token-based syntax coloring for properties, enums, types
- ✅ **Document Formatting** — Auto-indent `.mat` files with GLSL block preservation
- ✅ **Shader API Hover** — Documentation for Filament GLSL APIs (`MaterialInputs`, `prepareMaterial`, `getUV0`, etc.)
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
│   │   ├── main.rs        # LSP server entry point
│   │   ├── lib.rs         # Public API exports
│   │   ├── lexer.rs       # Two-tier lexer (Material + Shader blocks)
│   │   ├── parser.rs      # AST parser with error recovery
│   │   ├── schema.rs      # Central schema (properties, enums, types)
│   │   ├── completion.rs  # Completion engine
│   │   ├── diagnostics.rs # Validation engine
│   │   ├── hover.rs       # Hover documentation engine
│   │   └── lsp/
│   │       ├── server.rs      # Document storage + AST cache
│   │       ├── handlers.rs    # LSP request handlers
│   │       ├── semantic_tokens.rs  # Semantic highlighting
│   │       └── conv.rs        # Type conversions
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

## CI / CD

GitHub Actions workflow (`.github/workflows/ci.yml`):

- **Rust checks**: `cargo fmt`, `cargo clippy`, `cargo test` on Windows
- **TypeScript check**: `tsc --noEmit`
- **Release builds**: Cross-platform binaries for Windows, macOS, Linux on tag push

---

## License

MIT
