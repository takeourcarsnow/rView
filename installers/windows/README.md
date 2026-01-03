# rView Windows Installer Configuration
# For use with WiX v5 or Inno Setup

# WiX v5 Configuration (rview.wxs)
# Run: wix build -o rview.msi rview.wxs

# This is a simplified template. For full WiX support, you'll need:
# 1. WiX Toolset v5: https://wixtoolset.org/
# 2. Proper GUID generation for ProductCode and UpgradeCode

# Note: The actual WiX XML file would be more complex.
# Below is a reference for what the installer should include:

# Installer Features:
# - Install rview.exe to Program Files
# - Add to PATH (optional)
# - Create Start Menu shortcut
# - Create Desktop shortcut (optional)
# - Register file associations for image types
# - Add uninstaller entry

# File Associations to Register:
# .jpg, .jpeg, .png, .gif, .bmp, .tiff, .tif, .webp, .ico
# .cr2, .cr3, .nef, .arw, .dng, .raf, .rw2, .orf, .pef

# For a simple portable installation, just extract the zip file.
# The application is fully portable and doesn't require installation.
