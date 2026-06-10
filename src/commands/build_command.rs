//! Implementation of the `build` subcommand
//!
//! This module handles the complete build and packaging workflow:
//! 1. Executing cargo build with JSON output
//! 2. Parsing build artifacts to find dynamic libraries
//! 3. Creating PackageCommand instances for each artifact
//! 4. Applying intelligent defaults for extension metadata

use crate::building;
use crate::commands::package_command::PackageCommand;
use crate::commands::Command;
use crate::error::ToolsError;
use crate::finished;
use crate::helpers::open_duplicate;
use crate::helpers::NormalizedPath;
use crate::logging;
use crate::options::build_option::BuildOptions;
use cargo_metadata::camino::Utf8PathBuf;
use cargo_metadata::semver::Version;
use cargo_metadata::Artifact;
use cargo_metadata::Message;
use cargo_metadata::MetadataCommand;
use cargo_metadata::Package;
use cargo_metadata::PackageId;
use cargo_metadata::PackageName;
use cargo_metadata::TargetKind;
use std::collections::HashMap;
use std::env::consts::ARCH;
use std::env::consts::OS;
use std::io::BufRead;
use std::process::Command as Process;
use std::process::Stdio;
use std::str::FromStr;
use target_lexicon::Architecture;
use target_lexicon::OperatingSystem;
use target_lexicon::Triple;

/// Command for building and packaging DuckDB extensions
///
/// This struct orchestrates the complete build-to-package workflow,
/// handling cargo execution, artifact detection, and metadata inference.
pub(crate) struct BuildCommand {
    /// Cargo build command with JSON output enabled
    process: Process,
    /// Target directory where build artifacts are located
    target_directory: String,
    /// Optional override for extension output path
    extension_path: Option<String>,
    /// Optional override for extension version
    extension_version: Option<String>,
    /// Optional override for target platform
    duckdb_platform: Option<String>,
    /// DuckDB (C API) version compatibility
    duckdb_version: String,
    /// ABI type specification (C_STRUCT or C_STRUCT_UNSTABLE)
    abi_type: String,
    /// List of packages in the workspace that produce CDyLib targets
    packages: Vec<Package>,
}

impl TryFrom<BuildOptions> for BuildCommand {
    type Error = ToolsError;

    /// Constructs a BuildCommand from command line options
    ///
    /// This conversion extracts project metadata, detects DuckDB version
    /// from dependencies, and filters packages that produce dynamic libraries.
    fn try_from(options: BuildOptions) -> Result<Self, Self::Error> {
        let mut metadata_command = MetadataCommand::new();
        if let Some(manifest_path) = options.manifest_path.as_ref() {
            metadata_command.manifest_path(manifest_path);
        }
        metadata_command.verbose(logging::enabled());
        let metadata = metadata_command.exec()?;
        let target_directory = metadata.target_directory.normalize();

        // Auto-detect DuckDB version from dependencies
        let duckdb_version = options.version().or_else(|| {
            metadata.packages.iter().find_map(|package| {
                if package.name == "duckdb" || package.name == "libduckdb-sys" {
                    Some(normalize_duckdb_version_from_crate_version(&package.version))
                } else {
                    None
                }
            })
        }).expect("Missing duckdb version");

        // Filter packages that are workspace members and produce CDyLib targets
        let packages = metadata
            .packages
            .into_iter()
            .filter(|package| metadata.workspace_members.contains(&package.id))
            .filter(|package| {
                package
                    .targets
                    .iter()
                    .any(|target| target.kind.contains(&TargetKind::CDyLib))
            })
            .collect::<Vec<_>>();

        let mut process = Process::new("cargo");
        let mut args = vec!["build".to_string(), "--message-format=json".to_string()];
        args.extend_from_slice(&options.args);
        process.args(&args);
        process.stdout(Stdio::piped());
        process.stderr(Stdio::piped());

        let abi_type = options.abi_type();
        Ok(Self {
            process,
            target_directory,
            extension_path: options.extension_path,
            extension_version: options.extension_version,
            duckdb_platform: options.duckdb_platform,
            duckdb_version,
            abi_type,
            packages,
        })
    }
}

