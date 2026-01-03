# rView 🦀

**rView** — A modern, blazing-fast image viewer built with **Rust**. RAW support, GPU-accelerated previews, and a clean interface.

![Rust](https://img.shields.io/badge/Rust-1.70+-B7410E?logo=rust) ![Platform](https://img.shields.io/badge/Platform-Windows%20|%20macOS%20|%20Linux-blue) ![License](https://img.shields.io/badge/License-MIT-green) ![Version](https://img.shields.io/badge/Version-0.2.0-B7410E)

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
| `←` / `→` | Previous / Next image |
| `1` | 100% zoom |
| `0` | Fit to window |
| `H` | Toggle UI panels |
| `F2` | Batch rename images |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` / `Ctrl+Shift+Z` | Redo |
| `Alt+Z` | Toggle zebra stripes |
| `M` | Move to 'selected' folder (undoable) |
| `Del` | Delete (to trash) |

## 🤝 Contributing
Bug reports and pull requests welcome! Check issues and open a PR. Run tests with `cargo test`.

## 📄 License
MIT — see [LICENSE](LICENSE).

---

<p align="center">
  <sub>Built with ❤️ and Rust • eframe/egui • image-rs • rawloader • imagepipe • tokio</sub>
</p>