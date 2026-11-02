//! Command line entrypoint of `krabik`.

use clap::Parser;

use krabik::{Args, run};

/// CLI entrypoint. Parses command-line arguments and dispatches to [`run`].
fn main() -> anyhow::Result<()> {
    run(Args::parse())
}
