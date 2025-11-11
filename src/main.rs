//! A simple command line tool to run Qwen 2.5 models using Candle.
//!
//! # Example usage
//! ```sh
//! cargo run --release -- --which 0.5b --prompt "Hello, world!"
//! ```
use anyhow::Result;
use clap::Parser;

use candle_qwen2_5::{run, Args};

fn main() -> Result<()> {
    use tracing_chrome::ChromeLayerBuilder;
    use tracing_subscriber::prelude::*;

    let args = Args::parse();
    let _guard = if args.tracing {
        let (chrome_layer, guard) = ChromeLayerBuilder::new().build();
        tracing_subscriber::registry().with(chrome_layer).init();
        Some(guard)
    } else {
        None
    };
    run(args)
}

