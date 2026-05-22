pub mod cli;
pub mod event;
pub mod log_io;
pub mod summary;

use anyhow::Result;

fn main() -> Result<()> {
    cli::run()
}
