# DuckDB Extension Macros

A modern, Rust 2024 Edition compatible procedural macro for creating DuckDB loadable extensions.

## Overview

This crate provides the `#[duckdb_extension]` attribute macro that simplifies the creation of DuckDB loadable extensions in Rust. It serves as a modern alternative to DuckDB's official `duckdb-loadable-macros` crate, which only supports Rust 2021 Edition.

## Features

- **Rust 2024 Edition Support**: Fully compatible with modern Rust editions
- **Drop-in Replacement**: API-compatible with `duckdb-loadable-macros`
- **Flexible Configuration**: Configure extension name and API version via attributes or environment variables
- **Comprehensive Error Handling**: Proper error propagation from Rust to DuckDB's C API
- **Safe FFI Integration**: Manages unsafe operations with proper error handling
- **Modern Dependencies**: Uses up-to-date versions of `darling`, `syn`, `quote`, and `proc-macro2`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
duckdb = { version = "1.4.2", features = ["vtab-loadable"] }  # or your preferred version
duckdb-ext-macros = "0.1.0"
libduckdb-sys = { version = "1.4.2", features = ["loadable-extension"] }
```

## Quick Start

```rust
use duckdb_ext_macros::duckdb_extension;

#[duckdb_extension(name = "my_extension", api_version = "v1.2.0")]
fn my_extension_init(connection: duckdb::Connection) -> Result<(), String> {
    // Your extension initialization logic here
    // Register functions, types, or perform setup

    println!("My extension loaded successfully!");
    Ok(())
}
```

## Usage

### Basic Usage

The simplest usage with default values:

```rust
#[duckdb_extension]
fn my_extension(connection: duckdb::Connection) -> Result<(), String> {
    // Extension logic
    Ok(())
}
```

This will:
- Use your cargo package name as the extension name (converted to snake_case)
- Use DuckDB API version "v1.2.0" as the minimum required version
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

The `#[duckdb_extension]` macro transforms your Rust function into a DuckDB-compatible C entrypoint. Here's what happens under the hood:

1. **Argument Parsing**: The macro parses attributes (`name`, `api_version`) or reads environment variables
2. **Name Resolution**: Determines the extension name with fallback logic
3. **API Version Resolution**: Determines the minimum required DuckDB API version
4. **Code Generation**: Generates a C-compatible function with the signature `{extension_name}_init_c_api`
5. **FFI Integration**: Creates safe wrappers around DuckDB's C API calls
6. **Error Handling**: Converts Rust errors to DuckDB error messages

### Generated Code Example

For this input:
```rust
#[duckdb_extension(name = "my_ext")]
fn init(conn: duckdb::Connection) -> Result<(), String> { Ok(()) }
```

The macro generates:
```rust
#[unsafe(no_mangle)]
pub extern "C" fn my_ext_init_c_api(
    info: libduckdb_sys::duckdb_extension_info,
    access: *const libduckdb_sys::duckdb_extension_access
) -> bool {
    // FFI and error handling code
}
```

## Comparison with Official `duckdb-loadable-macros`

| Feature | `duckdb-ext-macros` | `duckdb-loadable-macros` |
|---------|---------------------|--------------------------|
| Rust Edition | **2024 Edition** | 2021 Edition only |
| Dependencies | Modern versions | Older versions |

## Migration from `duckdb-loadable-macros`

If you're currently using `duckdb-loadable-macros`, migration is straightforward:

1. **Update Cargo.toml**:
   ```toml
   # Remove or replace:
   # duckdb-loadable-macros = "..."

   # Add:
   duckdb-ext-macros = "0.1.0"
   ```

2. **Update imports**:
   ```rust
   // Change from:
   // use duckdb_loadable_macros::duckdb_entrypoint;

   // To:
   use duckdb_ext_macros::duckdb_extension;
   ```

3. **Update attribute**:
   ```rust
   // Change from:
   // #[duckdb_entrypoint]

   // To:
   #[duckdb_extension]
   ```

The API is designed to be compatible, so your existing extension logic should work without changes.

## Configuration Options

### Macro Attributes

- `name`: The extension name (used in the C entrypoint function)
- `api_version`: Minimum DuckDB C API version required (default: "v1.2.0")

### Environment Variables

- `DUCKDB_EXTENSION_NAME`: Extension name (fallback if `name` attribute not provided)
- `DUCKDB_MINIMUM_API_VERSION`: Minimum API version (fallback if `api_version` attribute not provided)

### Name Resolution Order

The extension name is determined in this order:
1. `name` attribute in the macro
2. `DUCKDB_EXTENSION_NAME` environment variable
3. `CARGO_PKG_NAME` environment variable (cargo package name)

### API Version Resolution Order

The minimum API version is determined in this order:
1. `api_version` attribute in the macro
2. `DUCKDB_MINIMUM_API_VERSION` environment variable
3. Default: "v1.2.0"

## Error Handling

The macro provides comprehensive error handling:

1. **Compile-time Errors**: Invalid macro syntax or configuration
2. **Runtime Errors**: Extension initialization failures
3. **FFI Errors**: C API interaction failures

Errors are propagated to DuckDB with descriptive messages. If error string allocation fails, a fallback message is used.

## License

This project is licensed under the same terms as DuckDB (MIT License).

## Acknowledgments

- DuckDB team for the excellent database and original `duckdb-loadable-macros`
- The Rust community for excellent procedural macro libraries
- All contributors to this project