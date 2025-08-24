#!/system/bin/sh

# Service script for aubo-rs
# This runs after system boot is complete

DATA_DIR="/data/adb/aubo-rs"
LOG_FILE="$DATA_DIR/service.log"
STATUS_FILE="$DATA_DIR/status.txt"
MODULE_PROP="/data/adb/modules/aubo_rs/module.prop"

# Logging function
log_service() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] service: $1" >> "$LOG_FILE"
}

log_service "aubo-rs service starting..."

# Update status
cat > "$STATUS_FILE" << EOF
status=starting
time=$(date '+%Y-%m-%d %H:%M:%S')
message=Service script starting, preparing module
EOF

# Verify module installation
if [ ! -f "/data/adb/modules/aubo_rs/lib/aubo_rs.so" ]; then
    log_service "ERROR: Native library not found"
    cat > "$STATUS_FILE" << EOF
status=error
time=$(date '+%Y-%m-%d %H:%M:%S')
message=Native library missing - module not properly installed
EOF
    exit 1
fi

# Check ZygiskNext status
if ! pgrep -f "zygisk" > /dev/null 2>&1; then
    log_service "WARNING: ZygiskNext process not detected"
    cat > "$STATUS_FILE" << EOF
status=warning
time=$(date '+%Y-%m-%d %H:%M:%S')
message=ZygiskNext not detected - module may not work properly
EOF
else
    log_service "ZygiskNext detected successfully"
fi

# Update module.prop with detailed runtime status including dynamic description
if [ -f "$MODULE_PROP" ]; then
    log_service "Updating module.prop with runtime information"
    
    # Get current status information
    HOOKS_STATUS="❌"
    FILTER_STATUS="❌"
    ZYGISK_STATUS="❌"
    LIBRARY_STATUS="❌"
    
    # Check library
    if [ -f "/data/adb/modules/aubo_rs/lib/aubo_rs.so" ]; then
        LIBRARY_STATUS="✅"
    fi
    
    # Check ZygiskNext
    if pgrep -f "zygisk" > /dev/null 2>&1; then
        ZYGISK_STATUS="✅"
    fi
    
    # Check if debug log shows activity (indicates hooks working)
    if [ -f "$DATA_DIR/debug.log" ] && [ -s "$DATA_DIR/debug.log" ]; then
        if grep -q "initialization completed" "$DATA_DIR/debug.log" 2>/dev/null; then
            HOOKS_STATUS="✅"
            FILTER_STATUS="✅"
        fi
    fi
    
    # Count blocked requests if stats file exists
    BLOCKED_COUNT="0"
    if [ -f "$DATA_DIR/stats/stats.json" ]; then
        BLOCKED_COUNT=$(grep -o '"blocked_requests":[0-9]*' "$DATA_DIR/stats/stats.json" 2>/dev/null | cut -d':' -f2 || echo "0")
    fi
    
    # Detect root method
    ROOT_METHOD="Unknown"
    if [ -d "/data/adb/magisk" ]; then
        ROOT_METHOD="Magisk"
    elif [ -d "/data/adb/ksu" ]; then
        ROOT_METHOD="KernelSU"
    elif [ -d "/data/adb/ap" ]; then
        ROOT_METHOD="APatch"
    fi
    
    # Create dynamic description
    DYNAMIC_DESC="[${HOOKS_STATUS}Network Hooks ${FILTER_STATUS}Ad Filters ${ZYGISK_STATUS}ZygiskNext ${LIBRARY_STATUS}Library. Root: ${ROOT_METHOD}, ${BLOCKED_COUNT} blocked] System-wide ad-blocker using Rust and ZygiskNext"
    
    # Remove old runtime info if it exists
    sed -i '/^# Runtime Status/,$d' "$MODULE_PROP"
    
    # Update description with dynamic status
    sed -i "s/^description=.*/description=$DYNAMIC_DESC/" "$MODULE_PROP"
    
    # Add comprehensive runtime information
    cat >> "$MODULE_PROP" << EOF

# Runtime Status
serviceStartTime=$(date '+%Y-%m-%d %H:%M:%S')
systemBootTime=$(uptime | awk '{print $3}' | sed 's/,//')
androidApi=$(getprop ro.build.version.sdk)
deviceModel=$(getprop ro.product.model)
androidVersion=$(getprop ro.build.version.release)
cpuArch=$(getprop ro.product.cpu.abi)
rootMethod=$ROOT_METHOD

# Module Status
moduleActive=true
hooksInstalled=${HOOKS_STATUS}
filtersLoaded=${FILTER_STATUS}
zygiskDetected=${ZYGISK_STATUS}
libraryLoaded=${LIBRARY_STATUS}
blockedRequests=${BLOCKED_COUNT}
dataDirectory=$DATA_DIR
nativeLibrary=/data/adb/modules/aubo_rs/lib/aubo_rs.so
configFile=$DATA_DIR/aubo-rs.toml
logFile=$LOG_FILE

# ZygiskNext Integration
zygiskProcessActive=$(if pgrep -f "zygisk" > /dev/null 2>&1; then echo "true"; else echo "false"; fi)
zygiskModule=$(if [ -f "/data/adb/modules/zygisksu/module.prop" ]; then echo "true"; else echo "false"; fi)