impl Command for BuildCommand {
    /// Executes the build and packaging workflow
    ///
    /// This method runs cargo build, processes the artifacts,
    /// and executes packaging for each detected dynamic library.
    fn execute(&mut self) -> Result<(), ToolsError> {
        for mut packager in self.build()? {
            packager.execute()?;
        }
        Ok(())
    }
}

impl BuildCommand {
    /// Executes the build process and creates Packager instances for each artifact
    ///
    /// This method:
    /// 1. Runs cargo build with JSON output
    /// 2. Parses the JSON messages to find CDyLib artifacts
    /// 3. Matches artifacts with their corresponding packages
    /// 4. Creates Packager instances for packaging each extension
    pub(crate) fn build(&mut self) -> Result<Vec<PackageCommand>, ToolsError> {
        let packages = self.packages.iter()
            .inspect(|package| building!("{} {}", package.name, package.version))
            .map(|package| (package.id.to_owned(), package))
            .collect::<HashMap<PackageId, &Package>>();

        // Execute cargo build and process JSON output
        self.process.spawn()?.wait_with_output()?.stdout.lines()
            .filter_map(Result::ok)
            .map(|json| serde_json::from_str::<Message>(&json))
            .filter_map(Result::ok)
            .filter_map(|message| match message {
                Message::CompilerArtifact(artifact) if artifact.target.kind.contains(&TargetKind::CDyLib) => Some(artifact),
                _ => None,
            })
            .filter_map(|artifact| {
                packages
                    .get(&artifact.package_id)
                    .map(|package| (package, artifact))
            })
            .flat_map(|(package, artifact)| self.packs(package, &artifact))
            .collect::<Result<Vec<_>, _>>()
    }

    /// Creates Packager instances for all filenames in an artifact
    ///
    /// This processes each filename in the artifact, filtering for valid
    /// dynamic library files within the target directory.
    fn packs(
        &self,
        package: &Package,
        artifact: &Artifact,
    ) -> Vec<Result<PackageCommand, ToolsError>> {
        artifact.filenames.iter()
            .filter_map(|path| path.canonicalize_utf8().ok())
            .map(|path| path.normalize())
            .filter(|path| path.starts_with(&self.target_directory))
            .filter(|path| [".dll", ".so", ".dylib"].iter().any(|ext| path.ends_with(ext)))
            .filter_map(|path| Utf8PathBuf::from_str(&path).ok())
            .inspect(|path| finished!("Dynamic Library ({})", path.as_str()))
            .map(|path| self.pack(&package.name, &package.version, &path))
            .collect()
    }

    /// Creates a Packager instance for a specific library file
    ///
    /// This method applies intelligent defaults for all parameters:
    /// - Extension path: auto-generated from package name
    /// - Extension version: extracted from Cargo.toml
    /// - Platform: detected from build target or host system
    /// - DuckDB version: from dependencies or user override
    fn pack(
        &self,
        package_name: &PackageName,
        package_version: &Version,
        filename: &Utf8PathBuf,
    ) -> Result<PackageCommand, ToolsError> {
        let library_path = filename.to_string();
        let extension_path = self
            .extension_path
            .to_owned()
            .unwrap_or_else(|| self.artifact_extension_path(filename, package_name));
        let extension_version = self
            .extension_version
            .to_owned()
            .unwrap_or_else(|| format!("v{package_version}"));
        let duckdb_platform = self
            .duckdb_platform
            .to_owned()
            .or_else(|| self.artifact_duckdb_platform(filename))
            .unwrap_or_else(Self::default_duckdb_platform);
        let duckdb_version = self.duckdb_version.to_owned();
        let abi_type = self.abi_type.to_owned();

        let file = open_duplicate(&library_path, &extension_path)?;
        Ok(PackageCommand {
            file,
            extension_path,
            extension_version,
            duckdb_platform,
            duckdb_version,
            abi_type,
        })
    }

    /// Generates the extension file path from the library path and package name
    ///
    /// This replaces the library filename with the package name and changes
    /// the extension to `.duckdb_extension`.
    fn artifact_extension_path(&self, filename: &Utf8PathBuf, package_name: &str) -> String {
        let mut path = filename.clone();
        path.set_file_name(package_name.replace('-', "_"));
        path.set_extension("duckdb_extension");
        path.to_string()
    }

