use clap::Parser;

use mobile_cli::{Args, run};

/// CLI entrypoint. Parses command-line arguments and dispatches to [`run`].
fn main() -> anyhow::Result<()> {
    run(Args::parse())
}
