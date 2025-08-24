#!/bin/bash

# Build script for aubo-rs ZygiskNext module
# This script builds both the Rust library and C++ ZygiskNext module

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

# Check if Android NDK is available
check_ndk() {
    if [ -z "$ANDROID_NDK_ROOT" ] && [ -z "$NDK_HOME" ] && [ -z "$ANDROID_NDK_HOME" ]; then
        log_error "Android NDK not found. Please set ANDROID_NDK_ROOT, NDK_HOME, or ANDROID_NDK_HOME"
        exit 1
    fi
    
    NDK_PATH="${ANDROID_NDK_ROOT:-${NDK_HOME:-$ANDROID_NDK_HOME}}"
    if [ ! -d "$NDK_PATH" ]; then
        log_error "Android NDK directory not found: $NDK_PATH"
        exit 1
    fi
    
    log_info "Using Android NDK: $NDK_PATH"
}

# Check if required tools are available
check_tools() {
    local missing_tools=()
    
    if ! command -v cargo &> /dev/null; then
        missing_tools+=("cargo")
    fi
    
    if ! command -v cmake &> /dev/null; then
        missing_tools+=("cmake")
    fi
    
    if ! command -v cargo-ndk &> /dev/null; then
        log_warn "cargo-ndk not found, trying to install..."
        cargo install cargo-ndk || missing_tools+=("cargo-ndk")
    fi
    
    if [ ${#missing_tools[@]} -ne 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        log_error "Please install the missing tools and try again"
        exit 1
    fi
}

# Add Android targets
setup_targets() {
    log_step "Setting up Android targets"
    
    rustup target add aarch64-linux-android
    rustup target add armv7-linux-androideabi
    
    log_info "Android targets added successfully"
}

# Build Rust library
build_rust() {
    log_step "Building Rust library"
    
    # Build for arm64 (primary target)
    log_info "Building for aarch64-linux-android..."
    cargo ndk -t arm64-v8a build --release --features "full"
    
    # Build for armv7 (optional)
    log_info "Building for armv7-linux-androideabi..."
    cargo ndk -t armeabi-v7a build --release --features "full" || log_warn "armv7 build failed, continuing with arm64 only"
    
    # Verify Rust library was built
    RUST_LIB="target/aarch64-linux-android/release/libaubo_rs.so"
    if [ ! -f "$RUST_LIB" ]; then
        log_error "Rust library build failed: $RUST_LIB not found"
        exit 1
    fi
    
    log_info "Rust library built successfully: $RUST_LIB"
}

# Build C++ ZygiskNext module
build_cpp() {
    log_step "Building C++ ZygiskNext module"
    
    # Create clean build directory
    BUILD_DIR="target/aarch64-linux-android/cpp_build"
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"
    
    # Set up NDK toolchain
    NDK_PATH="${ANDROID_NDK_ROOT:-${NDK_HOME:-$ANDROID_NDK_HOME}}"
    TOOLCHAIN_FILE="$NDK_PATH/build/cmake/android.toolchain.cmake"
    
    if [ ! -f "$TOOLCHAIN_FILE" ]; then
        log_error "Android CMake toolchain not found: $TOOLCHAIN_FILE"
        exit 1
    fi
    
    # Configure with CMake
    log_info "Configuring C++ module with CMake..."
    cd "$BUILD_DIR"
    cmake -DCMAKE_TOOLCHAIN_FILE="$TOOLCHAIN_FILE" \
          -DANDROID_ABI=arm64-v8a \
          -DANDROID_PLATFORM=android-29 \
          -DCMAKE_BUILD_TYPE=Release \
          ../../../src/cpp
    
    # Build with make
    log_info "Building C++ module..."
    make -j$(nproc 2>/dev/null || echo 4)
    
    cd - > /dev/null
    
    # Verify C++ module was built
    CPP_MODULE="$BUILD_DIR/libaubo_module.so"
    if [ ! -f "$CPP_MODULE" ]; then
        log_error "C++ module build failed: $CPP_MODULE not found"
        exit 1
    fi
    
    log_info "C++ module built successfully: $CPP_MODULE"
}

# Package libraries
package_libs() {
    log_step "Packaging libraries"
    
    # Create lib directory structure
    mkdir -p lib/arm64
    
    # Copy Rust library
    RUST_LIB="target/aarch64-linux-android/release/libaubo_rs.so"
    if [ -f "$RUST_LIB" ]; then
        cp "$RUST_LIB" lib/arm64/
        log_info "Copied Rust library to lib/arm64/"
    else
        log_error "Rust library not found: $RUST_LIB"
        exit 1
    fi
    
    # Copy C++ module
    CPP_MODULE="target/aarch64-linux-android/cpp_build/libaubo_module.so"
    if [ -f "$CPP_MODULE" ]; then
        cp "$CPP_MODULE" lib/arm64/aubo_module.so
        log_info "Copied C++ module to lib/arm64/aubo_module.so"
    else
        log_error "C++ module not found: $CPP_MODULE"
        exit 1
    fi
    
    # Show library sizes
    echo
    log_info "Built libraries:"
    ls -lh lib/arm64/
    echo
}

# Main build process
main() {
    log_step "Starting aubo-rs build process"
    
    # Checks
    check_ndk
    check_tools
    
    # Setup
    setup_targets
    
    # Build components
    build_rust
    build_cpp
    
    # Package
    package_libs
    
    log_step "Build completed successfully!"
    log_info "Libraries are ready in lib/arm64/ directory"
    log_info "You can now create a Magisk module ZIP with these libraries"
}

# Run main function
main "$@"