//! Implementation of the `meta` subcommand
//!
//! This module owns DuckDB extension metadata footer handling. It can
//! read, update, append, and display the 534-byte metadata footer used by
//! DuckDB extension files, and is reused by the `package` subcommand.

use crate::commands::Command;
use crate::error::ToolsError;
use crate::logging;
use crate::options::meta_options::MetaOptions;
use crate::packing;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

/// Total byte length of a DuckDB extension metadata footer
const METADATA_LENGTH: usize = 534;
/// Byte length of the fixed metadata header signature
const HEADER_LENGTH: usize = 22;
/// Byte length of regular string or reserved metadata fields
const FIELD_LENGTH: usize = 32;
/// Byte length of the signature area at the end of the footer
const SIGNATURE_LENGTH: usize = 256;
/// Current magic version stored by DuckDB extension metadata
const DEFAULT_MAGIC_VERSION: &str = "4";

/// Fixed byte sequence that identifies a DuckDB extension metadata footer
const HEADER: [u8; HEADER_LENGTH] = [
    0, 147, 4, 16, 100, 117, 99, 107, 100, 98, 95, 115, 105, 103, 110, 97, 116, 117, 114, 101, 128, 4,
];

/// ABI type for extensions built against stable DuckDB C API
pub(crate) const STABLE_ABI_TYPE: &'static str = "C_STRUCT";
/// ABI type for extensions built against unstable DuckDB C API
pub(crate) const UNSTABLE_ABI_TYPE: &'static str = "C_STRUCT_UNSTABLE";

/// Command for reading and updating DuckDB extension metadata
///
/// This struct stores the target extension file and an optional metadata
/// update. Without an update, the command only displays current metadata.
pub(crate) struct MetaCommand {
    /// Path to the extension file to inspect or update
    extension_file: String,
    /// Parsed metadata fields update requested by command line options
    fields: Fields,
}

impl TryFrom<MetaOptions> for MetaCommand {
    type Error = ToolsError;

    /// Constructs a MetaCommand from command line options
    ///
    /// This conversion parses hexadecimal metadata fields and prepares
    /// fixed-width values for later in-place updates.
    fn try_from(options: MetaOptions) -> Result<Self, Self::Error> {
        let fields = Fields::try_from(&options)?;
        Ok(Self {
            extension_file: options.extension_file,
            fields,
        })
    }
}

impl Command for MetaCommand {
    /// Executes the metadata read or update operation
    ///
    /// This method detects whether the target file already contains a
    /// footer, optionally writes the requested changes, and displays the
    /// resulting metadata.
    fn execute(&mut self) -> Result<(), ToolsError> {
        let mut file = self.open_file()?;
        let metadata = ExtensionMetadata::read_from(&mut file)?;
        let has_metadata = metadata.is_some();
        let mut metadata = metadata.unwrap_or_else(ExtensionMetadata::empty);

        if self.fields.should_update() {
            if !has_metadata {
                metadata = ExtensionMetadata::blank_footer();
            }
            self.fields.apply_to(&mut metadata);
            metadata.write_to(&mut file, has_metadata)?;
        }

        metadata.display();
        Ok(())
    }
}

impl MetaCommand {
    /// Opens the extension file for reading or editing
    ///
    /// Read-only mode uses a read handle. Edit mode uses a read/write
    /// handle so an existing footer can be overwritten in place.
    fn open_file(&self) -> Result<File, ToolsError> {
        let file = if self.fields.should_update() {
            OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.extension_file)?
        } else {
            OpenOptions::new().read(true).open(&self.extension_file)?
        };
        Ok(file)
    }
}

/// Parsed optional metadata field updates
///
/// Each field is optional so unspecified command line options can preserve
/// existing metadata values or stay empty when creating a new footer.
#[derive(Default)]
struct Fields {
    /// First reserved 32-byte field
    field_1: Option<[u8; FIELD_LENGTH]>,
    /// Second reserved 32-byte field
    field_2: Option<[u8; FIELD_LENGTH]>,
    /// Third reserved 32-byte field
    field_3: Option<[u8; FIELD_LENGTH]>,
    /// ABI type field implied by DuckDB version options
    abi_type: Option<[u8; FIELD_LENGTH]>,
    /// Extension version field
    extension_version: Option<[u8; FIELD_LENGTH]>,
    /// DuckDB version or DuckDB C API version field
    duckdb_version: Option<[u8; FIELD_LENGTH]>,
    /// DuckDB platform identifier field
    duckdb_platform: Option<[u8; FIELD_LENGTH]>,
    /// Magic version field
    magic_version: Option<[u8; FIELD_LENGTH]>,
    /// Signature area field
    signature: Option<[u8; SIGNATURE_LENGTH]>,
}

