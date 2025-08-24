#!/system/bin/sh

SKIPUNZIP=1
SONAME=@SONAME@
VERSION="@VERSION@"
MODULE_ID="aubo_rs"
DATA_DIR="/data/adb/aubo-rs"
INSTALL_LOG="$DATA_DIR/logs/install.log"

# Enhanced logging functions with multiple output streams
log_info() {
    ui_print "- $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] INFO: $1" >> "$INSTALL_LOG"
    echo "<6>aubo-rs-installer: INFO: $1" > /dev/kmsg 2>/dev/null || true
}

log_error() {
    ui_print "! ERROR: $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] ERROR: $1" >> "$INSTALL_LOG"
    echo "<3>aubo-rs-installer: ERROR: $1" > /dev/kmsg 2>/dev/null || true
}

log_warn() {
    ui_print "* WARNING: $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] WARN: $1" >> "$INSTALL_LOG"
    echo "<4>aubo-rs-installer: WARN: $1" > /dev/kmsg 2>/dev/null || true
}

log_debug() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] DEBUG: $1" >> "$INSTALL_LOG"
    echo "<7>aubo-rs-installer: DEBUG: $1" > /dev/kmsg 2>/dev/null || true
}

# Create data directory early with proper error handling
if ! mkdir -p "$DATA_DIR"; then
    ui_print "! FATAL: Cannot create data directory $DATA_DIR"
    exit 1
fi

if ! chmod 755 "$DATA_DIR"; then
    ui_print "! FATAL: Cannot set permissions on $DATA_DIR"
    exit 1
fi

# Initialize installation log
echo "[$(date '+%Y-%m-%d %H:%M:%S')] === aubo-rs Installation Started ===" > "$INSTALL_LOG"

log_info "Installing aubo-rs system-wide ad-blocker"
log_info "Version: $VERSION"
log_info "Architecture: $ARCH"
log_info "Android API: $API"
log_info "Module ID: $MODULE_ID"
log_debug "Data directory: $DATA_DIR"
log_debug "Module path: $MODPATH"

# Enhanced system requirements validation
log_info "Validating system requirements..."

# Check architecture
if [ "$ARCH" != "arm64" ]; then
    log_error "Unsupported architecture: $ARCH (arm64 required)"
    cat > "$DATA_DIR/status.txt" << EOF
status=error
time=$(date '+%Y-%m-%d %H:%M:%S')
message=Unsupported architecture: $ARCH
arch=$ARCH
api_level=$API
EOF
    abort "! Unsupported architecture: $ARCH. This module requires arm64 devices."
fi
log_info "✓ Architecture check passed: $ARCH"

# Check Android API level
if [ "$API" -lt 29 ]; then
    log_error "Unsupported Android version (API $API, requires 29+)"
    cat > "$DATA_DIR/status.txt" << EOF
status=error
time=$(date '+%Y-%m-%d %H:%M:%S')
message=Unsupported Android API: $API (requires 29+)
arch=$ARCH
api_level=$API
EOF
    abort "! Unsupported Android version. This module requires Android 10+ (API 29+)."
fi
log_info "✓ Android API check passed: $API"

# Enhanced ZygiskNext detection and validation
log_info "Checking ZygiskNext installation..."
ZYGISK_MODULE_PATH="/data/adb/modules/zygisksu"
if [ ! -d "$ZYGISK_MODULE_PATH" ]; then
    log_error "ZygiskNext module not found at $ZYGISK_MODULE_PATH"
    log_error "This module requires ZygiskNext to function properly"
    log_info "Please install ZygiskNext first: https://github.com/Dr-TSNG/ZygiskNext"
    cat > "$DATA_DIR/status.txt" << EOF
status=error
time=$(date '+%Y-%m-%d %H:%M:%S')
message=ZygiskNext module not found - required dependency missing
arch=$ARCH
api_level=$API
zygisk_detected=false
EOF
    abort "! ZygiskNext is required but not installed"
fi

if [ ! -f "$ZYGISK_MODULE_PATH/module.prop" ]; then
    log_warn "ZygiskNext module.prop not found - installation may be incomplete"
