# 🖼️ Image Viewer

A modern, cross-platform image viewer built with Rust. Supports standard image formats and camera RAW files with professional features for photographers.

![Rust](https://img.shields.io/badge/Rust-1.70+-orange)
![Platform](https://img.shields.io/badge/Platform-Windows%20|%20macOS%20|%20Linux-blue)
![License](https://img.shields.io/badge/License-MIT-green)

## ✨ Features

### 🆕 Recent Updates (v2.0.0)
- **Panel Hiding**: Press 'H' to hide/show all UI panels for distraction-free viewing
- **Move to Selected Folder**: Press 'M' to organize images into a 'selected' subfolder with undo support
- **Enhanced Undo/Redo**: Improved operation history for file management
- **Performance Optimizations**: Release build with optimized caching and async loading

### Core Features
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Standard formats**: JPEG, PNG, GIF, BMP, TIFF, WebP, ICO, PNM
- **RAW support**: CR2, CR3, NEF, ARW, ORF, RW2, DNG, RAF, and many more
- **GPU-accelerated rendering**: Smooth performance even with large images

### Navigation & Viewing
- **Maintain zoom on navigate** ⭐: Compare focus across photos at 100% zoom (like FastStone!)
- **Maintain pan position**: Stay at the same spot when switching images
- **Mouse wheel zoom**: Zoom towards cursor position
- **Drag to pan**: Click and drag to move around the image
- **Double-click**: Toggle between fit and 100% zoom
- **Thumbnail strip**: Quick visual navigation

### Professional Features
- **EXIF data display**: Camera info, exposure settings, lens, date
- **RGB histogram**: Real-time histogram display
- **Rotation**: Rotate images without modifying files
- **Multiple fit modes**: Fit, Fill, 1:1, Fit Width, Fit Height
- **Slideshow**: Auto-advance with configurable interval
- **Undo/Redo**: Full operation history for file operations (delete, move, rotate, etc.)

### User Experience
- **Modern dark UI**: Clean, distraction-free interface
- **Drag & drop**: Drop images or folders to open
- **Keyboard shortcuts**: Full keyboard navigation
- **Smart caching**: Preloads adjacent images for instant switching
- **Custom backgrounds**: Dark, Light, Gray, or Checkered
- **Panel hiding**: Press 'H' to hide/show all UI panels for distraction-free viewing
- **Move to selected folder**: Press 'M' to move current image to 'selected' subfolder with undo support

## 🚀 Installation

### From Source

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and build**:
   ```bash
   git clone https://github.com/yourusername/image-viewer.git
   cd image-viewer
   cargo build --release
   ```

3. **Run**:
   ```bash
   cargo run --release
   ```

   Or find the binary in `target/release/image_viewer`

   **✅ Tested and verified**: The application successfully builds and runs with JPEG and RAW image support.

### Quick Test

Test files are included in the `testfiles/` directory:
- `_MG_6741.CR2` (Canon RAW format)
- `_MG_7957.jpg` (JPEG format)

Use these to test loading performance and features.

### Platform-Specific Notes

#### Windows
No additional dependencies required.

#### macOS
No additional dependencies required.

#### Linux
Install required system libraries:
```bash
# Ubuntu/Debian
sudo apt install libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Fedora
sudo dnf install gtk3-devel

# Arch
sudo pacman -S gtk3
```

## ⌨️ Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `←` `→` | Previous / Next image |
| `A` `D` | Previous / Next image (alternative) |
| `Home` `End` | First / Last image |
| `Page Up/Down` | Skip 10 images |
| `+` `-` | Zoom in / out |
| `0` | Fit to window |
| `1` | 100% zoom (actual size) |
| `L` `R` | Rotate left / right |
| `Space` | Toggle slideshow |
| `F11` `F` | Toggle fullscreen |
| `Esc` | Exit fullscreen / Stop slideshow |
| `Del` | Delete current image (to trash) |
| `I` | Toggle EXIF info |
| `T` | Toggle thumbnail bar |
| `S` | Toggle sidebar |
| `H` | Toggle panel visibility |
| `M` | Move image to 'selected' folder |

## 🖱️ Mouse Controls

| Action | Result |
|--------|--------|
| Scroll wheel | Zoom in/out at cursor |
| Click + drag | Pan image |
| Double-click | Toggle fit / 100% zoom |
| Click thumbnail | Jump to image |

## 🎯 The "FastStone" Feature

One of the key features is the ability to **maintain zoom level and pan position when navigating between images**. This is essential for photographers who want to:

1. Zoom to 100% on an area of interest (e.g., eyes for focus check)
2. Navigate to the next image with `→`
3. See the exact same area at the same zoom level

Enable this in Settings:
- ✅ **Keep zoom on navigate**
- ✅ **Keep pan position on navigate**

## 📁 Supported Formats

### Standard Formats
- JPEG (.jpg, .jpeg)
- PNG (.png)
- GIF (.gif)
- BMP (.bmp)
- TIFF (.tiff, .tif)
- WebP (.webp)
- ICO (.ico)
- PNM (.pnm, .pbm, .pgm, .ppm)

### RAW Formats
- Canon (.cr2, .cr3)
- Nikon (.nef)
- Sony (.arw)
- Olympus (.orf)
- Panasonic (.rw2)
- Adobe DNG (.dng)
- Fuji (.raf)
- Samsung (.srw)
- Pentax (.pef)
- And many more...

## 🔧 Configuration

Settings are automatically saved and include:
- Theme preference
- Background color
- Panel visibility
- Thumbnail size
- Zoom/pan maintain settings
- Slideshow interval
- Recent folders
- Sort preferences

Config location:
- **Windows**: `%APPDATA%\imageviewer\ImageViewer\settings.json`
- **macOS**: `~/Library/Application Support/com.imageviewer.ImageViewer/settings.json`
- **Linux**: `~/.config/imageviewer/settings.json`

## 🏗️ Architecture

Built with modern Rust technologies:
- **eframe/egui**: Immediate-mode GUI framework
- **image**: Standard image format decoding
- **rawloader/imagepipe**: RAW file processing
- **tokio**: Async runtime for background loading
- **serde**: Settings serialization

## 📜 License

MIT License - see [LICENSE](LICENSE) for details.

## 🤝 Contributing

Contributions welcome! Please feel free to submit issues and pull requests.

## 🙏 Acknowledgments

- [egui](https://github.com/emilk/egui) - Amazing immediate-mode GUI
- [rawloader](https://github.com/pedrocr/rawloader) - RAW file support
- [image-rs](https://github.com/image-rs/image) - Image processing