impl TryFrom<&MetaOptions> for Fields {
    type Error = ToolsError;

    /// Parses command line values into fixed-width metadata field updates
    fn try_from(options: &MetaOptions) -> Result<Self, Self::Error> {
        Ok(Self {
            field_1: parse_optional_hex_field(&options.field_1, FIELD_LENGTH, "field-1")?,
            field_2: parse_optional_hex_field(&options.field_2, FIELD_LENGTH, "field-2")?,
            field_3: parse_optional_hex_field(&options.field_3, FIELD_LENGTH, "field-3")?,
            abi_type: parse_optional_string_field(&options.abi_type(), "ABI type")?,
            extension_version: parse_optional_string_field(&options.extension_version, "extension version")?,
            duckdb_version: parse_optional_string_field(&options.version(), "DuckDB/C API version")?,
            duckdb_platform: parse_optional_string_field(&options.duckdb_platform, "DuckDB platform")?,
            magic_version: parse_optional_string_field(&options.magic_version, "magic version")?,
            signature: parse_optional_hex_field(&options.signature, SIGNATURE_LENGTH, "signature")?,
        })
    }
}

impl Fields {
    /// Returns whether any metadata field should be updated
    fn should_update(&self) -> bool {
        self.field_1.is_some()
            || self.field_2.is_some()
            || self.field_3.is_some()
            || self.abi_type.is_some()
            || self.extension_version.is_some()
            || self.duckdb_version.is_some()
            || self.duckdb_platform.is_some()
            || self.magic_version.is_some()
            || self.signature.is_some()
    }

    /// Applies all specified fields to a metadata footer
    fn apply_to(&self, metadata: &mut ExtensionMetadata) {
        if let Some(field) = self.field_1 {
            metadata.field_1 = field;
        }
        if let Some(field) = self.field_2 {
            metadata.field_2 = field;
        }
        if let Some(field) = self.field_3 {
            metadata.field_3 = field;
        }
        if let Some(field) = self.abi_type {
            metadata.abi_type = field;
        }
        if let Some(field) = self.extension_version {
            metadata.extension_version = field;
        }
        if let Some(field) = self.duckdb_version {
            metadata.duckdb_version = field;
        }
        if let Some(field) = self.duckdb_platform {
            metadata.duckdb_platform = field;
        }
        if let Some(field) = self.magic_version {
            metadata.magic_version = field;
        }
        if let Some(field) = self.signature {
            metadata.signature = field;
        }
    }
}

/// DuckDB extension metadata footer fields
///
/// This struct mirrors the fixed footer layout and keeps all fields in
/// their encoded byte representation.
#[derive(Clone)]
pub(crate) struct ExtensionMetadata {
    /// Fixed metadata header signature
    header: [u8; HEADER_LENGTH],
    /// First reserved 32-byte field
    field_1: [u8; FIELD_LENGTH],
    /// Second reserved 32-byte field
    field_2: [u8; FIELD_LENGTH],
    /// Third reserved 32-byte field
    field_3: [u8; FIELD_LENGTH],
    /// ABI type field (`C_STRUCT` or `C_STRUCT_UNSTABLE`)
    abi_type: [u8; FIELD_LENGTH],
    /// Extension version field
    extension_version: [u8; FIELD_LENGTH],
    /// DuckDB version or DuckDB C API version field
    duckdb_version: [u8; FIELD_LENGTH],
    /// DuckDB platform identifier field
    duckdb_platform: [u8; FIELD_LENGTH],
    /// Metadata magic version field
    magic_version: [u8; FIELD_LENGTH],
    /// Official signature or zero-filled unsigned marker
    signature: [u8; SIGNATURE_LENGTH],
}