else
    ZYGISK_VERSION=$(grep "version=" "$ZYGISK_MODULE_PATH/module.prop" | cut -d'=' -f2)
    log_info "✓ ZygiskNext detected - Version: $ZYGISK_VERSION"
fi

# Check if ZygiskNext is enabled
if [ -f "$ZYGISK_MODULE_PATH/disable" ]; then
    log_warn "ZygiskNext is currently disabled - please enable it for aubo-rs to work"
else
    log_info "✓ ZygiskNext is enabled"
fi

log_info "Extracting module files..."

# Extract core module files
for file in module.prop post-fs-data.sh service.sh zn_modules.txt sepolicy.rule; do
    if ! unzip -o "$ZIPFILE" "$file" -d "$MODPATH"; then
        log_error "Failed to extract $file"
        abort "! Failed to extract essential files"
    fi
done

# Extract native library
mkdir -p "$MODPATH/lib"
if ! unzip -o "$ZIPFILE" "lib/arm64/lib$SONAME.so" -d "$MODPATH"; then
    log_error "Failed to extract native library"
    abort "! Failed to extract native library"
fi

# Move library to correct location
if [ -f "$MODPATH/lib/arm64/lib$SONAME.so" ]; then
    mv "$MODPATH/lib/arm64/lib$SONAME.so" "$MODPATH/lib/lib$SONAME.so"
    rm -rf "$MODPATH/lib/arm64"
    log_info "Native library installed successfully"
else
    log_error "Native library not found after extraction"
    abort "! Native library verification failed"
fi

log_info "Setting up data directory and configuration..."

# Extract configuration file if it doesn't exist
if [ ! -f "$DATA_DIR/$SONAME.toml" ]; then
    if unzip -o "$ZIPFILE" "$SONAME.toml" -d "$DATA_DIR"; then
        log_info "Created default configuration"
    else
        log_warn "Failed to extract config, will use built-in defaults"
    fi
fi

# Create additional directories
mkdir -p "$DATA_DIR/logs"
mkdir -p "$DATA_DIR/filters"
mkdir -p "$DATA_DIR/stats"

# Set proper permissions
chmod -R 755 "$DATA_DIR"
chown -R root:root "$DATA_DIR" 2>/dev/null || true

# Create initial status file
cat > "$DATA_DIR/status.txt" << EOF
status=installed
version=$VERSION
install_time=$(date '+%Y-%m-%d %H:%M:%S')
arch=$ARCH
api_level=$API
message=Module installed successfully, awaiting system startup
EOF

# Update module.prop with detailed information
cat >> "$MODPATH/module.prop" << EOF

# Runtime Information
author=aubo-rs Project
support=https://github.com/aubo-rs/aubo-rs
minApi=$API
maxApi=9999

# Status Information  
installTime=$(date '+%Y-%m-%d %H:%M:%S')
installArch=$ARCH
installVersion=$VERSION
EOF

log_info "Creating comprehensive diagnostic tools..."

# Enhanced health check script with comprehensive system analysis
cat > "$DATA_DIR/health_check.sh" << 'EOF'
#!/system/bin/sh

DATA_DIR="/data/adb/aubo-rs"
LOG_FILE="$DATA_DIR/logs/health.log"
MODULE_PATH="/data/adb/modules/aubo_rs"

echo "[$(date '+%Y-%m-%d %H:%M:%S')] === Comprehensive Health Check ===" >> "$LOG_FILE"
echo "System: $(getprop ro.build.version.release) API $(getprop ro.build.version.sdk)" >> "$LOG_FILE"
echo "Device: $(getprop ro.product.model) ($(getprop ro.product.cpu.abi))" >> "$LOG_FILE"
echo "" >> "$LOG_FILE"

# Module Status
echo "=== Module Status ===" >> "$LOG_FILE"
if [ -f "$DATA_DIR/status.txt" ]; then
    echo "✓ Status file exists" >> "$LOG_FILE"
    cat "$DATA_DIR/status.txt" >> "$LOG_FILE"
else
    echo "✗ Status file missing" >> "$LOG_FILE"
fi
echo "" >> "$LOG_FILE"

