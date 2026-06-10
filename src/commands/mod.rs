//! Command implementations for cargo-duckdb-ext subcommands
//!
//! This module defines the `Command` trait and provides implementations
//! for the `new`, `build`, `meta`, and `package` subcommands.

mod build_command;
mod meta_command;
mod new_command;
mod package_command;

use crate::commands::build_command::BuildCommand;
use crate::commands::meta_command::MetaCommand;
pub(crate) use crate::commands::meta_command::STABLE_ABI_TYPE;
pub(crate) use crate::commands::meta_command::UNSTABLE_ABI_TYPE;
use crate::commands::new_command::NewCommand;
use crate::commands::package_command::PackageCommand;
use crate::error::ToolsError;
use crate::logging;
use crate::options::Commands;
use crate::options::Options;

/// Trait defining the interface for all subcommands
///
/// Each subcommand must implement this trait to provide a consistent
/// execution interface for the main application.
pub trait Command {
    /// Executes the command with its configured options
    ///
    /// # Returns
    /// `Ok(())` on success, or a `ToolsError` if execution fails
    fn execute(&mut self) -> Result<(), ToolsError>;
}

impl TryInto<Box<dyn Command>> for Options {
    type Error = ToolsError;

    /// Converts parsed command line options into an executable command
    ///
    /// This method initializes logging based on the quiet flag and
    /// dispatches to the appropriate command implementation.
    fn try_into(self) -> Result<Box<dyn Command>, Self::Error> {
        logging::init(self.quiet);
        let task: Box<dyn Command> = match self.command {
            Commands::Build(options) => Box::new(BuildCommand::try_from(options)?),
            Commands::Meta(options) => Box::new(MetaCommand::try_from(options)?),
            Commands::Package(options) => Box::new(PackageCommand::try_from(options)?),
            Commands::New(options) => Box::new(NewCommand::try_from(options)?),
        };
        Ok(task)
    }
}
