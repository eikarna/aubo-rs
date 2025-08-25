#!/system/bin/sh

echo "======================================="
echo "    aubo-rs Device Diagnostic Tool"
echo "======================================="
echo "Time: $(date)"
echo ""

MODULE_PATH="/data/adb/modules/aubo_rs"
DATA_PATH="/data/adb/aubo-rs"

echo "=== Module Directory Structure ==="
if [ -d "$MODULE_PATH" ]; then
    echo "✓ Module directory exists: $MODULE_PATH"
    echo "Contents:"
    ls -la "$MODULE_PATH"
    echo ""
    
    echo "Lib directory:"
    if [ -d "$MODULE_PATH/lib" ]; then
        ls -la "$MODULE_PATH/lib"
        echo ""
        
        echo "File details:"
        for file in "$MODULE_PATH/lib"/*; do
            if [ -f "$file" ]; then
                echo "  $(basename "$file"): $(stat -c '%s bytes, perms: %a, owner: %u:%g' "$file")"
                
                # Check SELinux context if available
                if command -v ls >/dev/null 2>&1; then
                    SELINUX_CONTEXT=$(ls -lZ "$file" 2>/dev/null | awk '{print $4}' || echo "unknown")
                    echo "    SELinux context: $SELINUX_CONTEXT"
                fi
                
                # Check file type
                if command -v file >/dev/null 2>&1; then
                    FILE_TYPE=$(file "$file" 2>/dev/null || echo "unknown")
                    echo "    File type: $FILE_TYPE"
                fi
                
                # Check if file is readable by current process
                if [ -r "$file" ]; then
                    echo "    Readable: YES"
                else
                    echo "    Readable: NO (errno would be $([ ! -r "$file" ]; echo $?))"
                fi
            fi
        done
    else
        echo "✗ Lib directory missing!"
    fi
    echo ""
    
    echo "System library overlay:"
    if [ -d "$MODULE_PATH/system/lib64" ]; then
        ls -la "$MODULE_PATH/system/lib64"
        echo ""
        
        echo "System library details:"
        for file in "$MODULE_PATH/system/lib64"/*; do
            if [ -f "$file" ]; then
                echo "  $(basename "$file"): $(stat -c '%s bytes, perms: %a, owner: %u:%g' "$file")"
                
                # Check if this file will appear in system
                SYSTEM_PATH="/system/lib64/$(basename "$file")"
                if [ -f "$SYSTEM_PATH" ]; then
                    echo "    System overlay active: YES"
                    SYSTEM_CONTEXT=$(ls -lZ "$SYSTEM_PATH" 2>/dev/null | awk '{print $4}' || echo "unknown")
                    echo "    System file context: $SYSTEM_CONTEXT"
                else
                    echo "    System overlay active: NO (file not visible in /system/lib64/)"
                fi
            fi
        done
    else
        echo "⚠ System library overlay not found"
        echo "  This may cause SELinux access issues"
    fi
    echo ""
    
    echo "Configuration files:"
    for file in module.prop zn_modules.txt; do
        if [ -f "$MODULE_PATH/$file" ]; then
            echo "✓ $file exists"
            echo "Content:"
            cat "$MODULE_PATH/$file" | sed 's/^/    /'
        else
            echo "✗ $file missing"
        fi
        echo ""
    done
else
    echo "✗ Module directory does not exist: $MODULE_PATH"
fi

echo "=== ZygiskNext Status ==="
ZYGISK_PATH="/data/adb/modules/zygisksu"
if [ -d "$ZYGISK_PATH" ]; then
    if [ -f "$ZYGISK_PATH/disable" ]; then
        echo "⚠ ZygiskNext is DISABLED"
    else
        echo "✓ ZygiskNext is enabled"
    fi
    
    echo "ZygiskNext version:"
    if [ -f "$ZYGISK_PATH/module.prop" ]; then
        grep "version=" "$ZYGISK_PATH/module.prop" | sed 's/^/    /'
    fi
else
    echo "✗ ZygiskNext not found at $ZYGISK_PATH"
fi

echo ""
echo "=== Process Status ==="
echo "ZygiskNext processes:"
pgrep -f zygisk | while read pid; do
    echo "  PID $pid: $(cat /proc/$pid/cmdline 2>/dev/null | tr '\0' ' ')"
done

echo ""
echo "aubo-rs related processes:"
pgrep -f aubo | while read pid; do
    echo "  PID $pid: $(cat /proc/$pid/cmdline 2>/dev/null | tr '\0' ' ')"
done

if [ ! $(pgrep -f aubo) ]; then
    echo "  No aubo-rs processes found"
fi

echo ""
echo "=== Library Dependencies ==="
RUST_LIB="$MODULE_PATH/lib/libaubo_rs.so"
CPP_MODULE="$MODULE_PATH/lib/aubo_module.so"

if [ -f "$RUST_LIB" ]; then
    echo "✓ Rust library exists"
    echo "Dependencies:"
    if command -v ldd >/dev/null 2>&1; then
        ldd "$RUST_LIB" 2>/dev/null | head -10 | sed 's/^/    /' || echo "    ldd failed"
    else
        echo "    ldd command not available"
    fi
else
    echo "✗ Rust library missing: $RUST_LIB"
fi

if [ -f "$CPP_MODULE" ]; then
    echo "✓ C++ module exists"
    echo "Dependencies:"
    if command -v ldd >/dev/null 2>&1; then
        ldd "$CPP_MODULE" 2>/dev/null | head -10 | sed 's/^/    /' || echo "    ldd failed"
    else
        echo "    ldd command not available"
    fi
else
    echo "✗ C++ module missing: $CPP_MODULE"
fi

echo ""
echo "=== Recent Logs ==="
echo "dmesg (last 10 aubo-rs entries):"
dmesg | grep -i aubo | tail -10 | sed 's/^/    /'

echo ""
echo "logcat (last 10 aubo-rs entries):"
logcat -d -s aubo-rs | tail -10 | sed 's/^/    /'

echo ""
echo "=== SELinux and Security Status ==="
if command -v getenforce >/dev/null 2>&1; then
    SELINUX_STATUS=$(getenforce)
    echo "SELinux status: $SELINUX_STATUS"
    
    if [ "$SELINUX_STATUS" = "Enforcing" ]; then
        echo "⚠ SELinux is enforcing - this may cause permission issues"
        echo "Check if files have correct SELinux labels:"
        
        if [ -f "$MODULE_PATH/lib/libaubo_rs.so" ]; then
            RUST_CONTEXT=$(ls -lZ "$MODULE_PATH/lib/libaubo_rs.so" 2>/dev/null | awk '{print $4}' || echo "unknown")
            echo "  Rust library context: $RUST_CONTEXT"
            
            if echo "$RUST_CONTEXT" | grep -q "adb_data_file"; then
                echo "  ⚠ WARNING: Rust library has adb_data_file context - may cause dlopen failures"
                echo "  The memfd_create approach should bypass this issue"
            fi
        fi
        
        if [ -f "$MODULE_PATH/lib/aubo_module.so" ]; then
            CPP_CONTEXT=$(ls -lZ "$MODULE_PATH/lib/aubo_module.so" 2>/dev/null | awk '{print $4}' || echo "unknown")
            echo "  C++ module context: $CPP_CONTEXT"
        fi
    fi
else
    echo "getenforce command not available"
fi

# Check current process context
if [ -f "/proc/self/attr/current" ]; then
    CURRENT_CONTEXT=$(cat /proc/self/attr/current 2>/dev/null || echo "unknown")
    echo "Current process context: $CURRENT_CONTEXT"
fi

echo ""
echo "=== Advanced Diagnostics ==="
echo "Memory and file descriptor limits:"
if [ -f "/proc/self/limits" ]; then
    grep -E "(Max open files|Max locked memory|Max address space)" /proc/self/limits | sed 's/^/  /'
fi

echo ""
echo "Available /proc/self/fd entries (first 10):"
ls -l /proc/self/fd/ 2>/dev/null | head -10 | sed 's/^/  /' || echo "  Cannot list /proc/self/fd"

echo ""
echo "Memfd support test:"
if [ -e "/proc/meminfo" ]; then
    echo "  Kernel supports /proc filesystem: YES"
else
    echo "  Kernel supports /proc filesystem: NO"
fi

# Test if we can create a simple memfd
echo "  Testing memfd_create availability: $(test -w /proc/self/fd && echo "Likely available" || echo "May not be available")"

echo ""
echo "Ashmem support test:"
if [ -e "/dev/ashmem" ]; then
    echo "  /dev/ashmem exists: YES"
    if [ -r "/dev/ashmem" ] && [ -w "/dev/ashmem" ]; then
        echo "  /dev/ashmem accessible: YES"
    else
        echo "  /dev/ashmem accessible: NO (permissions: $(stat -c %a /dev/ashmem 2>/dev/null || echo unknown))"
    fi
else
    echo "  /dev/ashmem exists: NO"
fi

echo ""
echo "=== Recommendations ==="
if [ ! -f "$MODULE_PATH/lib/aubo_module.so" ]; then
    echo "- Reinstall the module - C++ module is missing"
elif [ ! -f "$MODULE_PATH/system/lib64/libaubo_rs.so" ]; then
    echo "- Reinstall the module - System library overlay is missing"
    echo "- This is required to bypass SELinux restrictions"
elif [ ! -f "/system/lib64/libaubo_rs.so" ]; then
    echo "- System library overlay not active - check Magisk/KSU status"
    echo "- Reboot may be required for overlay to take effect"
elif [ -f "$ZYGISK_PATH/disable" ]; then
    echo "- Enable ZygiskNext in Magisk Manager"
else
    echo "- Try rebooting the device"
    echo "- Check logcat for additional error details: logcat -s aubo-rs"
    echo "- Verify SELinux policy is loaded: dmesg | grep sepolicy"
    echo "- Check for AVC denials: dmesg | grep avc | grep aubo"
fi

echo ""
echo "======================================="
echo "Copy this output and share it for debugging"
echo "======================================="