# DuckDB Extension Macros

[![Crates.io](https://img.shields.io/crates/v/duckdb-ext-macros.svg)](https://crates.io/crates/duckdb-ext-macros)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust 2024 Edition compatible procedural macro crate for creating DuckDB loadable extensions.

This crate lives in the [`cargo-duckdb-ext-tools`](https://github.com/redraiment/cargo-duckdb-ext-tools) workspace and is the entrypoint macro used by projects generated with `cargo duckdb-ext new`.

## Overview

`duckdb-ext-macros` provides the `#[duckdb_extension]` attribute macro. The macro transforms a Rust initialization function into the C-compatible entrypoint DuckDB expects when loading an extension, while handling the FFI boilerplate and error propagation.

It is designed as a modern alternative to DuckDB's official `duckdb-loadable-macros` crate, with Rust 2024 Edition support and dependency versions aligned with the rest of this workspace.

## Features

- **Rust 2024 Edition Support**: Compatible with modern Rust editions
- **Generated C API Entrypoint**: Produces `{extension_name}_init_c_api`
- **Flexible Configuration**: Configure extension name and API version via attributes or environment variables
- **Error Propagation**: Converts initialization failures into DuckDB error messages
- **Workspace Integration**: Used by the `cargo-duckdb-ext-tools` project templates
- **Modern Macro Stack**: Uses current `darling`, `syn`, `quote`, and `proc-macro2`

## Installation

Projects created with `cargo duckdb-ext new` already include this crate. For custom projects, add it to your `Cargo.toml` with DuckDB's loadable extension dependencies:

```toml
[dependencies]
duckdb = { version = "1.10503.1", features = ["vtab-loadable"] } # or your preferred version
duckdb-ext-macros = "0.2.0"
libduckdb-sys = { version = "1.10503.1", features = ["loadable-extension"] }
```

## Quick Start

```rust
use duckdb::Connection;
use duckdb_ext_macros::duckdb_extension;
use std::error::Error;

#[duckdb_extension(name = "my_extension", api_version = "v1.2.0")]
pub fn extension_entrypoint(connection: Connection) -> Result<(), Box<dyn Error>> {
    // Register functions, types, or perform setup.
    Ok(())
}
```

Build and package the extension with the companion CLI:

```bash
cargo duckdb-ext build -- --release
```

## Usage

### Basic Usage

The simplest usage with default values:

```rust
#[duckdb_extension]
fn extension_entrypoint(connection: duckdb::Connection) -> Result<(), String> {
    // Extension logic
    Ok(())
}
```

This will:

- Use your Cargo package name as the extension name, converting `-` to `_`
- Use DuckDB C API version `v1.2.0` as the minimum required version
- Generate a C entrypoint function named `{package_name}_init_c_api`

### Custom Extension Name

```rust
#[duckdb_extension(name = "custom_extension")]
fn init(connection: duckdb::Connection) -> Result<(), String> {
    Ok(())
}
```

### Custom API Version

```rust
#[duckdb_extension(api_version = "v1.1.0")]
fn init(connection: duckdb::Connection) -> Result<(), String> {
    Ok(())
}
```

### Environment Variables

You can also configure the extension via environment variables, which is useful for build scripts or CI/CD:

```bash
# Set extension name
export DUCKDB_EXTENSION_NAME="my_extension"

# Set minimum API version
export DUCKDB_MINIMUM_API_VERSION="v1.2.0"
```

Environment variables are used as fallbacks when macro attributes are not provided.

## How It Works

The `#[duckdb_extension]` macro transforms your Rust function into a DuckDB-compatible C entrypoint:

1. Parses macro attributes (`name`, `api_version`) and fallback environment variables
2. Resolves the extension name
3. Resolves the minimum DuckDB C API version
4. Generates a C-compatible function with the signature `{extension_name}_init_c_api`
5. Initializes DuckDB's Rust extension API through `libduckdb_sys`
6. Opens a `duckdb::Connection` from DuckDB's raw database pointer
7. Calls your initialization function
8. Converts Rust errors into DuckDB error messages

### Generated Code Shape

For this input:

```rust
#[duckdb_extension(name = "my_ext")]
fn init(conn: duckdb::Connection) -> Result<(), String> {
    Ok(())
}
```

The macro generates an entrypoint with this public ABI shape:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn my_ext_init_c_api(
    info: libduckdb_sys::duckdb_extension_info,
    access: *const libduckdb_sys::duckdb_extension_access,
) -> bool {
    // FFI and error handling code
}
```

The original Rust function is kept and called by the generated entrypoint.

## Configuration Options

### Macro Attributes

- `name`: Extension name used in the generated C entrypoint
- `api_version`: Minimum DuckDB C API version required, defaulting to `v1.2.0`

### Environment Variables

- `DUCKDB_EXTENSION_NAME`: Extension name fallback when `name` is not provided
- `DUCKDB_MINIMUM_API_VERSION`: Minimum API version fallback when `api_version` is not provided

### Name Resolution Order

1. `name` attribute in the macro
2. `DUCKDB_EXTENSION_NAME` environment variable
3. `CARGO_PKG_NAME` environment variable

Resolved names convert `-` to `_` for C symbol compatibility.

### API Version Resolution Order

1. `api_version` attribute in the macro
2. `DUCKDB_MINIMUM_API_VERSION` environment variable
3. Default: `v1.2.0`

## Error Handling

The macro handles:

- **Compile-time errors**: Invalid macro syntax or unsupported item types
- **Runtime errors**: Extension initialization failures
- **FFI errors**: DuckDB C API initialization and connection setup failures

Errors are propagated to DuckDB with descriptive messages. If an error string cannot be converted to a C string, the generated code uses a fallback error message.

## Relationship to `cargo-duckdb-ext-tools`

This crate only generates the extension entrypoint. It does not append DuckDB's 534-byte extension metadata footer. Use the workspace CLI for packaging:

```bash
cargo duckdb-ext build -- --release
```

or:

```bash
cargo duckdb-ext package \
  -i target/release/libmy_extension.dylib \
  -o my_extension.duckdb_extension \
  -v v1.0.0 \
  -p osx_arm64 \
  -d v1.4.2
```

## Migration from `duckdb-loadable-macros`

If you're currently using `duckdb-loadable-macros`, migration is straightforward:

1. Update `Cargo.toml`:

   ```toml
   # Remove or replace:
   # duckdb-loadable-macros = "..."

   # Add:
   duckdb-ext-macros = "0.2.0"
   ```

2. Update imports:

   ```rust
   // Change from:
   // use duckdb_loadable_macros::duckdb_entrypoint;

   // To:
   use duckdb_ext_macros::duckdb_extension;
   ```

3. Update the attribute:

   ```rust
   // Change from:
   // #[duckdb_entrypoint]

   // To:
   #[duckdb_extension]
   ```

Your extension registration logic can usually stay unchanged.

## License

This project is licensed under the MIT License. See the workspace [`LICENSE`](../../LICENSE) file for details.

## Acknowledgments

- DuckDB team for the excellent database and original `duckdb-loadable-macros`
- The Rust community for procedural macro tooling
- All contributors to this project
