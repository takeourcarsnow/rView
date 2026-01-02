# rView Project Overview 🦀

A comprehensive guide to the technologies, features, and learnings from building this Rust image viewer.

## 🦀 **Key Learnings & Technologies**

### **1. Rust Ecosystem & Language Fundamentals**
- **Error handling**: `anyhow`, `thiserror` for robust error management
- **Async programming**: `tokio` runtime for multi-threaded async operations
- **Trait system**: Module organization and trait implementations
- **Lifetimes & ownership**: Managing image data, caching, and state
- **Cargo profiles**: Optimized release builds (LTO, single codegen unit)

### **2. GUI Development**
- **egui/eframe**: Immediate mode GUI framework
- Custom icon fonts integration (`iconflow` with Lucide icons)
- Complex state management for image viewer
- Viewport configuration, drag-and-drop, persistence

### **3. GPU Acceleration (WebGPU)**
- **WGSL shaders** for compute operations:
  - Image adjustments (45+ parameters: exposure, contrast, saturation, etc.)
  - Histogram computation
  - RAW demosaicing
  - Focus peaking & zebra overlays
- **wgpu**: Cross-platform GPU API
- Async GPU operations with compute pipelines

### **4. Async/Parallel Processing**
- `tokio::spawn` for background tasks
- `rayon` for parallel image processing
- Smart caching with preloading
- Non-blocking UI during heavy operations

### **5. Image Processing**
- **RAW format support**: `rawloader`, `imagepipe` (CR2, NEF, ARW, DNG, RAF)
- Multiple format support via `image` crate
- EXIF data extraction (`kamadak-exif`)
- Film emulation with custom tone curves
- Image adjustments pipeline

### **6. Cross-Platform Development**
- **CI/CD**: GitHub Actions workflow for automated releases
- Multi-platform builds: Windows, macOS, Linux
- Platform-specific features (wallpaper setting on all 3 OSes)
- Artifact management and automatic release creation

### **7. Database & Persistence**
- **SQLite** (`rusqlite`) for catalog/collections
- Serde for settings serialization (JSON)
- File watching (`notify`) for live updates
- Metadata database management

### **8. Advanced Features**
- Fuzzy search (`fuzzy-matcher`)
- Natural filename sorting (`natord`)
- Trash/undo operations (`trash`)
- Clipboard integration (`arboard`)
- System notifications (`notify-rust`)
- File dialog integration (`rfd`)

### **9. Performance Optimization**
- Custom build profiles (dev-fast, quick-check)
- LTO and optimization flags
- Smart caching strategies
- Memory-mapped I/O (`memmap2`)
- CPU core detection for parallel processing

### **10. DevOps & Release Management**
- **Git tagging** for automatic releases (`v*` tags)
- Cross-compilation setup
- Distribution of binaries via GitHub Releases
- Automated changelog generation

## 🎯 **Impressive Technical Achievements**

### GPU Computing
- Custom compute shaders written in WGSL
- Real-time image processing pipeline
- Async GPU operations with proper synchronization

### Async Architecture
- Tokio-based async runtime for non-blocking operations
- Background image loading and caching
- Parallel processing with rayon for CPU-bound tasks

### Professional Photography Workflow
- RAW image format support for multiple camera brands
- EXIF metadata parsing and display
- Focus peaking and zebra pattern overlays
- Film emulation presets

### Cross-Platform Distribution
- Automated builds for Windows, macOS, and Linux
- GitHub Actions CI/CD pipeline
- Single-command release process via git tags

### Code Organization
- Clean module structure with separation of concerns
- GPU, UI, image processing, and app logic separated
- Comprehensive error handling throughout

## 📦 **Major Dependencies**

| Category | Crates |
|----------|--------|
| GUI | `eframe`, `egui`, `egui_extras` |
| Image | `image`, `rawloader`, `imagepipe`, `imageproc` |
| GPU | `wgpu`, `pollster`, `bytemuck` |
| Async | `tokio`, `rayon` |
| Database | `rusqlite` |
| Utilities | `anyhow`, `thiserror`, `serde`, `chrono` |

## 🚀 **Release Process**

1. Update version in `Cargo.toml`
2. Create and push a git tag: `git tag v0.1.0 && git push origin v0.1.0`
3. GitHub Actions automatically:
   - Builds for Windows, macOS, Linux
   - Creates GitHub release
   - Uploads binaries as artifacts
   - Generates release notes

## 💡 **What Makes This Special**

This is an excellent first Rust project that goes beyond typical beginner projects by incorporating:

- **Modern GPU programming** with compute shaders
- **Professional-grade async patterns** with tokio
- **Cross-platform GUI** with immediate mode rendering
- **Real-world usefulness** for photographers and visual artists
- **Production-ready CI/CD** with automated releases
- **Performance optimization** at multiple levels

---

*Built as a first Rust project to learn the language, ecosystem, and modern development practices.*