    /// Extracts DuckDB platform identifier from the build artifact path
    ///
    /// This method analyzes the target triple from the build directory
    /// structure and maps it to DuckDB platform identifiers.
    fn artifact_duckdb_platform(&self, filename: &Utf8PathBuf) -> Option<String> {
        filename
            .strip_prefix(&self.target_directory)
            .ok()
            .and_then(|path| path.components().next())
            .map(|target| Triple::from_str(target.as_str()))
            .and_then(Result::ok)
            .map(|triple| {
                let os = match triple.operating_system {
                    OperatingSystem::Linux => "linux",
                    OperatingSystem::MacOSX(_) => "osx",
                    OperatingSystem::Windows => "windows",
                    _ => panic!("Unsupported operating system: {}", triple),
                };
                let arch = match triple.architecture {
                    Architecture::X86_64 => "amd64",
                    Architecture::Aarch64(_) => "arm64",
                    Architecture::X86_32(_) => "amd",
                    Architecture::Arm(_) => "arm",
                    _ => panic!("Unsupported architecture: {}", triple),
                };
                format!("{os}_{arch}")
            })
    }

    /// Provides a default platform identifier based on the host system
    ///
    /// This is used when no target triple is detected in the build path,
    /// typically for native builds without explicit target specification.
    fn default_duckdb_platform() -> String {
        let os = match OS {
            "macos" => "osx",
            os => os,
        };
        let arch = match ARCH {
            "x86" => "amd",
            "x86_64" => "amd64",
            "aarch64" => "arm64",
            arch => arch,
        };
        format!("{os}_{arch}")
    }
}

/// Converts the version of the `duckdb`/`libduckdb-sys` crate to a DuckDB version string.
///
/// Starting with DuckDB 1.5.0, the `duckdb`/`libduckdb-sys` crate uses a special versioning scheme
/// where the minor component encodes the MAJOR, MINOR, and PATCH versions of DuckDB
/// as a single integer: `MAJOR * 10000 + MINOR * 100 + PATCH`.
/// For example, version `1.10502.0` corresponds to DuckDB `1.5.2`.
///
/// This function detects such encoded versions and decodes
/// them into the standard `vMAJOR.MINOR.PATCH` format expected by DuckDB.
/// All other version strings are returned with a 'v' prefix unchanged.
fn normalize_duckdb_version_from_crate_version(version: &Version) -> String {
    // Heuristic: if minor is large enough to encode a three-part version
    // (>= 10000, i.e., 5 or more digits), it's the new scheme.
    if version.minor >= 10000 {
        let major = version.minor / 10000;
        let minor = (version.minor % 10000) / 100;
        let patch = version.minor % 100;
        format!("v{}.{}.{}", major, minor, patch)
    } else {
        format!("v{}", version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_legacy_version_scheme() {
        let v = Version::parse("0.10.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v0.10.0");
        let v = Version::parse("1.4.2").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.4.2");
    }

    #[test]
    fn test_new_version_scheme() {
        // DuckDB 1.5.2  -> crate version 1.10502.0
        let v = Version::parse("1.10502.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.5.2");
        // DuckDB 2.1.0  -> crate version 2.20100.0 (crate major may differ)
        let v = Version::parse("2.20100.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v2.1.0");
        // DuckDB 1.4.2  -> crate version 1.10402.0
        let v = Version::parse("1.10402.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.4.2");
        // DuckDB 3.14.15 -> crate version 3.31415.0
        let v = Version::parse("3.31415.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v3.14.15");
    }

    #[test]
    fn test_small_minor_not_decoded() {
        // minor < 10000, treat as legacy.
        let v = Version::parse("1.123.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.123.0");
    }

    #[test]
    fn test_pre_release_ignored() {
        // Pre-release info is ignored; decoding still happens based on minor.
        let v = Version::parse("1.10502.0-alpha.1").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.5.2");
    }

    #[test]
    fn test_build_metadata_ignored() {
        let v = Version::parse("1.10502.0+build.42").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.5.2");
    }

    #[test]
    fn test_zero_padding() {
        // DuckDB 1.0.0 -> crate version 1.10000.0
        let v = Version::parse("1.10000.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v1.0.0");
        // DuckDB 0.1.5 -> crate version 0.105.0? (minor=105, <10000 -> legacy)
        let v = Version::parse("0.105.0").unwrap();
        assert_eq!(normalize_duckdb_version_from_crate_version(&v), "v0.105.0");
    }
}