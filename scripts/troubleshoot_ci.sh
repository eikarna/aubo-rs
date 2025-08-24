#!/bin/bash

# CI/CD Troubleshooting Script for aubo-rs
# This script helps diagnose and fix common build issues

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# Clean all build artifacts
clean_all() {
    log_step "Cleaning all build artifacts"
    
    # Clean Rust artifacts
    if command -v cargo &> /dev/null; then
        log_info "Cleaning Cargo artifacts..."
        cargo clean
        cargo clean --target aarch64-linux-android 2>/dev/null || true
    fi
    
    # Clean C++ build directories
    log_info "Cleaning C++ build directories..."
    rm -rf target/*/cpp_build
    rm -rf target/cpp_build
    rm -rf build/
    rm -rf lib/
    
    # Clean CMake cache files
    find . -name "CMakeCache.txt" -delete 2>/dev/null || true
    find . -name "CMakeFiles" -type d -exec rm -rf {} + 2>/dev/null || true
    
    log_info "✓ All artifacts cleaned"
}

# Check prerequisites
check_prerequisites() {
    log_step "Checking build prerequisites"
    
    local issues=()
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        issues+=("Rust/Cargo not found")
    else
        log_info "✓ Rust: $(rustc --version)"
    fi
    
    # Check Android NDK
    if [ -z "$ANDROID_NDK_HOME" ] && [ -z "$ANDROID_NDK_ROOT" ] && [ -z "$NDK_HOME" ]; then
        issues+=("Android NDK environment variables not set")
    else
        NDK_PATH="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-$NDK_HOME}}"
        if [ -d "$NDK_PATH" ]; then
            log_info "✓ Android NDK: $NDK_PATH"
        else
            issues+=("Android NDK directory not found: $NDK_PATH")
        fi
    fi
    
    # Check CMake
    if ! command -v cmake &> /dev/null; then
        issues+=("CMake not found")
    else
        log_info "✓ CMake: $(cmake --version | head -1)"
    fi
    
    # Check cargo-ndk
    if ! command -v cargo-ndk &> /dev/null; then
        log_warn "cargo-ndk not found - attempting to install"
        cargo install cargo-ndk || issues+=("cargo-ndk installation failed")
    else
        log_info "✓ cargo-ndk available"
    fi
    
    # Check Android target
    if ! rustup target list --installed | grep -q aarch64-linux-android; then
        log_warn "Android target not installed - attempting to add"
        rustup target add aarch64-linux-android || issues+=("Failed to add Android target")
    else
        log_info "✓ Android target: aarch64-linux-android"
    fi
    
    if [ ${#issues[@]} -ne 0 ]; then
        log_error "Prerequisites check failed:"
        for issue in "${issues[@]}"; do
            log_error "  - $issue"
        done
        return 1
    fi
    
    log_info "✓ All prerequisites satisfied"
}

# Diagnose CMake issues
diagnose_cmake() {
    log_step "Diagnosing CMake configuration"
    
    # Check for toolchain file
    NDK_PATH="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-$NDK_HOME}}"
    TOOLCHAIN_FILE="$NDK_PATH/build/cmake/android.toolchain.cmake"
    
    if [ -f "$TOOLCHAIN_FILE" ]; then
        log_info "✓ CMake toolchain found: $TOOLCHAIN_FILE"
    else
        log_error "✗ CMake toolchain not found: $TOOLCHAIN_FILE"
        return 1
    fi
    
    # Check CMake version compatibility
    CMAKE_VERSION=$(cmake --version | head -1 | awk '{print $3}')
    CMAKE_MAJOR=$(echo $CMAKE_VERSION | cut -d. -f1)
    CMAKE_MINOR=$(echo $CMAKE_VERSION | cut -d. -f2)\n    \n    if [ "$CMAKE_MAJOR" -lt 3 ] || ([ "$CMAKE_MAJOR" -eq 3 ] && [ "$CMAKE_MINOR" -lt 18 ]); then\n        log_warn "CMake version $CMAKE_VERSION may be too old (recommend 3.18+)"\n    else\n        log_info "✓ CMake version compatible: $CMAKE_VERSION"\n    fi
}

# Fix common CMake issues
fix_cmake_issues() {
    log_step "Fixing common CMake issues"
    
    # Remove any existing CMake cache
    log_info "Removing CMake cache files..."
    find . -name "CMakeCache.txt" -delete 2>/dev/null || true
    find . -name "CMakeFiles" -type d -exec rm -rf {} + 2>/dev/null || true
    
    # Clean C++ build directories completely
    log_info "Cleaning C++ build directories..."
    rm -rf target/*/cpp_build
    
    log_info "✓ CMake issues fixed"
}

