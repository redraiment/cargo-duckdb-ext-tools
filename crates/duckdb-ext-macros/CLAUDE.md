# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a **Rust procedural macro library** (`proc-macro = true`) for creating DuckDB extensions. The main macro `#[duckdb_extension]` simplifies the process of creating DuckDB extensions in Rust by generating the proper C API entrypoint functions and handling FFI boilerplate.

## Build System

- **Build tool**: Cargo (Rust's package manager)
- **Build command**: `cargo build`
- **Release build**: `cargo build --release`
- **Check compilation**: `cargo check`
- **Clean build artifacts**: `cargo clean`

## Code Architecture

### Main Macro: `#[duckdb_extension]`

The macro transforms a regular Rust function into a DuckDB extension entrypoint. Key features:

1. **Automatic entrypoint generation**: Creates a function named `{extension_name}_init_c_api` that DuckDB calls
2. **API version handling**: Supports configuration via attribute or `DUCKDB_MINIMUM_API_VERSION` environment variable (defaults to "v1.2.0")
3. **Extension name resolution**: In order of precedence:
   - `name` parameter in macro attribute
   - `DUCKDB_EXTENSION_NAME` environment variable
   - `CARGO_PKG_NAME` environment variable (cargo package name)
4. **Error handling**: Properly propagates Rust errors to DuckDB's error system

### Source Structure

- `src/lib.rs`: Contains the entire macro implementation
- `Cargo.toml`: Defines the crate as a procedural macro with dependencies:
  - `darling`: For parsing macro attributes
  - `proc-macro2`, `quote`, `syn`: Standard Rust macro development crates

### Macro Usage Example

```rust
#[duckdb_extension(name = "my_extension", api_version = "v1.2.0")]
fn my_extension_init(connection: duckdb::Connection) -> Result<(), String> {
    // Extension initialization logic
    Ok(())
}
```

This generates a C-compatible function `my_extension_init_c_api` that:
1. Initializes the DuckDB Rust extension API
2. Opens a database connection
3. Calls `my_extension_init` with the connection
4. Handles errors by setting DuckDB's error message

### FFI Integration

The macro interfaces with DuckDB's C API through `libduckdb_sys` (not explicitly listed as dependency but referenced in code). The generated code:
- Uses `unsafe` blocks for FFI operations
- Converts Rust strings to C strings for error messages
- Manages raw pointer operations for DuckDB's extension info and access structures

## Development Notes

- This is a **procedural macro crate** - it generates code at compile time
- The crate doesn't include tests or examples in the repository
- The macro assumes the user's crate has `duckdb` and `libduckdb_sys` dependencies
- Environment variables can override macro parameters for build configuration flexibility