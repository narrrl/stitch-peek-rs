# stitch-peek

A Nautilus/GNOME thumbnailer for **PES embroidery files**. Browse your embroidery designs in the file manager with automatic thumbnail previews.

Built as two crates:

- **rustitch** -- library for parsing PES files and rendering stitch data to images
- **stitch-peek** -- CLI thumbnailer that integrates with GNOME/Nautilus via the freedesktop thumbnail spec

<!-- TODO: Add screenshot of Nautilus showing PES thumbnails -->

## Installation

### From .deb (Debian/Ubuntu)

Download the latest `.deb` from the [Releases](../../releases) page:

```sh
sudo dpkg -i stitch-peek_*_amd64.deb
```

This installs the binary, thumbnailer entry, and MIME type definition. Restart Nautilus to pick up the changes:

```sh
nautilus -q
```

### From source

Requires Rust 1.70+.

```sh
git clone https://github.com/YOUR_USER/stitch-peek-rs.git
cd stitch-peek-rs
cargo build --release

sudo install -Dm755 target/release/stitch-peek /usr/local/bin/stitch-peek
sudo install -Dm644 data/stitch-peek.thumbnailer /usr/share/thumbnailers/stitch-peek.thumbnailer
sudo install -Dm644 data/pes.xml /usr/share/mime/packages/pes.xml
sudo update-mime-database /usr/share/mime
nautilus -q
```

## Usage

### As a thumbnailer

Once installed, Nautilus will automatically generate thumbnails for `.pes` files. No manual action needed -- just open a folder containing PES files.

### Standalone CLI

Generate a thumbnail manually:

```sh
stitch-peek -i design.pes -o preview.png -s 256
```

| Flag | Description | Default |
|------|-------------|---------|
| `-i` | Input PES file | required |
| `-o` | Output PNG path | required |
| `-s` | Thumbnail size (pixels) | 128 |

### As a library

Add `rustitch` to your project:

```toml
[dependencies]
rustitch = { git = "https://github.com/YOUR_USER/stitch-peek-rs.git" }
```

```rust
let pes_data = std::fs::read("design.pes")?;
let png_bytes = rustitch::thumbnail(&pes_data, 256)?;
std::fs::write("preview.png", &png_bytes)?;
```

## How it works

1. **Parse** the PES binary format -- extract the PEC section containing stitch commands and thread color indices
2. **Decode** the stitch byte stream into movement commands (stitches, jumps, trims, color changes)
3. **Resolve** relative movements into absolute coordinates grouped by thread color
4. **Render** anti-aliased line segments onto a transparent canvas using [tiny-skia](https://github.com/nickel-org/tiny-skia), scaled to fit the requested thumbnail size
5. **Encode** the result as a PNG image

## Supported formats

Currently supports **PES** (Brother PE-Design) embroidery files, versions 1 through 6. The PEC section -- which contains the actual stitch data -- is consistent across versions.

## Project structure

```
stitch-peek-rs/
├── rustitch/              # Library crate
│   └── src/
│       ├── lib.rs         # Public API
│       ├── pes/           # PES format parser
│       │   ├── header.rs  # File header (#PES magic, version, PEC offset)
│       │   ├── pec.rs     # PEC section (colors, stitch decoding)
│       │   └── palette.rs # Brother 65-color thread palette
│       └── render.rs      # tiny-skia renderer
├── stitch-peek/           # Binary crate (CLI thumbnailer)
│   └── src/main.rs
└── data/
    ├── stitch-peek.thumbnailer  # Nautilus integration
    └── pes.xml                  # MIME type definition
```

## Development

```sh
cargo test          # run all tests
cargo clippy        # lint
cargo fmt --check   # check formatting
```

Pull requests must bump the version in `stitch-peek/Cargo.toml` -- CI will reject merges without a version bump.

## Contributing

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/my-change`)
3. Make your changes and add tests where appropriate
4. Ensure `cargo test && cargo clippy && cargo fmt --check` pass
5. Bump the version in `stitch-peek/Cargo.toml`
6. Open a pull request

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
