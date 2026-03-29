# rustitch

[![crates.io](https://img.shields.io/crates/v/rustitch)](https://crates.io/crates/rustitch)
[![docs.rs](https://img.shields.io/docsrs/rustitch)](https://docs.rs/rustitch)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../LICENSE)

A Rust library for parsing **PES embroidery files** and rendering stitch data to images.

Part of the [stitch-peek-rs](https://git.narl.io/nvrl/stitch-peek-rs) project.

## Usage

Add `rustitch` to your `Cargo.toml`:

```toml
[dependencies]
rustitch = "0.1"
```

### Generate a thumbnail

```rust
let pes_data = std::fs::read("design.pes")?;
let png_bytes = rustitch::thumbnail(&pes_data, 256)?;
std::fs::write("preview.png", &png_bytes)?;
```

### Parse and inspect a design

```rust
use rustitch::pes::{self, StitchCommand};

let data = std::fs::read("design.pes")?;
let design = pes::parse(&data)?;

println!("PES version: {}", std::str::from_utf8(&design.header.version).unwrap());
println!("Label: {}", design.pec_header.label);
println!("Colors: {}", design.pec_header.color_count);

let stitch_count = design.commands.iter()
    .filter(|c| matches!(c, StitchCommand::Stitch { .. }))
    .count();
println!("Stitches: {stitch_count}");
```

### Resolve and render manually

```rust
use rustitch::pes;

let data = std::fs::read("design.pes")?;
let design = pes::parse(&data)?;
let resolved = pes::resolve(&design)?;

println!("Segments: {}", resolved.segments.len());
println!("Bounding box: ({}, {}) to ({}, {})",
    resolved.bounds.min_x, resolved.bounds.min_y,
    resolved.bounds.max_x, resolved.bounds.max_y);

let png_bytes = rustitch::render_thumbnail(&resolved, 512)?;
std::fs::write("large_preview.png", &png_bytes)?;
```

## Supported formats

**PES** (Brother PE-Design) embroidery files, versions 1 through 10. The PEC section containing stitch data is consistent across versions.

## How it works

1. **Parse** the PES binary header to locate the PEC section
2. **Decode** the PEC stitch byte stream (7-bit and 12-bit encoded relative movements, jumps, trims, color changes)
3. **Resolve** relative movements into absolute coordinate segments grouped by thread color, using the 65-color Brother PEC palette
4. **Render** anti-aliased line segments with [tiny-skia](https://github.com/nickel-org/tiny-skia), scaled to fit the requested size
5. **Encode** as PNG with proper alpha handling

## License

MIT
