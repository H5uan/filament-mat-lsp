# Filament Material LSP

VS Code Language Server Protocol support for Google Filament material files (.mat).

## Features

- ✅ **Syntax Highlighting** for Filament material files
- ✅ **Code Completion** for:
  - Material properties
  - Shading model values
  - Blending mode values
  - Parameter types
  - Required vertex attributes
- ✅ **Basic Validation** for required properties (name and shadingModel)

## Getting Started

1. **Build the project**
   ```bash
   npm install
   npm run compile
   ```

2. **Test in VS Code**
   - Open this repository in VS Code
   - Press `F5` to launch the Extension Development Host
   - Create a new file with `.mat` extension
   - Start writing Filament material files!

3. **Test Rust core** (optional)
   ```bash
   cd native
   cargo test
   ```

## Project Structure

```
filament-mat-lsp/
├── .github/workflows/  # GitHub Actions CI
├── client/             # VS Code Language Client
├── server/             # TypeScript LSP Server
├── native/             # Rust core library
│   ├── src/
│   │   ├── lib.rs         # Node.js bindings
│   │   ├── token.rs       # Token types
│   │   ├── lexer.rs       # Lexer
│   │   ├── parser.rs      # Parser
│   │   ├── completion.rs  # Completion engine
│   │   └── diagnostics.rs # Diagnostics engine
├── syntaxes/           # TextMate grammar
├── language-configuration.json
└── package.json
```

## Filament Material Example

```mat
material {
    name: MyMaterial,
    shadingModel: lit,
    requires: [position, normal, uv0],

    parameters: [
        { type: float4, name: baseColor },
        { type: sampler2d, name: baseColorMap },
    ],

    blending: opaque,
    culling: back,
}

vertex {
    // Vertex shader
}

fragment {
    // Fragment shader
}
```

## License

MIT
