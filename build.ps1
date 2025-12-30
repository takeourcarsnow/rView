# Build and run script for Image Viewer on Windows
# This script temporarily adds mingw64 to PATH and runs the application

$mingwPath = "C:\msys64\mingw64\bin"
if (-not (Test-Path $mingwPath)) {
    Write-Host "MSYS2 mingw64 not found at $mingwPath. Please install MSYS2 first."
    Write-Host "Run: winget install --id MSYS2.MSYS2"
    exit 1
}

# Add mingw to PATH
$env:PATH = "$mingwPath;$env:PATH"

# Run the application
cargo run