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

### **10. Advanced Features**
- Fuzzy search (`fuzzy-matcher`)
- Natural filename sorting (`natord`)
- Trash/undo operations (`trash`)
- Clipboard integration (`arboard`)
- System notifications (`notify-rust`)
- File dialog integration (`rfd`)
- Batch processing (resize, convert, rename)
- Catalog/collections system with SQLite
- Image ratings and color labels
- Command palette and go-to dialogs
- Loupe tool and grid overlays
- Lightbox mode and slideshow
- Memory-mapped I/O (`memmap2`)
- Icon fonts (`iconflow` with Lucide icons)

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
- Film emulation presets with custom tone curves
- Batch processing pipeline
- Catalog system with collections and database storage
- Image ratings and color labeling system
- Advanced adjustment tools (45+ parameters)

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
| GPU/WebGPU | `wgpu`, `pollster`, `bytemuck`, `futures-intrusive` |
| Image Processing | `image`, `rawloader`, `imagepipe`, `imageproc` |
| Async Runtime | `tokio`, `rayon` |
| Database | `rusqlite` |
| File System | `walkdir`, `notify`, `trash`, `memmap2` |
| Utilities | `anyhow`, `thiserror`, `serde`, `chrono`, `uuid` |
| Search & Sort | `fuzzy-matcher`, `natord` |
| System Integration | `arboard`, `notify-rust`, `rfd`, `open`, `directories` |
| Performance | `lazy_static`, `num_cpus`, `criterion` |
| Additional | `regex`, `palette`, `printpdf`, `jpeg-encoder`, `iconflow` |

## 🚀 **Release Process**

1. Update version in `Cargo.toml` (current: 0.5.0)
2. Create and push a git tag: `git tag v0.5.0 && git push origin v0.5.0`
3. GitHub Actions automatically:
   - Builds for Windows, macOS, Linux
   - Creates GitHub release
   - Uploads binaries as artifacts
   - Generates release notes

## 💡 **What Makes This Special**

This is an excellent Rust project that demonstrates advanced systems programming techniques:

- **Modern GPU programming** with compute shaders and WebGPU
- **Professional-grade async patterns** with tokio runtime
- **Cross-platform GUI** with immediate mode rendering
- **Real-world production features** for photographers and visual artists
- **Database integration** with SQLite for catalog management
- **Batch processing pipeline** for image operations
- **Comprehensive keyboard shortcuts** and power-user features
- **Automated CI/CD** with multi-platform releases
- **Performance optimization** at multiple levels (GPU, CPU, memory)
- **Clean architecture** separating GPU, UI, image processing, and app logic

---

*Built as a first Rust project to learn the language, ecosystem, and modern development practices.*
