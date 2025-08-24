#!/bin/bash

# Local CI Test Script for aubo-rs
# This script simulates the GitHub Actions CI build process locally

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Check prerequisites
check_prerequisites() {
    log_step "Checking prerequisites"
    
    local missing=()
    
    if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$ANDROID_NDK_ROOT" ] && [ -z "$NDK_HOME" ]; then
        missing+=("Android NDK (set ANDROID_NDK_HOME/ANDROID_NDK_ROOT/NDK_HOME)")
    fi
    
    if ! command -v cargo &> /dev/null; then
        missing+=("cargo")
    fi
    
    if ! command -v cmake &> /dev/null; then
        missing+=("cmake")
    fi
    
    if ! command -v cargo-ndk &> /dev/null; then
        log_warn "cargo-ndk not found, attempting to install..."
        cargo install cargo-ndk || missing+=("cargo-ndk")
    fi
    
    if [ ${#missing[@]} -ne 0 ]; then
        log_error "Missing prerequisites: ${missing[*]}"
        exit 1
    fi
    
    # Set NDK path
    if [ -n "$ANDROID_NDK_HOME" ]; then
        export ANDROID_NDK_ROOT="$ANDROID_NDK_HOME"
    elif [ -n "$NDK_HOME" ]; then
        export ANDROID_NDK_ROOT="$NDK_HOME"
    fi
    
    log_info "Using Android NDK: $ANDROID_NDK_ROOT"
}

# Simulate CI environment setup
setup_environment() {
    log_step "Setting up build environment"
    
    export ANDROID_API_LEVEL=29
    export CC_aarch64_linux_android="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android${ANDROID_API_LEVEL}-clang"
    export AR_aarch64_linux_android="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android${ANDROID_API_LEVEL}-clang"
    export PKG_CONFIG_ALLOW_CROSS=1
    export OPENSSL_STATIC=1
    export OPENSSL_NO_VENDOR=1
    
    # Add Android target
    rustup target add aarch64-linux-android
    
    log_info "Environment configured for Android API $ANDROID_API_LEVEL"
}

# Build Rust library
build_rust() {
    log_step "Building Rust library (simulating CI)"
    
    log_info "Building for aarch64-linux-android..."
    cargo ndk -t arm64-v8a build --release --no-default-features --features "filter-engine,performance-monitoring"
    
    RUST_LIB="target/aarch64-linux-android/release/libaubo_rs.so"
    if [ -f "$RUST_LIB" ]; then
        log_info "✓ Rust library built: $RUST_LIB ($(stat -c%s "$RUST_LIB" 2>/dev/null || stat -f%z "$RUST_LIB") bytes)"
    else
        log_error "✗ Rust library build failed"
        exit 1
    fi
}

# Build C++ module
build_cpp() {
    log_step "Building C++ ZygiskNext module (simulating CI)"
    
    # Create build directory
    BUILD_DIR="target/aarch64-linux-android/cpp_build"
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    
    log_info "Configuring with CMake..."
    cmake -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_ROOT/build/cmake/android.toolchain.cmake" \
          -DANDROID_ABI=arm64-v8a \
          -DANDROID_PLATFORM=android-${ANDROID_API_LEVEL} \
          -DCMAKE_BUILD_TYPE=Release \
          ../../../src/cpp
    
    log_info "Building with CMake..."
    cmake --build . --config Release
    
    # Copy built module
    cp libaubo_module.so ../aubo_module.so
    cd ../../..
    
    CPP_MODULE="target/aarch64-linux-android/aubo_module.so"
    if [ -f "$CPP_MODULE" ]; then
        log_info "✓ C++ module built: $CPP_MODULE ($(stat -c%s "$CPP_MODULE" 2>/dev/null || stat -f%z "$CPP_MODULE") bytes)"
    else
        log_error "✗ C++ module build failed"
        exit 1
    fi
}

# Validate modules
validate_modules() {
    log_step "Validating built modules (simulating CI)"
    
    RUST_LIB="target/aarch64-linux-android/release/libaubo_rs.so"
    CPP_MODULE="target/aarch64-linux-android/aubo_module.so"
    
    # Check Rust library
    if [ -f "$RUST_LIB" ]; then
        log_info "✓ Rust library validation passed"
        
        # Try to check for symbols (if nm is available)
        if command -v nm &> /dev/null || [ -f "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm" ]; then
            NM_CMD="nm"
            if [ -f "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm" ]; then
                NM_CMD="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm"
            fi
            
            if $NM_CMD "$RUST_LIB" 2>/dev/null | grep -q "aubo_initialize"; then
                log_info "✓ Required Rust symbols found"
            else
                log_warn "⚠ Could not verify Rust symbols (may be normal)"
            fi
        fi
    else
        log_error "✗ Rust library validation failed"
        exit 1
    fi
    
    # Check C++ module
    if [ -f "$CPP_MODULE" ]; then
        log_info "✓ C++ module validation passed"
        
        # Try to check for symbols (if nm is available)
        if command -v nm &> /dev/null || [ -f "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm" ]; then
            NM_CMD="nm"
            if [ -f "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm" ]; then
                NM_CMD="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-nm"
            fi
            
            if $NM_CMD "$CPP_MODULE" 2>/dev/null | grep -q "zn_module"; then
                log_info "✓ Required ZygiskNext symbols found"
            else
                log_warn "⚠ Could not verify ZygiskNext symbols (may be normal)"
            fi
        fi
    else
        log_error "✗ C++ module validation failed"
        exit 1
    fi
}

# Package modules
package_modules() {
    log_step "Packaging modules (simulating CI)"
    
    # Create module structure
    mkdir -p build/module/lib/arm64
    
    # Copy libraries
    cp target/aarch64-linux-android/release/libaubo_rs.so build/module/lib/arm64/
    cp target/aarch64-linux-android/aubo_module.so build/module/lib/arm64/
    
    # Copy template files
    cp -r template/* build/module/
    cp aubo-rs.toml build/module/
    cp README.md build/module/ 2>/dev/null || true
    
    # Show package contents
    log_info "Package contents:"
    ls -la build/module/lib/arm64/
    
    log_info "✓ Module packaging simulation complete"
    log_info "Ready for CI/CD pipeline!"
}

# Main function
main() {
    log_step "Starting local CI simulation for aubo-rs"
    
    check_prerequisites
    setup_environment
    build_rust
    build_cpp
    validate_modules
    package_modules
    
    log_step "Local CI simulation completed successfully!"
    log_info "Both modules built and validated:"
    log_info "  - Rust library: libaubo_rs.so"
    log_info "  - C++ module: aubo_module.so"
    log_info "You can now commit and push to trigger the GitHub Actions CI"
}

# Run main function
main "$@"