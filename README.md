# BerryEditor - 100% Rust Code Editor

A fully-featured code editor built entirely in Rust using Leptos and WebAssembly.

## Features

- 🦀 **100% Rust** - No JavaScript required
- 🚀 **WASM-powered** - Runs natively in the browser
- 🎨 **Syntax Highlighting** - Support for Rust, JavaScript, Python, and more
- 📁 **File Tree** - Navigate project files
- 🔍 **Search & Replace** - Powerful text search with regex support
- 🗺️ **Minimap** - Code overview navigation
- 📝 **Multi-cursor** - Edit multiple locations simultaneously
- 🔧 **LSP Support** - Code intelligence via Language Server Protocol
- 🌳 **Git Integration** - View diffs and manage changes

## Development

### Prerequisites

- Rust toolchain (stable)
- `trunk` for building and serving
- `wasm-pack` for testing

### Install Trunk

```bash
cargo install trunk
```

### Run Development Server

```bash
trunk serve
```

Then open http://127.0.0.1:8080/berry-editor/

### Run Tests

```bash
wasm-pack test --headless --chrome
```

## Architecture

- **Leptos 0.7** - Reactive UI framework
- **Ropey** - Efficient rope-based text buffer
- **Web-sys** - Direct browser API bindings
- **wasm-bindgen** - Rust/WASM/JavaScript interop

## Project Structure

```
gui-editor/
├── src/
│   ├── lib.rs           # WASM entry point
│   ├── main.rs          # Application entry
│   ├── components.rs    # UI components
│   ├── editor.rs        # Editor panel
│   ├── file_tree.rs     # File explorer
│   ├── buffer.rs        # Text buffer (rope-based)
│   ├── syntax.rs        # Syntax highlighting
│   ├── cursor.rs        # Multi-cursor support
│   ├── search.rs        # Search & replace
│   ├── minimap.rs       # Code minimap
│   ├── lsp.rs           # LSP client
│   └── git.rs           # Git integration
├── index.html           # HTML entry point
├── Cargo.toml           # Rust dependencies
└── Trunk.toml           # Trunk configuration
```

## License

MIT