# Diagnostics
lastHealthCheck=pending
debugLogEnabled=true
dmesgLogging=enabled
statusFile=$STATUS_FILE
EOF
    
    log_service "module.prop updated with dynamic status: $DYNAMIC_DESC"
fi

# Create comprehensive status update
cat > "$STATUS_FILE" << EOF
status=active
time=$(date '+%Y-%m-%d %H:%M:%S')
message=Module service started successfully
version=$(grep "version=" "$MODULE_PROP" | cut -d'=' -f2)
boot_time=$(uptime | awk '{print $3}' | sed 's/,//')
api_level=$(getprop ro.build.version.sdk)
zygisk_detected=$(if pgrep -f "zygisk" > /dev/null 2>&1; then echo "true"; else echo "false"; fi)
library_exists=$(if [ -f "/data/adb/modules/aubo_rs/lib/aubo_rs.so" ]; then echo "true"; else echo "false"; fi)
config_exists=$(if [ -f "$DATA_DIR/aubo-rs.toml" ]; then echo "true"; else echo "false"; fi)
EOF

# Run initial health check
if [ -f "$DATA_DIR/health_check.sh" ]; then
    log_service "Running initial health check"
    sh "$DATA_DIR/health_check.sh"
fi

# Set up monitoring (runs every 5 minutes)
{
    while true; do
        sleep 300  # 5 minutes
        
        # Update last seen time
        sed -i "s/time=.*/time=$(date '+%Y-%m-%d %H:%M:%S')/" "$STATUS_FILE"
        
        # Update dynamic description in module.prop
        if [ -f "$MODULE_PROP" ]; then
            # Get current status
            HOOKS_STATUS="❌"
            FILTER_STATUS="❌"
            ZYGISK_STATUS="❌"
            LIBRARY_STATUS="❌"
            
            # Check library
            if [ -f "/data/adb/modules/aubo_rs/lib/aubo_rs.so" ]; then
                LIBRARY_STATUS="✅"
            fi
            
            # Check ZygiskNext
            if pgrep -f "zygisk" > /dev/null 2>&1; then
                ZYGISK_STATUS="✅"
            fi
            
            # Check if debug log shows recent activity
            if [ -f "$DATA_DIR/debug.log" ] && [ -s "$DATA_DIR/debug.log" ]; then
                # Check if there's recent activity (within last 30 minutes)
                RECENT_ACTIVITY=$(find "$DATA_DIR/debug.log" -mmin -30 2>/dev/null)
                if [ -n "$RECENT_ACTIVITY" ] && grep -q "initialization completed\|System initialized" "$DATA_DIR/debug.log" 2>/dev/null; then
                    HOOKS_STATUS="✅"
                    FILTER_STATUS="✅"
                fi
            fi
            
            # Count blocked requests
            BLOCKED_COUNT="0"
            if [ -f "$DATA_DIR/stats/stats.json" ]; then
                BLOCKED_COUNT=$(grep -o '"blocked_requests":[0-9]*' "$DATA_DIR/stats/stats.json" 2>/dev/null | cut -d':' -f2 || echo "0")
            fi
            
            # Detect root method
            ROOT_METHOD="Unknown"
            if [ -d "/data/adb/modules/magisk_busybox" ] || [ -f "/system/xbin/magisk" ]; then
                ROOT_METHOD="Magisk"
            elif [ -d "/data/adb/modules/kernelsu" ] || [ -f "/system/bin/ksu" ]; then
                ROOT_METHOD="KernelSU"
            elif [ -d "/data/adb/modules/apatch" ] || [ -f "/system/bin/apd" ]; then
                ROOT_METHOD="APatch"
            fi
            
            # Update description
            DYNAMIC_DESC="[${HOOKS_STATUS}Network Hooks ${FILTER_STATUS}Ad Filters ${ZYGISK_STATUS}ZygiskNext ${LIBRARY_STATUS}Library. Root: ${ROOT_METHOD}, ${BLOCKED_COUNT} blocked] System-wide ad-blocker using Rust and ZygiskNext"
            sed -i "s/^description=.*/description=$DYNAMIC_DESC/" "$MODULE_PROP"
        fi
        
        # Quick health check
        if [ ! -f "/data/adb/modules/aubo_rs/lib/aubo_rs.so" ]; then
            echo "[$(date '+%Y-%m-%d %H:%M:%S')] service: WARNING - Native library missing" >> "$LOG_FILE"
            sed -i "s/status=.*/status=error/" "$STATUS_FILE"
            sed -i "s/message=.*/message=Native library disappeared/" "$STATUS_FILE"
        fi
        
        # Log heartbeat
        echo "[$(date '+%Y-%m-%d %H:%M:%S')] service: Heartbeat - module monitoring active" >> "$LOG_FILE"
    done
} &

# Log successful startup
log_service "aubo-rs service started successfully"
log_service "Monitoring enabled - status updates every 5 minutes"
log_service "Check status with: sh $DATA_DIR/check_status.sh"

# Trigger dmesg logging to help with debugging
echo "aubo-rs: Service script completed - module should be active" > /dev/kmsg

exit 0