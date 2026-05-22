pub mod cli;
pub mod command_run;
pub mod diff;
pub mod event;
pub mod git;
pub mod log_io;
pub mod repair;
pub mod summary;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
