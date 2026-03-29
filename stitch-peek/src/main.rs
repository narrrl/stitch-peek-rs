use anyhow::{Context, Result};
use clap::Parser;
use std::fs;

#[derive(Parser)]
#[command(name = "stitch-peek", about = "PES embroidery file thumbnailer")]
struct Args {
    /// Input PES file path
    #[arg(short = 'i', long = "input")]
    input: std::path::PathBuf,

    /// Output PNG file path
    #[arg(short = 'o', long = "output")]
    output: std::path::PathBuf,

    /// Thumbnail size in pixels
    #[arg(short = 's', long = "size", default_value = "128")]
    size: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let data = fs::read(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;

    let png =
        rustitch::thumbnail(&data, args.size).with_context(|| "failed to generate thumbnail")?;

    fs::write(&args.output, &png)
        .with_context(|| format!("failed to write {}", args.output.display()))?;

    Ok(())
}
