//! Command line options for the `meta` subcommand
//!
//! This module defines parameters for reading and updating DuckDB
//! extension metadata footers in `.duckdb_extension` files.

use crate::commands::STABLE_ABI_TYPE;
use crate::commands::UNSTABLE_ABI_TYPE;
use clap::Args;

/// Command line options for the `meta` subcommand
///
/// This struct defines all optional metadata fields that can be updated.
/// If no optional field is supplied, the command only displays metadata.
#[derive(Args, Debug)]
pub struct MetaOptions {
    /// First reserved 32-byte field as lowercase hexadecimal bytes
    #[arg(long, value_name = "VALUE")]
    pub field_1: Option<String>,

    /// Second reserved 32-byte field as lowercase hexadecimal bytes
    #[arg(long, value_name = "VALUE")]
    pub field_2: Option<String>,

    /// Third reserved 32-byte field as lowercase hexadecimal bytes
    #[arg(long, value_name = "VALUE")]
    pub field_3: Option<String>,

    /// Version of the extension (e.g., "v1.0.0")
    #[arg(
        short = 'v',
        long,
        visible_alias = "extension_version",
        value_name = "EXTENSION-VERSION"
    )]
    pub extension_version: Option<String>,

    /// Target platform identifier (e.g., "osx_arm64", "linux_amd64")
    #[arg(short = 'p', long, value_name = "DUCKDB-PLATFORM")]
    pub duckdb_platform: Option<String>,

    /// Magic version stored in the metadata footer
    #[arg(short = 'm', long, value_name = "MAGIC-VERSION")]
    pub magic_version: Option<String>,

    /// Extension signature area as lowercase hexadecimal bytes
    #[arg(short = 's', long, value_name = "SIGNATURE")]
    pub signature: Option<String>,

    /// Version specification (DuckDB version or C API version)
    #[command(flatten)]
    version: Version,

    /// Path to the extension file to inspect or update
    #[arg(value_name = "EXTENSION-FILE")]
    pub extension_file: String,
}

/// Version specification for DuckDB extension metadata
///
/// This struct represents mutually exclusive version options:
/// either a DuckDB version or a DuckDB C API version.
#[derive(Args, Debug)]
#[group(multiple = false)]
struct Version {
    /// DuckDB version the extension is built for (e.g., "v1.4.2")
    #[arg(short = 'd', long, value_name = "DUCKDB-VERSION")]
    duckdb_version: Option<String>,

    /// DuckDB C API version the extension is built for (e.g., "v1.2.0")
    #[arg(short = 'a', long, value_name = "DUCKDB-CAPI-VERSION")]
    duckdb_capi_version: Option<String>,
}

impl MetaOptions {
    /// Returns the DuckDB version or DuckDB C API version, if provided
    pub fn version(&self) -> Option<String> {
        self.version
            .duckdb_version
            .as_ref()
            .or(self.version.duckdb_capi_version.as_ref())
            .map(ToString::to_string)
    }

    /// Determines the ABI type based on version specification
    ///
    /// Returns `C_STRUCT` for C API versions (stable ABI) or
    /// `C_STRUCT_UNSTABLE` for regular DuckDB versions (unstable ABI).
    pub fn abi_type(&self) -> Option<String> {
        if self.version.duckdb_capi_version.is_some() {
            Some(STABLE_ABI_TYPE.to_string())
        } else if self.version.duckdb_version.is_some() {
            Some(UNSTABLE_ABI_TYPE.to_string())
        } else {
            None
        }
    }
}