# File Verification
echo "=== Critical Files ===" >> "$LOG_FILE"
if [ -f "$MODULE_PATH/lib/libaubo_rs.so" ]; then
    SIZE=$(stat -c%s "$MODULE_PATH/lib/libaubo_rs.so" 2>/dev/null || echo "0")
    echo "✓ Native library: $SIZE bytes" >> "$LOG_FILE"
else
    echo "✗ Native library: MISSING" >> "$LOG_FILE"
fi

if [ -f "$DATA_DIR/aubo-rs.toml" ]; then
    echo "✓ Configuration file: Present" >> "$LOG_FILE"
else
    echo "⚠ Configuration file: Missing" >> "$LOG_FILE"
fi

# ZygiskNext Status
echo "" >> "$LOG_FILE"
echo "=== ZygiskNext Status ===" >> "$LOG_FILE"
if [ -d "/data/adb/modules/zygisksu" ]; then
    if [ -f "/data/adb/modules/zygisksu/disable" ]; then
        echo "⚠ ZygiskNext: DISABLED" >> "$LOG_FILE"
    else
        echo "✓ ZygiskNext: ENABLED" >> "$LOG_FILE"
    fi
else
    echo "✗ ZygiskNext: NOT FOUND" >> "$LOG_FILE"
fi

if pgrep -f "zygisk" > /dev/null 2>&1; then
    echo "✓ ZygiskNext process: RUNNING" >> "$LOG_FILE"
else
    echo "⚠ ZygiskNext process: NOT DETECTED" >> "$LOG_FILE"
fi

# Log Analysis
echo "" >> "$LOG_FILE"
echo "=== Log Analysis ===" >> "$LOG_FILE"
if [ -f "$DATA_DIR/logs/debug.log" ]; then
    LINES=$(wc -l < "$DATA_DIR/logs/debug.log" 2>/dev/null || echo "0")
    echo "✓ Debug log: $LINES lines" >> "$LOG_FILE"
    if [ "$LINES" -gt 0 ]; then
        echo "Recent entries:" >> "$LOG_FILE"
        tail -3 "$DATA_DIR/logs/debug.log" | sed 's/^/  /' >> "$LOG_FILE"
    fi
else
    echo "⚠ Debug log: NOT FOUND" >> "$LOG_FILE"
fi

# dmesg Check
DMESG_COUNT=$(dmesg | grep -c "aubo-rs" 2>/dev/null || echo "0")
if [ "$DMESG_COUNT" -gt 0 ]; then
    echo "✓ dmesg entries: $DMESG_COUNT found" >> "$LOG_FILE"
else
    echo "⚠ dmesg entries: NONE" >> "$LOG_FILE"
fi

echo "" >> "$LOG_FILE"
echo "[$(date '+%Y-%m-%d %H:%M:%S')] Health check completed" >> "$LOG_FILE"
EOF

chmod 755 "$DATA_DIR/health_check.sh"

# Enhanced status checker with real-time information
cat > "$DATA_DIR/check_status.sh" << 'EOF'
#!/system/bin/sh

echo "========================================"
echo "        aubo-rs Status Report"
echo "========================================"
echo "Time: $(date '+%Y-%m-%d %H:%M:%S')"
echo "Device: $(getprop ro.product.model)"
echo "Android: $(getprop ro.build.version.release) (API $(getprop ro.build.version.sdk))"
echo ""

echo "--- Current Status ---"
if [ -f "/data/adb/aubo-rs/status.txt" ]; then
    while IFS='=' read -r key value; do
        [ -n "$key" ] && printf "%-12s: %s\n" "$key" "$value"
    done < "/data/adb/aubo-rs/status.txt"
else
    echo "❌ Status file not found!"
fi

echo ""
echo "--- System Checks ---"
# Module files
if [ -f "/data/adb/modules/aubo_rs/lib/libaubo_rs.so" ]; then
    echo "✅ Native library: Installed"
else
    echo "❌ Native library: Missing"
fi

# ZygiskNext
if [ -d "/data/adb/modules/zygisksu" ] && [ ! -f "/data/adb/modules/zygisksu/disable" ]; then
    echo "✅ ZygiskNext: Active"
elif [ -d "/data/adb/modules/zygisksu" ]; then
    echo "⚠️ ZygiskNext: Disabled"
