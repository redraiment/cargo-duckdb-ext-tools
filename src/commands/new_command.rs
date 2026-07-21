//! Implementation of the `new` subcommand
//!
//! This module handles creation of new DuckDB extension projects,
//! including project scaffolding, dependency management, and
//! template code generation.

use crate::commands::Command;
use crate::creating;
use crate::error::ToolsError;
use crate::logging;
use crate::options::new_option::NewOptions;
use heck::ToPascalCase;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command as Process;
use std::process::Stdio;

/// Command for creating new DuckDB extension projects
///
/// This struct encapsulates all the state needed to generate a complete
/// DuckDB extension project, including both scalar and table functions.
pub(crate) struct NewCommand {
    /// Root directory of the new project
    root: PathBuf,
    /// Whether to generate a table function (vs scalar function)
    is_table_function: bool,
    /// Name of the generated function (snake_case)
    function_name: String,
    /// Name of the generated struct (PascalCase)
    struct_name: String,
    /// Arguments to pass to `cargo new`
    arguments: Vec<String>,
}

impl TryFrom<NewOptions> for NewCommand {
    type Error = ToolsError;

    /// Constructs a NewCommand from command line options
    ///
    /// This conversion prepares all the arguments for `cargo new`,
    /// determines the function and struct names, and sets up the
    /// project structure based on user preferences.
    fn try_from(options: NewOptions) -> Result<Self, Self::Error> {
        let mut arguments = vec![
            String::from("new"),
            String::from("--lib"),
        ];
        if let Some(vcs) = &options.vcs {
            arguments.push(String::from("--vcs"));
            arguments.push(vcs.to_string());
        }
        if let Some(edition) = &options.edition {
            arguments.push(String::from("--edition"));
            arguments.push(edition.to_string());
        }
        if let Some(name) = &options.name {
            arguments.push(String::from("--name"));
            arguments.push(name.to_string());
        }
        if let Some(registry) = &options.registry {
            arguments.push(String::from("--registry"));
            arguments.push(registry.to_string());
        }
        for configuration in &options.configuations {
            arguments.push(String::from("--config"));
            arguments.push(configuration.to_string());
        }

        let root = PathBuf::from(&options.path);
        let function_name = options.name.or_else(|| {
            root.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_lowercase().replace("-", "_"))
        }).expect("Missing project name");
        let struct_name = function_name.to_pascal_case();
        arguments.push(options.path);

        Ok(NewCommand {
            root,
            is_table_function: !options.scalar && options.table,
            function_name,
            struct_name,
            arguments,
        })
    }
}

impl Command for NewCommand {
    /// Executes the new project creation workflow
    ///
    /// This method performs the complete project creation process:
    /// 1. Generate basic cargo project
    /// 2. Update Cargo.toml with extension-specific settings
    /// 3. Add DuckDB dependencies
    /// 4. Generate template code in src/lib.rs
    fn execute(&mut self) -> Result<(), ToolsError> {
        self.generate()?;
        self.update_cargo_toml()?;
        self.add_dependencies()?;
        self.update_src_lib_rs()?;
        Ok(())
    }
}

impl NewCommand {
    /// Executes `cargo new` to create the basic project structure
    ///
    /// This runs the actual `cargo new` command with all the configured
    /// arguments to create a new Rust library project.
    fn generate(&self) -> Result<(), ToolsError> {
        creating!("Cargo {:?}", self.arguments);
        Self::cargo(&self.arguments)
    }