impl ExtensionMetadata {
    /// Creates an empty metadata value for displaying absent footers
    fn empty() -> Self {
        Self {
            header: [0; HEADER_LENGTH],
            field_1: [0; FIELD_LENGTH],
            field_2: [0; FIELD_LENGTH],
            field_3: [0; FIELD_LENGTH],
            abi_type: [0; FIELD_LENGTH],
            extension_version: [0; FIELD_LENGTH],
            duckdb_version: [0; FIELD_LENGTH],
            duckdb_platform: [0; FIELD_LENGTH],
            magic_version: [0; FIELD_LENGTH],
            signature: [0; SIGNATURE_LENGTH],
        }
    }

    /// Creates a writable blank metadata footer with the current defaults
    ///
    /// The header signature is always present in written footers. The magic
    /// version defaults to `4` when creating new metadata.
    fn blank_footer() -> Self {
        let mut metadata = Self::empty();
        metadata.header = HEADER;
        metadata.magic_version[0] = DEFAULT_MAGIC_VERSION.as_bytes()[0];
        metadata
    }

    /// Creates complete metadata for the `package` subcommand
    ///
    /// This preserves the existing packaging defaults: empty reserved and
    /// signature fields, ABI/version/platform values, and magic version `4`.
    pub(crate) fn package(
        abi_type: &str,
        extension_version: &str,
        duckdb_version: &str,
        duckdb_platform: &str,
    ) -> Result<Self, ToolsError> {
        let mut metadata = Self::blank_footer();
        metadata.abi_type = string_field(abi_type, "ABI type")?;
        metadata.extension_version = string_field(extension_version, "extension version")?;
        metadata.duckdb_version = string_field(duckdb_version, "DuckDB/C API version")?;
        metadata.duckdb_platform = string_field(duckdb_platform, "DuckDB platform")?;
        Ok(metadata)
    }

    /// Reads the metadata footer from the end of a file, if present
    ///
    /// A footer is considered present only when the last 534 bytes start
    /// with the fixed DuckDB metadata signature.
    fn read_from(file: &mut File) -> Result<Option<Self>, ToolsError> {
        let length = file.metadata()?.len();
        if length < METADATA_LENGTH as u64 {
            return Ok(None);
        }

        file.seek(SeekFrom::End(-(METADATA_LENGTH as i64)))?;
        let mut bytes = [0u8; METADATA_LENGTH];
        file.read_exact(&mut bytes)?;
        if bytes[..HEADER_LENGTH] != HEADER {
            return Ok(None);
        }

        let mut offset = 0;
        Ok(Some(Self {
            header: next_field(&bytes, &mut offset),
            field_1: next_field(&bytes, &mut offset),
            field_2: next_field(&bytes, &mut offset),
            field_3: next_field(&bytes, &mut offset),
            abi_type: next_field(&bytes, &mut offset),
            extension_version: next_field(&bytes, &mut offset),
            duckdb_version: next_field(&bytes, &mut offset),
            duckdb_platform: next_field(&bytes, &mut offset),
            magic_version: next_field(&bytes, &mut offset),
            signature: next_field(&bytes, &mut offset),
        }))
    }

    /// Writes the footer by appending it or replacing an existing footer
    fn write_to(&self, file: &mut File, replace_existing: bool) -> Result<(), ToolsError> {
        let offset = if replace_existing { METADATA_LENGTH } else { 0 };
        file.seek(SeekFrom::End(-(offset as i64)))?;
        file.write_all(&self.to_bytes())?;
        Ok(())
    }

    /// Appends the footer to the end of a package output file
    pub(crate) fn append_to(&self, file: &mut File) -> Result<(), ToolsError> {
        self.write_to(file, false)
    }

    /// Displays metadata fields in a structured, Cargo-style format
    pub(crate) fn display(&self) {
        packing!("Header Signature ({})", hex_string(&self.header));
        packing!("Reserved Field 1 ({})", hex_string(&self.field_1));
        packing!("Reserved Field 2 ({})", hex_string(&self.field_2));
        packing!("Reserved Field 3 ({})", hex_string(&self.field_3));
        let abi_type = string_value(&self.abi_type);
        packing!("ABI Type ({})", abi_type);
        packing!("Extension Version ({})", string_value(&self.extension_version));
        let version_kind = if abi_type == STABLE_ABI_TYPE {
            "DuckDB C API Version"
        } else {
            "DuckDB Version"
        };
        packing!("{} ({})", version_kind, string_value(&self.duckdb_version));
        packing!("DuckDB Platform ({})", string_value(&self.duckdb_platform));
        packing!("Magic Version ({})", string_value(&self.magic_version));
        packing!("Signature ({})", hex_string(&self.signature));
    }

