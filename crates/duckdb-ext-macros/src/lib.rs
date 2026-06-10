//! A procedural macro for creating DuckDB extensions in Rust.
//!
//! This crate provides the `#[duckdb_extension]` attribute macro that simplifies the creation
//! of DuckDB loadable extensions. It generates the proper C API entrypoint function and handles
//! FFI boilerplate, error propagation, and API version compatibility.
//!
//! # Key Features
//! - **Rust 2024 Edition Support**: Compatible with modern Rust editions (unlike the official
//!   `duckdb-loadable-macros` which only supports Rust 2021)
//! - **Flexible Configuration**: Extension name and API version can be configured via macro
//!   attributes or environment variables
//! - **Error Handling**: Proper error propagation from Rust to DuckDB's C API
//! - **Safe FFI Integration**: Manages unsafe operations with proper error handling
//!
//! # Usage
//! ```rust
//! #[duckdb_extension(name = "my_extension", api_version = "v1.2.0")]
//! fn my_extension_init(connection: duckdb::Connection) -> Result<(), String> {
//!     // Your extension initialization logic here
//!     Ok(())
//! }
//! ```

use darling::Error;
use darling::FromMeta;
use darling::ast::NestedMeta;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote_spanned;
use std::env;
use syn::Item;
use syn::parse_macro_input;
use syn::spanned::Spanned;

/// Default DuckDB C API version used when no version is specified.
/// This corresponds to DuckDB's v1.2.0 API, which is a stable and widely supported version.
const DEFAULT_API_VERSION: &str = "v1.2.0";

/// Arguments parsed from the `#[duckdb_extension]` macro attribute.
///
/// This struct defines the configurable parameters for the DuckDB extension macro.
/// Fields can be set via macro attributes or environment variables, with macro
/// attributes taking precedence.
#[derive(Debug, FromMeta)]
struct DuckDBExtensionMacroArguments {
    #[darling(default)]
    /// The name of the extension, used to generate the C entrypoint function.
    ///
    /// If not specified, the macro will attempt to get the name from:
    /// 1. `DUCKDB_EXTENSION_NAME` environment variable
    /// 2. `CARGO_PKG_NAME` environment variable (cargo package name)
    ///
    /// The name is converted from kebab-case to snake_case for C compatibility.
    name: Option<String>,

    /// The minimum DuckDB C API version required by this extension.
    ///
    /// It's recommended to set this to the lowest API version that your
    /// extension supports to maximize compatibility. If not specified,
    /// the macro will use:
    /// 1. `DUCKDB_MINIMUM_API_VERSION` environment variable
    /// 2. Default value: "v1.2.0"
    api_version: Option<String>,
}

/// Procedural macro attribute for creating DuckDB loadable extensions.
///
/// This macro transforms a Rust function into a DuckDB extension entrypoint that
/// can be loaded by DuckDB's C API. It handles FFI boilerplate, error propagation,
/// and API version compatibility.
///
/// # Arguments
/// - `attributes`: Macro attributes (`name`, `api_version`)
/// - `items`: The Rust function to transform into a DuckDB extension
///
/// # Returns
/// A `TokenStream` containing the original function plus the generated C entrypoint.
#[proc_macro_attribute]
pub fn duckdb_extension(attributes: TokenStream, items: TokenStream) -> TokenStream {
    // Parse macro arguments using darling
    let args = match NestedMeta::parse_meta_list(attributes.into())
        .map_err(|error| Error::from(error))
        .and_then(|attributes| DuckDBExtensionMacroArguments::from_list(&attributes)) {
        Ok(args) => args,
        Err(error) => return TokenStream::from(error.write_errors()),
    };

    // Determine the minimum API version with fallback logic:
    // 1. Macro attribute `api_version`
    // 2. DUCKDB_MINIMUM_API_VERSION environment variable
    // 3. DEFAULT_API_VERSION constant ("v1.2.0")
    let minimum_api_version = args.api_version
        .or_else(|| env::var("DUCKDB_MINIMUM_API_VERSION").ok())
        .unwrap_or_else(|| DEFAULT_API_VERSION.to_string());

    // Determine the extension name with fallback logic:
    // 1. Macro attribute `name`
    // 2. DUCKDB_EXTENSION_NAME environment variable
    // 3. CARGO_PKG_NAME environment variable (cargo package name)
    // Convert kebab-case to snake_case for C compatibility
    let extension_name = args.name
        .or_else(|| env::var("DUCKDB_EXTENSION_NAME").ok())
        .or_else(|| env::var("CARGO_PKG_NAME").ok())
        .map(|extension_name| extension_name.replace("-", "_"))
        .expect("Failed to get extension name");

    // Parse the input items to determine what we're transforming
    let ast = parse_macro_input!(items as Item);
    match ast {
        Item::Fn(function) => {
            // Generate the C entrypoint function name: {extension_name}_init_c_api
            let entrypoint = Ident::new(format!("{extension_name}_init_c_api").as_str(), Span::call_site());
            let function_name = function.sig.ident.clone();

            // Generate the transformed code using quote_spanned to preserve span information
            quote_spanned! {function.span() =>
                /// Entrypoint that will be called by DuckDB
                #[unsafe(no_mangle)]
                pub extern "C" fn #entrypoint(info: libduckdb_sys::duckdb_extension_info, access: *const libduckdb_sys::duckdb_extension_access) -> bool {
                    unsafe {
                        // Chain of operations that must succeed:
                        // 1. Initialize DuckDB Rust extension API with the specified version
                        // 2. Get database pointer from DuckDB's C API
                        // 3. Open a Rust Connection from the raw database pointer
                        // 4. Call the user's extension initialization function
                        let result = libduckdb_sys::duckdb_rs_extension_api_init(info, access, #minimum_api_version)
                            .map_err(|message| message.to_owned())
                            .and_then(|_| (*access).get_database.ok_or(String::from("Failed to get database")))
                            .map(|getter| *getter(info))
                            .and_then(|db| duckdb::Connection::open_from_raw(db.cast()).map_err(|error| error.to_string()))
                            .and_then(|connection| #function_name(connection).map_err(|error| error.to_string()));

                        // Handle errors by setting DuckDB's error message
                        if let Err(message) = result {
                            let setter = (*access).set_error.expect("Failed to get error setter");
                            let message = if let Ok(error) = std::ffi::CString::new(message) {
                                error.as_ptr()
                            } else {
                                // Fallback error message if we can't allocate C string
                                c"An error occured but the extension failed to allocate memory for an error string".as_ptr()
                            };
                            setter(info, message);
                            false  // Return false to indicate extension initialization failed
                        } else {
                            true   // Return true to indicate successful initialization
                        }
                    }
                }

                #function
            }.into()
        }
        _ => panic!("Only function items are allowed on duckdb extension entrypoint"),
    }
}
