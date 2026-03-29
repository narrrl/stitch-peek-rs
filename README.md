# stitch-peek-rs

A Nautilus/GNOME thumbnailer for **PES embroidery files**. Browse your embroidery designs in the file manager with automatic thumbnail previews.

Built as two crates:

| Crate | Description |
|-------|-------------|
| [**rustitch**](rustitch/) | Library for parsing PES files and rendering stitch data to images |
| [**stitch-peek**](stitch-peek/) | CLI thumbnailer that integrates with GNOME/Nautilus |

<!-- TODO: Add screenshot of Nautilus showing PES thumbnails -->

## Installation

### From .deb (Debian/Ubuntu)

Download the latest `.deb` from the [Releases](https://git.narl.io/nvrl/stitch-peek-rs/releases) page:

```sh
sudo dpkg -i stitch-peek_*_amd64.deb
```

This installs the binary, thumbnailer entry, and MIME type definition. Restart Nautilus to pick up the changes:

```sh
nautilus -q
```

### From crates.io

```sh
cargo install stitch-peek
```

Then install the data files:

```sh
sudo install -Dm644 data/stitch-peek.thumbnailer /usr/share/thumbnailers/stitch-peek.thumbnailer
sudo install -Dm644 data/pes.xml /usr/share/mime/packages/pes.xml
sudo update-mime-database /usr/share/mime
nautilus -q
```

### From source

Requires Rust 1.85+.

```sh
git clone https://git.narl.io/nvrl/stitch-peek-rs.git
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
rustitch = "0.1"
```

```rust
let pes_data = std::fs::read("design.pes")?;
let png_bytes = rustitch::thumbnail(&pes_data, 256)?;
std::fs::write("preview.png", &png_bytes)?;
```

See the [rustitch README](rustitch/README.md) for more API examples.

## Supported formats

**PES** (Brother PE-Design) embroidery files, versions 1 through 10. The PEC section containing stitch data is consistent across versions.

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

[MIT](LICENSE)
