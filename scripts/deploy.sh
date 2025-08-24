#!/bin/bash

# aubo-rs Deployment Script
# Handles automated deployment and testing on Android devices

set -euo pipefail

# Configuration
DEVICE_TEMP_DIR="/data/local/tmp"
MODULE_DATA_DIR="/data/adb/aubo-rs"
MAGISK_MODULES_DIR="/data/adb/modules"
LOG_FILE="$MODULE_DATA_DIR/deploy.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check if device is connected
check_device() {
    log_info "Checking device connection..."
    
    if ! command -v adb &> /dev/null; then
        log_error "ADB not found. Please install Android SDK Platform Tools."
        exit 1
    fi
    
    local devices=$(adb devices | grep -v "List of devices" | grep "device$" | wc -l)
    if [ "$devices" -eq 0 ]; then
        log_error "No Android device connected or device not authorized."
        log_info "Please connect your device and enable USB debugging."
        exit 1
    elif [ "$devices" -gt 1 ]; then
        log_warning "Multiple devices connected. Using first device."
    fi
    
    log_success "Device connected"
}

# Check device requirements
check_device_requirements() {
    log_info "Checking device requirements..."
    
    # Check root access
    if ! adb shell "su -c 'echo test'" 2>/dev/null | grep -q "test"; then
        log_error "Root access not available. Please root your device first."
        exit 1
    fi
    
    # Check Android version
    local sdk_version=$(adb shell getprop ro.build.version.sdk)
    if [ "$sdk_version" -lt 29 ]; then
        log_error "Android API level $sdk_version detected. Minimum required: 29 (Android 10)"
        exit 1
    fi
    
    # Check architecture
    local arch=$(adb shell getprop ro.product.cpu.abi)
    if [[ "$arch" != "arm64-v8a" ]]; then
        log_error "Unsupported architecture: $arch. Only arm64-v8a is supported."
        exit 1
    fi
    
    # Check Magisk
    if ! adb shell "command -v magisk" &>/dev/null; then
        log_error "Magisk not found. Please install Magisk first."
        exit 1
    fi
    
    # Check ZygiskNext
    if ! adb shell "ls /data/adb/modules | grep -q zygisknext"; then
        log_warning "ZygiskNext module not detected. aubo-rs requires ZygiskNext."
        log_info "Please install ZygiskNext module first."
    fi
    
    log_success "Device requirements met"
    log_info "Device info: Android API $sdk_version, $arch"
}

# Find module ZIP file
find_module() {
    log_info "Looking for aubo-rs module..."
    
    local module_zip=""
    
    # Check release directory first
    if [ -d "release" ]; then
        module_zip=$(find release -name "aubo-rs-*.zip" | head -1)
    fi
    
    # Check current directory
    if [ -z "$module_zip" ]; then
        module_zip=$(find . -maxdepth 1 -name "aubo-rs-*.zip" | head -1)
    fi
    
    if [ -z "$module_zip" ]; then
        log_error "No aubo-rs module ZIP found."
        log_info "Please build the module first using: ./scripts/build.sh"
        exit 1
    fi
    
    echo "$module_zip"
}

# Deploy module
deploy_module() {
    local module_zip="$1"
    local zip_name=$(basename "$module_zip")
    
    log_info "Deploying module: $zip_name"
    
    # Push module to device
    log_info "Uploading module to device..."
    adb push "$module_zip" "$DEVICE_TEMP_DIR/"
    
    # Backup existing module if present
    if adb shell "test -d $MAGISK_MODULES_DIR/aubo_rs"; then
        log_info "Backing up existing module..."
        adb shell "su -c 'cp -r $MAGISK_MODULES_DIR/aubo_rs $DEVICE_TEMP_DIR/aubo_rs_backup'"
    fi
    
    # Install module via Magisk
    log_info "Installing module via Magisk..."
    local install_output=$(adb shell "su -c 'magisk --install-module $DEVICE_TEMP_DIR/$zip_name'" 2>&1)
    
    if echo "$install_output" | grep -q "Success"; then
        log_success "Module installed successfully"
    else
        log_error "Module installation failed:"
        echo "$install_output"
        exit 1
    fi
    
    # Cleanup
    adb shell "rm -f $DEVICE_TEMP_DIR/$zip_name"
}

# Configure module
configure_module() {
    log_info "Configuring module..."
    
    # Ensure data directory exists
    adb shell "su -c 'mkdir -p $MODULE_DATA_DIR'"
    adb shell "su -c 'mkdir -p $MODULE_DATA_DIR/filters'"
    
    # Check if configuration exists
    if ! adb shell "test -f $MODULE_DATA_DIR/aubo-rs.toml"; then
        log_info "Uploading default configuration..."
        adb push "aubo-rs.toml" "$MODULE_DATA_DIR/"
    else
        log_info "Configuration file already exists"
    fi
    
    # Set proper permissions
    adb shell "su -c 'chown -R root:root $MODULE_DATA_DIR'"
    adb shell "su -c 'chmod -R 755 $MODULE_DATA_DIR'"
    
    # Enable debug logging for initial deployment
    adb shell "su -c 'touch $MODULE_DATA_DIR/debug.log'"
    adb shell "su -c 'chmod 644 $MODULE_DATA_DIR/debug.log'"
    
    log_success "Module configured"
}

