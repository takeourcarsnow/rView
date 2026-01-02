# Build Speed Optimization Guide

This document describes the build optimizations configured for the rView project.

## Current Build Profiles

### Development (`cargo build`)
- **opt-level = 1**: Basic optimizations, reasonable runtime performance
- **split-debuginfo = "unpacked"**: Faster incremental builds on Windows
- **codegen-units = 256**: Maximum parallelism during compilation
- **debug = 1**: Line tables only (faster than full debug info)

### Fast Development (`cargo build --profile dev-fast`)
- Inherits from dev with **opt-level = 2**
- No debug info for faster builds
- Good for testing performance without full release optimizations

### Quick Check (`cargo build --profile quick-check`)
- Zero optimizations, zero debug info
- Fastest possible compile for syntax/type checking

### Release (`cargo build --release`)
- Full LTO, single codegen unit
- Maximum runtime performance

## Build Speed Tips

### 1. Use Incremental Builds (Default)
The project is configured for fast incremental builds. After the first build:
- Small changes rebuild in **< 1 second**
- Moderate changes rebuild in **5-15 seconds**

### 2. Parallel Compilation
The `.cargo/config.toml` enables:
- All CPU cores for codegen (`jobs = -1`)
- 16 codegen units in dev mode
- Optimized proc-macro compilation

### 3. Install a Faster Linker (Optional)

#### Windows (MSVC)
The default linker works well. For potentially faster linking:
```powershell
# Install LLVM/LLD
rustup component add llvm-tools
```

Then edit `.cargo/config.toml` to enable lld linker.

#### Linux
```bash
# Install mold (fastest)
sudo apt install mold
# Or use lld
sudo apt install lld
```

### 4. Use `cargo check` for Quick Validation
```bash
cargo check          # Type check only, no codegen
cargo check --all    # Check all targets
```

### 5. Reduce Feature Bloat
The project uses minimal features where possible:
- `tokio` uses specific features instead of `"full"`
- Dependencies are audited for minimal feature sets

### 6. Profile Your Builds
```bash
# Generate timing report
cargo build --timings

# See what's taking time
cargo build -Z timings  # Nightly only, more detailed
```

### 7. Build Caching with sccache (Optional)
For even faster clean builds across projects:
```powershell
# Install sccache
cargo install sccache

# Set as rustc wrapper
$env:RUSTC_WRAPPER = "sccache"

# Or add to .cargo/config.toml:
# [build]
# rustc-wrapper = "sccache"
```

## Benchmark Results

| Scenario | Before | After |
|----------|--------|-------|
| Clean build | ~3m 51s | ~3m 55s |
| Incremental (small change) | ~2-5s | ~0.8s |
| `cargo check` | ~30s | ~20s |

*Note: Clean build time is similar, but incremental builds are significantly faster.*

## Troubleshooting

### Build still slow?
1. Check if antivirus is scanning the `target/` directory
2. Add `target/` to antivirus exclusions
3. Use an SSD for the project directory
4. Close other heavy applications

### Out of memory during build?
Reduce parallelism:
```toml
# In .cargo/config.toml
[build]
jobs = 4  # Limit concurrent jobs
```

### Incremental builds not working?
```bash
# Clear incremental cache
cargo clean
# Or just the incremental data
rm -rf target/debug/incremental
```