    /// Updates Cargo.toml with extension-specific configuration
    ///
    /// This adds the `cdylib` crate type for dynamic library output
    /// and configures release profile optimizations (LTO and stripping).
    fn update_cargo_toml(&self) -> Result<(), ToolsError> {
        let path = self.root.join("Cargo.toml");
        creating!("Update Cargo.toml ({})", path.display());
        let mut file = OpenOptions::new().append(true).open(path)?;
        writeln!(file, "{}", r#"
[lib]
crate-type = ["cdylib"]

[profile.release]
lto = true
strip = true"#)?;
        Ok(())
    }

    /// Adds DuckDB extension dependencies to the project
    ///
    /// This uses `cargo add` to add the necessary DuckDB crates:
    /// - `duckdb` with appropriate features (vtab-loadable or vscalar)
    /// - `libduckdb-sys` with loadable-extension feature
    /// - `duckdb-ext-macros` for the extension entry point macro
    fn add_dependencies(&self) -> Result<(), ToolsError> {
        let path = self.root.join("Cargo.toml").display().to_string();
        creating!("Cargo add duckdb");
        Self::cargo(&["add", "--manifest-path", path.as_str(), "duckdb", "--features", if self.is_table_function { "vtab-loadable" } else { "vscalar" }])?;
        creating!("Cargo add libduckdb-sys");
        Self::cargo(&["add", "--manifest-path", path.as_str(), "libduckdb-sys", "--features", "loadable-extension"])?;
        creating!("Cargo add duckdb-ext-macros");
        Self::cargo(&["add", "--manifest-path", path.as_str(), "duckdb-ext-macros"])?;
        Ok(())
    }

    /// Generates the template code in src/lib.rs
    ///
    /// This writes either a table function or scalar function template
    /// based on user selection, with proper name substitution.
    fn update_src_lib_rs(&self) -> Result<(), ToolsError> {
        creating!("Write lib.rs");
        let path = self.root.join("src").join("lib.rs");
        let mut file = OpenOptions::new().write(true).open(path)?;
        let code = if self.is_table_function { TABLE_FUNCTION_TEMPLATE } else { SCALAR_FUNCTION_TEMPLATE };
        let code = code.replace("{{struct-name}}", &self.struct_name)
            .replace("{{function-name}}", &format!("{:?}", self.function_name));
        writeln!(file, "{}", code)?;
        Ok(())
    }

    /// Executes a cargo command with the given arguments
    ///
    /// # Arguments
    /// * `args` - Arguments to pass to the cargo command
    ///
    /// # Type Parameters
    /// * `S` - String-like type that can be converted to OsStr
    /// * `I` - Iterator over arguments
    fn cargo<S, I>(args: I) -> Result<(), ToolsError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut process = Process::new("cargo");
        process.args(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        process.spawn()?.wait()?;
        Ok(())
    }
}

/// Template for DuckDB table function extensions
///
/// This template provides a complete table function implementation
/// with bind data, init data, and function execution logic.
const TABLE_FUNCTION_TEMPLATE: &str = r#"use duckdb::core::DataChunkHandle;
use duckdb::core::Inserter;
use duckdb::core::LogicalTypeHandle;
use duckdb::core::LogicalTypeId;
use duckdb::vtab::BindInfo;
use duckdb::vtab::InitInfo;
use duckdb::vtab::TableFunctionInfo;
use duckdb::vtab::VTab;
use duckdb::Connection;
use duckdb_ext_macros::duckdb_extension;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

#[repr(C)]
pub(crate) struct {{struct-name}}BindData {
    name: String,
}

#[repr(C)]
pub(crate) struct {{struct-name}}InitData {
    done: AtomicBool,
}

struct {{struct-name}}TableFunction;

impl VTab for {{struct-name}}TableFunction {
    type InitData = {{struct-name}}InitData;
    type BindData = {{struct-name}}BindData;

    fn bind(bind: &BindInfo) -> duckdb::Result<Self::BindData, Box<dyn Error>> {
        bind.add_result_column("🐥", LogicalTypeHandle::from(LogicalTypeId::Varchar));
        Ok(Self::BindData {
            name: bind.get_parameter(0).to_string(),
        })
    }

    fn init(_init: &InitInfo) -> duckdb::Result<Self::InitData, Box<dyn Error>> {
        Ok(Self::InitData {
            done: AtomicBool::new(false),
        })
    }

    fn func(
        func: &TableFunctionInfo<Self>,
        output: &mut DataChunkHandle,
    ) -> duckdb::Result<(), Box<dyn Error>> {
        let bind = func.get_bind_data();
        let init = func.get_init_data();
        if init.done.swap(true, Ordering::Relaxed) {
            output.set_len(0);
        } else {
            let vector = output.flat_vector(0);
            vector.insert(0, format!("Hello {}", bind.name).as_str());
            output.set_len(1);
        }
        Ok(())
    }

    fn parameters() -> Option<Vec<LogicalTypeHandle>> {
        Some(vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)])
    }
}

/// DuckDB extension entry point.
#[duckdb_extension]
pub fn extension_entrypoint(connection: Connection) -> Result<(), Box<dyn Error>> {
    connection.register_table_function::<{{struct-name}}TableFunction>({{function-name}})?;
    Ok(())
}
"#;
/// Template for DuckDB scalar function extensions
///
/// This template provides a complete scalar function implementation
/// with parameter handling and return value generation.
const SCALAR_FUNCTION_TEMPLATE: &str = r#"use duckdb::core::DataChunkHandle;
use duckdb::core::Inserter;
use duckdb::core::LogicalTypeHandle;
use duckdb::core::LogicalTypeId;
use duckdb::types::DuckString;
use duckdb::vscalar::ScalarFunctionSignature;
use duckdb::vscalar::VScalar;
use duckdb::vtab::arrow::WritableVector;
use duckdb::Connection;
use duckdb_ext_macros::duckdb_extension;
use libduckdb_sys::duckdb_string_t;
use std::error::Error;

struct {{struct-name}}ScalarFunction;

impl VScalar for {{struct-name}}ScalarFunction {
    type State = ();

    unsafe fn invoke(_: &Self::State, input: &mut DataChunkHandle, output: &mut dyn WritableVector) -> Result<(), Box<dyn Error>> {
        let mut parameters = input.flat_vector(0);
        let returns = output.flat_vector();
        for parameter in parameters.as_mut_slice_with_len::<duckdb_string_t>(input.len()) {
            let mut name = DuckString::new(parameter);
            returns.insert(0, &format!("Hello {}!", name.as_str()));
        }
        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        vec![ScalarFunctionSignature::exact(
            vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
            LogicalTypeHandle::from(LogicalTypeId::Varchar),
        )]
    }
}

/// DuckDB extension entry point.
#[duckdb_extension]
pub fn extension_entrypoint(connection: Connection) -> Result<(), Box<dyn Error>> {
    connection.register_scalar_function::<{{struct-name}}ScalarFunction>({{function-name}})?;
    Ok(())
}
"#;