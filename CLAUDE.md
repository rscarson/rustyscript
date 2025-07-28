# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Building and Testing
```bash
# Standard build
cargo build

# Run all tests
cargo test

# Run tests with all features
cargo test --all-features

# Run specific test by name
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Build documentation
cargo doc --open

# Update README from lib.rs docs
cargo rdme
```

### Common Feature Combinations
```bash
# Test with web features
cargo test --features "web_stub,url,crypto,json,console"

# Test with filesystem access
cargo test --features "fs"

# Test all safe features (no sandbox-breaking)
cargo test --features "console,crypto,url,json,broadcast_channel,cache,cron,kv,webgpu,webstorage,webidl"
```

## Architecture Overview

RustyScript is a Rust library for executing JavaScript in a secure, sandboxed environment. The architecture follows a layered design:

### Core Runtime System
- **`Runtime`**: Public API with sync/async/immediate execution variants
- **`InnerRuntime`**: Private implementation wrapping Deno's V8 runtime
- **`StaticRuntime`**: Thread-safe wrapper for concurrent access
- **`RuntimeBuilder`**: Configurable runtime construction with security policies

### Module System
- **`ModuleLoader`**: Handles module resolution, transpilation, and caching
- **`Module`**: Represents compiled JavaScript/TypeScript modules
- **`ModuleHandle`**: Safe reference to loaded modules with lifecycle management

### Extension Architecture
Extensions are feature-gated Deno ops that provide JavaScript APIs:
- **Safe extensions** (included by default): `console`, `crypto`, `url`, `json`
- **Sandbox-breaking extensions** (opt-in): `fs`, `io`, `http`, `ffi`, `net`
- **Experimental**: `node` (Node.js compatibility layer)

### JavaScript Value System
- **`JsValue`**: Primary interface for Rust-JavaScript interop
- **Serde integration**: Automatic serialization/deserialization between Rust and JS types
- **Type-safe conversions**: `from_value<T>()` and `to_value()` for structured data

## Key Patterns

### Triple API Pattern
Most functionality is available in three variants:
- **Sync**: Blocking execution (`eval_sync`, `call_sync`)
- **Async**: Non-blocking with futures (`eval`, `call`)  
- **Immediate**: Direct V8 execution without microtask processing (`eval_immediate`)

### Security Model
- **Sandboxed by default**: No access to filesystem, network, or system APIs
- **Explicit capability grants**: Enable features through Cargo features or runtime options
- **Extension-based permissions**: Fine-grained control over available JavaScript APIs

### Error Handling
- Custom `Error` type wrapping V8 JavaScript errors
- Automatic error conversion between Rust and JavaScript contexts
- Stack trace preservation across language boundaries

## Development Guidelines

### Working with Extensions
- Extensions are in `src/ext/` with feature gates in `Cargo.toml`
- Each extension has `mod.rs`, `init_*.js`, and optional Rust implementation
- Use `RuntimeBuilder::with_*()` methods to enable extensions programmatically

### Testing Approach
- Tests are primarily inline with `#[cfg(test)]` modules in source files
- Examples in `examples/` serve as integration tests and documentation
- Use `RuntimeBuilder::new_with_defaults()` for basic test setups
- Test different feature combinations to ensure proper isolation

### Module Development
- JavaScript/TypeScript files can be loaded from filesystem or embedded as strings
- Use `include_str!()` macro for embedding JS code in Rust
- TypeScript is automatically transpiled using SWC
- Module caching is handled automatically but can be controlled via `ModuleLoader`

### Common Debugging
- Use `console.log()` in JavaScript code (requires `console` feature)
- Enable `log` feature and use `env_logger` to see internal Deno logs
- V8 errors include JavaScript stack traces when available
- Use `Runtime::take_snapshot()` to inspect runtime state

## Architecture Notes

### Thread Safety
- `Runtime` is `!Send + !Sync` - must stay on creating thread
- `StaticRuntime` provides `Send + Sync` wrapper for multi-threaded scenarios
- Workers (`worker` feature) provide isolated per-thread runtimes

### Memory Management
- V8 heap size can be configured via `RuntimeBuilder::max_heap_size()`
- Automatic garbage collection with manual `Runtime::run_gc()` option
- JavaScript values are automatically cleaned up when `JsValue` is dropped

### Performance Considerations
- Snapshots can significantly improve startup time for repeated runtime creation
- Module caching reduces repeated compilation overhead
- Use `eval_immediate` for CPU-bound operations that don't need microtasks