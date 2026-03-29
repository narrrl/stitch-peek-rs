# stitch-peek

A CLI tool and **Nautilus/GNOME thumbnailer** for PES embroidery files. Generates PNG previews of embroidery designs directly in your file manager.

Part of the [stitch-peek-rs](https://git.narl.io/nvrl/stitch-peek-rs) project. Uses [rustitch](https://crates.io/crates/rustitch) for PES parsing and rendering.

## Installation

### From .deb (Debian/Ubuntu)

Download the latest `.deb` from the [Releases](https://git.narl.io/nvrl/stitch-peek-rs/releases) page:

```sh
sudo dpkg -i stitch-peek_*_amd64.deb
```

### From crates.io

```sh
cargo install stitch-peek
```

Then install the thumbnailer and MIME type files manually:

```sh
sudo install -Dm644 data/stitch-peek.thumbnailer /usr/share/thumbnailers/stitch-peek.thumbnailer
sudo install -Dm644 data/pes.xml /usr/share/mime/packages/pes.xml
sudo update-mime-database /usr/share/mime
```

### From source

```sh
git clone https://git.narl.io/nvrl/stitch-peek-rs.git
cd stitch-peek-rs
cargo install --path stitch-peek
```

After installing, restart Nautilus to pick up the thumbnailer:

```sh
nautilus -q
```

## Usage

### As a thumbnailer

Once installed with the `.thumbnailer` file in place, Nautilus automatically generates thumbnails for `.pes` files. No manual action needed.

### Standalone CLI

```sh
stitch-peek -i design.pes -o preview.png -s 256
```

| Flag | Description | Default |
|------|-------------|---------|
| `-i` | Input PES file | required |
| `-o` | Output PNG path | required |
| `-s` | Thumbnail size (pixels) | 128 |

## How it works

The tool follows the [freedesktop thumbnail specification](https://specifications.freedesktop.org/thumbnail/latest/). Nautilus calls it with an input file, output path, and requested size. It parses the PES file, renders the stitch pattern as anti-aliased colored lines on a transparent background, and writes a PNG.

## License

MIT
