# rView 🦀

**rView** — A modern, blazing-fast image viewer built with **Rust**. RAW support, GPU-accelerated previews, and a clean interface.

![Rust](https://img.shields.io/badge/Rust-1.70+-B7410E?logo=rust) ![Platform](https://img.shields.io/badge/Platform-Windows%20|%20macOS%20|%20Linux-blue) ![License](https://img.shields.io/badge/License-MIT-green) ![Version](https://img.shields.io/badge/Version-0.5.0-B7410E)

<p align="center">
  <img src="assets/rview-logo.svg" alt="rView Logo" width="128">
</p>

---

## ✨ Highlights
- 🖥️ **Cross-platform** — Windows, macOS, Linux
- 📷 **RAW support** — CR2/CR3, NEF, ARW, DNG, RAF, and more
- ⚡ **GPU-accelerated** — Fast previews with optional GPU rendering
- 🔍 **Smart viewing** — Maintain zoom & pan between photos (perfect for focus checks)
- 💾 **Smart caching** — Smooth navigation with intelligent preloading
- ⌨️ **Keyboard-first** — Efficient shortcuts for power users
- ↩️ **Undoable ops** — Safe file operations with undo support
- 🏷️ **Catalog system** — Organize images with collections and metadata
- ⭐ **Rating & labeling** — Rate images and apply color labels
- 🔧 **Batch processing** — Resize, convert, and process multiple images
- 🎨 **Advanced adjustments** — 45+ parameters with GPU acceleration

## 🚀 Quick Start
1. Install Rust:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
2. Clone and build:
   ```bash
   git clone https://github.com/rview-app/rview.git
   cd rview
   cargo build --release
   ```
3. Run:
   ```bash
   cargo run --release
   ```

## 📷 Supported Formats
- **Common:** JPEG, PNG, GIF, BMP, TIFF, WebP, ICO, PNM
- **RAW:** Canon, Nikon, Sony, Olympus, Panasonic, Adobe DNG, Fuji, Pentax, and more

## ⌨️ Keyboard Shortcuts
| Key | Action |
|-----|--------|
| `←` / `→` / `A` / `D` | Previous / Next image |
| `Home` / `End` | First / Last image |
| `PageUp` / `PageDown` | Page up/down in thumbnails |
| `+` / `-` | Zoom in/out |
| `0` | Fit to window |
| `1` | 100% zoom |
| `2` | 200% zoom |
| `H` | Toggle UI panels |
| `P` | Toggle panels |
| `T` | Toggle thumbnails |
| `S` | Toggle sidebar |
| `A` | Toggle adjustments |
| `E` | Toggle EXIF overlay |
| `C` | Toggle compare view |
| `F` | Toggle fullscreen |
| `F11` | Toggle fullscreen |
| `G` | Toggle lightbox mode |
| `Ctrl+G` | Toggle grid overlay |
| `Ctrl+L` | Toggle loupe |
| `\` | Show original image |
| `Ctrl+F` | Toggle focus peaking |
| `Alt+Z` | Toggle zebra stripes |
| `F2` | Batch rename images |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Del` | Delete (to trash) |
| `M` | Move to 'selected' folder |
| `Ctrl+P` | Command palette |
| `Ctrl+G` | Go to image dialog |
| `Ctrl+O` | Open file/folder |
| `Ctrl+Shift+O` | Open folder |
| `Ctrl+C` | Copy to clipboard |
| `Ctrl+A` | Select all images |
| `Alt+0-5` | Rate image (0-5 stars) |
| `Ctrl+1-5` | Color label (Red/Yellow/Green/Blue/Purple) |
| `Ctrl+0` | Remove color label |
| `Esc` | Close dialogs / Exit fullscreen / Stop slideshow |

## 🤝 Contributing
Bug reports and pull requests welcome! Check issues and open a PR. Run tests with `cargo test`.

## 📄 License
MIT — see [LICENSE](LICENSE).

---

<p align="center">
  <sub>Built with ❤️ and Rust • eframe/egui • wgpu/WebGPU • rawloader/imagepipe • tokio • rayon</sub>
</p>