else
    echo "❌ ZygiskNext: Not installed"
fi

# Logging status
if [ -f "/data/adb/aubo-rs/logs/debug.log" ]; then
    LINES=$(wc -l < "/data/adb/aubo-rs/logs/debug.log" 2>/dev/null || echo "0")
    if [ "$LINES" -gt 0 ]; then
        echo "✅ Debug logging: Active ($LINES entries)"
    else
        echo "⚠️ Debug logging: File exists but empty"
    fi
else
    echo "❌ Debug logging: No log file"
fi

# dmesg entries
DMESG_COUNT=$(dmesg | grep -c "aubo-rs" 2>/dev/null || echo "0")
if [ "$DMESG_COUNT" -gt 0 ]; then
    echo "✅ dmesg entries: $DMESG_COUNT found"
else
    echo "⚠️ dmesg entries: None (check if module loaded)"
fi

echo ""
echo "--- Troubleshooting ---"
echo "Status check: sh /data/adb/aubo-rs/check_status.sh"
echo "Health check: sh /data/adb/aubo-rs/health_check.sh"
echo "Live logging: logcat -s aubo-rs"
echo "dmesg filter: dmesg | grep aubo-rs"
echo "Debug info:   sh /data/adb/aubo-rs/debug_helper.sh"
echo ""
if [ ! -f "/data/adb/aubo-rs/logs/debug.log" ] || [ "$DMESG_COUNT" -eq 0 ]; then
    echo "🔄 If module appears inactive, try rebooting your device"
fi
echo "========================================"
EOF

chmod 755 "$DATA_DIR/check_status.sh"

log_info "Installation completed successfully!"
log_info "Running final verification..."

# Final verification
ERRORS=0
CRITICAL_FILES="$MODPATH/lib/lib$SONAME.so $DATA_DIR/aubo-rs.toml $DATA_DIR/status.txt $DATA_DIR/check_status.sh $DATA_DIR/health_check.sh"
for file in $CRITICAL_FILES; do
    if [ ! -f "$file" ]; then
        log_error "Critical file missing: $file"
        ERRORS=$((ERRORS + 1))
    fi
done

if [ $ERRORS -eq 0 ]; then
    log_info "✅ All critical files verified successfully"
else
    log_error "❌ Found $ERRORS missing files - module may not work properly"
fi

# Update module.prop with comprehensive status
cat >> "$MODPATH/module.prop" << EOF

# Installation Status
installationStatus=completed
installationTime=$(date '+%Y-%m-%d %H:%M:%S')
installationErrors=$ERRORS
installerVersion=enhanced-v2.0

# System Information
systemArch=$ARCH
systemAPI=$API
zygiskNextDetected=true

# File Locations
dataDir=$DATA_DIR
configFile=$DATA_DIR/aubo-rs.toml
nativeLib=$MODPATH/lib/lib$SONAME.so
statusFile=$DATA_DIR/status.txt
healthCheck=$DATA_DIR/health_check.sh
statusCheck=$DATA_DIR/check_status.sh

# Features
features=enhanced_logging,dmesg_integration,health_monitoring,status_reporting
diagnosticTools=available
fallbackConfig=enabled
EOF

log_info ""
log_info "🎉 Installation Summary:"
log_info "   ✅ Version: $VERSION installed successfully"
log_info "   📁 Data directory: $DATA_DIR"
log_info "   ⚙️ Configuration: $DATA_DIR/aubo-rs.toml"
log_info "   🔍 Status checker: sh '$DATA_DIR/check_status.sh'"
log_info "   🚑 Health monitor: sh '$DATA_DIR/health_check.sh'"
log_info "   📝 Live logging: logcat -s aubo-rs"
log_info "   🔎 dmesg filter: dmesg | grep aubo-rs"
log_info ""
log_info "🔄 Next steps:"
log_info "   1. Reboot your device to activate the module"
log_info "   2. Enable 'dmesg log for developer' in ZygiskNext WebUI"
log_info "   3. After reboot, run status check to verify operation"
log_info "   4. If issues occur, use the diagnostic tools provided"
log_info ""
log_info "Installation completed at $(date '+%Y-%m-%d %H:%M:%S')"