# Test minimal C++ build
test_cpp_build() {
    log_step "Testing minimal C++ build"
    
    if [ ! -d "src/cpp" ]; then
        log_error "C++ source directory not found: src/cpp"
        return 1
    fi
    
    NDK_PATH="${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-$NDK_HOME}}"
    BUILD_DIR="target/test_cpp_build"
    
    # Clean test directory
    rm -rf "$BUILD_DIR"
    mkdir -p "$BUILD_DIR"
    cd "$BUILD_DIR"
    
    log_info "Testing CMake configuration..."
    if cmake -DCMAKE_TOOLCHAIN_FILE="$NDK_PATH/build/cmake/android.toolchain.cmake" \
            -DANDROID_ABI=arm64-v8a \
            -DANDROID_PLATFORM=android-29 \
            -DCMAKE_BUILD_TYPE=Release \
            ../../src/cpp; then
        log_info "✓ CMake configuration successful"
        
        log_info "Testing build..."
        if make -j$(nproc 2>/dev/null || echo 2); then
            log_info "✓ C++ build successful"
        else
            log_error "✗ C++ build failed"
            cd - > /dev/null
            return 1
        fi
    else
        log_error "✗ CMake configuration failed"
        cd - > /dev/null
        return 1
    fi
    
    cd - > /dev/null
    rm -rf "$BUILD_DIR"
}

# Show CI environment simulation
show_ci_env() {
    log_step "CI Environment Information"
    
    echo "Environment variables that would be set in CI:"
    echo "  ANDROID_API_LEVEL=29"
    echo "  ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-${ANDROID_NDK_ROOT:-$NDK_HOME}}"
    echo "  CC_aarch64_linux_android=\$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
    echo "  AR_aarch64_linux_android=\$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"
    echo "  CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=\$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android29-clang"
    echo "  PKG_CONFIG_ALLOW_CROSS=1"
    echo "  OPENSSL_STATIC=1"
    echo "  OPENSSL_NO_VENDOR=1"
}

# Main troubleshooting function
main() {
    log_step "aubo-rs CI/CD Troubleshooting Tool"
    
    case "${1:-all}" in
        "clean")
            clean_all
            ;;
        "check")
            check_prerequisites
            ;;
        "cmake")
            diagnose_cmake
            ;;
        "fix")
            fix_cmake_issues
            ;;
        "test")
            test_cpp_build
            ;;
        "env")
            show_ci_env
            ;;
        "all")
            clean_all
            check_prerequisites
            diagnose_cmake
            fix_cmake_issues
            test_cpp_build
            show_ci_env
            log_step "Troubleshooting completed successfully!"
            ;;
        *)
            echo "Usage: $0 [clean|check|cmake|fix|test|env|all]"
            echo "  clean - Clean all build artifacts"
            echo "  check - Check prerequisites"
            echo "  cmake - Diagnose CMake issues"
            echo "  fix   - Fix common CMake issues"
            echo "  test  - Test C++ build"
            echo "  env   - Show CI environment info"
            echo "  all   - Run all steps (default)"
            ;;
    esac
}

main "$@"