//! Implementation of the `package` subcommand
//!
//! This module handles the low-level packaging of dynamic libraries
//! into DuckDB extensions by appending the 534-byte metadata footer.

use crate::commands::Command;
use crate::commands::meta_command::ExtensionMetadata;
use crate::error::ToolsError;
use crate::finished;
use crate::helpers::open_duplicate;
use crate::logging;
use crate::options::package_option::PackageOptions;
use std::fs::File;

/// Command for packaging dynamic libraries as DuckDB extensions
///
/// This struct holds the output file and encoded metadata needed to create
/// a DuckDB extension file.
pub(crate) struct PackageCommand {
    /// File handle opened in append mode for writing metadata
    pub(crate) file: File,
    /// Path where the output extension file should be created
    pub(crate) extension_path: String,
    /// Encoded DuckDB extension metadata footer
    pub(crate) metadata: ExtensionMetadata,
}

impl TryFrom<PackageOptions> for PackageCommand {
    type Error = ToolsError;

    /// Constructs a PackageCommand from command line options
    ///
    /// This conversion sets up the global quiet flag and creates the
    /// extension file by duplicating the source library.
    fn try_from(options: PackageOptions) -> Result<Self, Self::Error> {
        let file = open_duplicate(&options.library_path, &options.extension_path)?;
        let duckdb_version = options.version();
        let abi_type = options.abi_type();
        let metadata = ExtensionMetadata::package(
            &abi_type,
            &options.extension_version,
            &duckdb_version,
            &options.duckdb_platform,
        )?;
        Ok(Self {
            file,
            extension_path: options.extension_path,
            metadata,
        })
    }
}

impl Command for PackageCommand {
    /// Executes the packaging operation
    ///
    /// This method appends the complete 534-byte metadata footer
    /// to the extension file.
    fn execute(&mut self) -> Result<(), ToolsError> {
        self.write_metadata()
    }
}

impl PackageCommand {
    /// Appends the 534-byte DuckDB extension metadata footer to the file
    ///
    /// The footer encoding is owned by `meta_command` so package and meta
    /// operations use the same metadata layout.
    fn write_metadata(&mut self) -> Result<(), ToolsError> {
        self.metadata.display();
        self.metadata.append_to(&mut self.file)?;
        finished!("DuckDB Extension ({})", self.extension_path);
        Ok(())
    }
}
