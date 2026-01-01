# Image Viewer 🖼️

A modern, fast, cross-platform image viewer written in **Rust** with RAW support and GPU-accelerated previews (experimental).

![Rust](https://img.shields.io/badge/Rust-1.70+-orange) ![Platform](https://img.shields.io/badge/Platform-Windows%20|%20macOS%20|%20Linux-blue) ![License](https://img.shields.io/badge/License-MIT-green) ![Version](https://img.shields.io/badge/Version-2.0.0-blue)

---

## Highlights
- Cross-platform (Windows, macOS, Linux)
- RAW support (CR2/CR3, NEF, ARW, DNG, RAF, etc.)
- GPU-accelerated rendering for fast previews (optional)
- Maintain zoom & pan when navigating between photos (great for focus checks)
- Smart caching, keyboard shortcuts, and undoable file operations

## Quick Start
1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Clone and build:
   ```bash
   git clone https://github.com/image-viewer/image-viewer.git
   cd image-viewer
   cargo build --release
   ```
3. Run:
   ```bash
   cargo run --release
   ```


Test images are available in `testfiles/` for quick checks.

## Supported Formats
- Common: JPEG, PNG, GIF, BMP, TIFF, WebP, ICO, PNM
- RAW: Canon, Nikon, Sony, Olympus, Panasonic, Adobe DNG, Fuji, Pentax, and more

## Useful Shortcuts
- ← / → : prev / next image
- `1` : 100% zoom, `0` : fit to window
- `H` : toggle UI panels, `M` : move to 'selected' folder (undoable)
- `Del` : delete (to trash)

## Contributing
Bug reports and pull requests welcome — check issues and open a PR. Run tests with `cargo test`.

## License
MIT — see `LICENSE`.

---

Built with eframe/egui, image-rs, rawloader, imagepipe, imageproc, tokio, and serde. Thanks to the upstream projects and contributors.