# Run basic tests
test_deployment() {
    log_info "Running deployment tests..."
    
    # Check if module directory exists
    if ! adb shell "test -d $MAGISK_MODULES_DIR/aubo_rs"; then
        log_error "Module directory not found after installation"
        return 1
    fi
    
    # Check module files
    local required_files=(
        "module.prop"
        "service.sh"
        "post-fs-data.sh"
        "lib/arm64/libaubo_rs.so"
    )
    
    for file in "${required_files[@]}"; do
        if ! adb shell "test -f $MAGISK_MODULES_DIR/aubo_rs/$file"; then
            log_error "Required file missing: $file"
            return 1
        fi
    done
    
    # Check library architecture
    local lib_info=$(adb shell "file $MAGISK_MODULES_DIR/aubo_rs/lib/arm64/libaubo_rs.so")
    if [[ "$lib_info" != *"aarch64"* ]]; then
        log_error "Library has wrong architecture: $lib_info"
        return 1
    fi
    
    # Check configuration
    if ! adb shell "test -f $MODULE_DATA_DIR/aubo-rs.toml"; then
        log_error "Configuration file not found"
        return 1
    fi
    
    log_success "Deployment tests passed"
    return 0
}

# Show deployment status
show_status() {
    log_info "Deployment Status:"
    
    # Module status
    if adb shell "test -d $MAGISK_MODULES_DIR/aubo_rs"; then
        local module_prop=$(adb shell "cat $MAGISK_MODULES_DIR/aubo_rs/module.prop" 2>/dev/null)
        if [ ! -z "$module_prop" ]; then
            echo "$module_prop" | grep -E "(name|version|description)"
        fi
        echo "Module Status: Installed"
    else
        echo "Module Status: Not Installed"
    fi
    
    # Configuration status
    if adb shell "test -f $MODULE_DATA_DIR/aubo-rs.toml"; then
        echo "Configuration: Present"
    else
        echo "Configuration: Missing"
    fi
    
    # ZygiskNext status
    if adb shell "ls /data/adb/modules | grep -q zygisknext"; then
        echo "ZygiskNext: Installed"
    else
        echo "ZygiskNext: Not Found"
    fi
    
    # Device info
    local android_version=$(adb shell getprop ro.build.version.release)
    local api_level=$(adb shell getprop ro.build.version.sdk)
    local arch=$(adb shell getprop ro.product.cpu.abi)
    
    echo "Device: Android $android_version (API $api_level), $arch"
}

# Show logs
show_logs() {
    log_info "Recent logs:"
    
    # aubo-rs logs
    if adb shell "test -f $MODULE_DATA_DIR/debug.log"; then
        echo "=== aubo-rs Debug Log ==="
        adb shell "tail -20 $MODULE_DATA_DIR/debug.log" 2>/dev/null || echo "No logs found"
    fi
    
    # Magisk logs
    echo "=== Magisk Module Log ==="
    adb shell "logcat -d | grep -E '(aubo-rs|aubo_rs)' | tail -10" 2>/dev/null || echo "No Magisk logs found"
}

# Uninstall module
uninstall_module() {
    log_info "Uninstalling aubo-rs module..."
    
    # Remove module directory
    if adb shell "test -d $MAGISK_MODULES_DIR/aubo_rs"; then
        adb shell "su -c 'rm -rf $MAGISK_MODULES_DIR/aubo_rs'"
        log_success "Module removed"
    else
        log_warning "Module not found"
    fi
    
    # Optionally remove data directory
    echo -n "Remove configuration and data? [y/N]: "
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        adb shell "su -c 'rm -rf $MODULE_DATA_DIR'"
        log_success "Data directory removed"
    fi
    
    log_info "Reboot required to complete uninstallation"
}

# Main function
main() {
    case "${1:-deploy}" in
        "deploy")
            check_device
            check_device_requirements
            
            local module_zip=$(find_module)
            deploy_module "$module_zip"
            configure_module
            
            if test_deployment; then
                log_success "Deployment completed successfully!"
                log_warning "Reboot required to activate the module"
                show_status
            else
                log_error "Deployment validation failed"
                exit 1
            fi
            ;;
        
        "status")
            check_device
            show_status
            ;;
        
        "logs")
            check_device
            show_logs
            ;;
        
        "test")
            check_device
            test_deployment
            ;;
        
        "uninstall")
            check_device
            uninstall_module
            ;;
        
        "help"|"--help"|"-h")
            echo "Usage: $0 [command]"
            echo ""
            echo "Commands:"
            echo "  deploy     Deploy aubo-rs module to device (default)"
            echo "  status     Show deployment status"
            echo "  logs       Show recent logs"
            echo "  test       Test deployment"
            echo "  uninstall  Remove module from device"
            echo "  help       Show this help"
            ;;
        
        *)
            log_error "Unknown command: $1"
            echo "Use '$0 help' for usage information"
            exit 1
            ;;
    esac
}

main "$@"