    /// Encodes metadata fields into the fixed 534-byte footer layout
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(METADATA_LENGTH);
        bytes.extend_from_slice(&self.header);
        bytes.extend_from_slice(&self.field_1);
        bytes.extend_from_slice(&self.field_2);
        bytes.extend_from_slice(&self.field_3);
        bytes.extend_from_slice(&self.abi_type);
        bytes.extend_from_slice(&self.extension_version);
        bytes.extend_from_slice(&self.duckdb_version);
        bytes.extend_from_slice(&self.duckdb_platform);
        bytes.extend_from_slice(&self.magic_version);
        bytes.extend_from_slice(&self.signature);
        debug_assert_eq!(bytes.len(), METADATA_LENGTH);
        bytes
    }
}

/// Reads the next fixed-size field from a metadata byte buffer
fn next_field<const N: usize>(bytes: &[u8], offset: &mut usize) -> [u8; N] {
    let mut field = [0u8; N];
    field.copy_from_slice(&bytes[*offset..*offset + N]);
    *offset += N;
    field
}

/// Parses an optional string field into the 32-byte footer representation
fn parse_optional_string_field(
    value: &Option<String>,
    field_name: &str,
) -> Result<Option<[u8; FIELD_LENGTH]>, ToolsError> {
    value.as_ref()
        .map(|value| string_field(value, field_name))
        .transpose()
}

/// Encodes a string into a zero-padded 32-byte metadata field
fn string_field(value: &str, field_name: &str) -> Result<[u8; FIELD_LENGTH], ToolsError> {
    let bytes = value.as_bytes();
    if bytes.len() > FIELD_LENGTH {
        return Err(ToolsError::InvalidArgument(format!("{field_name} must be at most {FIELD_LENGTH} bytes")));
    }

    let mut field = [0u8; FIELD_LENGTH];
    field[..bytes.len()].copy_from_slice(bytes);
    Ok(field)
}

/// Parses an optional hexadecimal field into a fixed-size byte array
fn parse_optional_hex_field<const N: usize>(
    value: &Option<String>,
    max_length: usize,
    field_name: &str,
) -> Result<Option<[u8; N]>, ToolsError> {
    value.as_ref()
        .map(|value| hex_field(value, max_length, field_name))
        .transpose()
}

/// Decodes lowercase hexadecimal text into a zero-padded metadata field
fn hex_field<const N: usize>(
    value: &str,
    max_length: usize,
    field_name: &str,
) -> Result<[u8; N], ToolsError> {
    if value.len() % 2 != 0 {
        return Err(ToolsError::InvalidArgument(format!("{field_name} must contain an even number of hexadecimal characters")));
    }
    if value.len() / 2 > max_length {
        return Err(ToolsError::InvalidArgument(format!("{field_name} must be at most {max_length} bytes")));
    }
    if !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(ToolsError::InvalidArgument(format!("{field_name} must be lowercase hexadecimal")));
    }

    let mut field = [0u8; N];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(chunk[0]);
        let low = hex_value(chunk[1]);
        field[index] = (high << 4) | low;
    }
    Ok(field)
}

/// Converts a validated hexadecimal character into its numeric value
fn hex_value(byte: u8) -> u8 {
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        _ => unreachable!("hex input is validated before conversion"),
    }
}

/// Formats raw bytes as lowercase hexadecimal text
fn hex_string(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 0x0f) as usize] as char);
    }
    value
}

/// Converts a zero-padded metadata string field into display text
fn string_value(bytes: &[u8]) -> String {
    let length = bytes
        .iter()
        .rposition(|byte| *byte != 0)
        .map(|index| index + 1)
        .unwrap_or(0);
    String::from_utf8_lossy(&bytes[..length]).to_string()
}
