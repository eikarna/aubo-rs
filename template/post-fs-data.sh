#!/system/bin/sh

# Post-fs-data script for aubo-rs
# This runs early in the boot process

DATA_DIR="/data/adb/aubo-rs"
LOG_FILE="$DATA_DIR/boot.log"

# Ensure data directory exists
mkdir -p "$DATA_DIR"
chmod 755 "$DATA_DIR"

echo "[$(date '+%Y-%m-%d %H:%M:%S')] post-fs-data: Starting aubo-rs early initialization" >> "$LOG_FILE"

# Update status
cat > "$DATA_DIR/status.txt" << EOF
status=initializing
time=$(date '+%Y-%m-%d %H:%M:%S')
message=Post-fs-data phase starting
EOF

# Create default config if missing
if [ ! -f "$DATA_DIR/aubo-rs.toml" ]; then
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] post-fs-data: Creating default configuration" >> "$LOG_FILE"
    cat > "$DATA_DIR/aubo-rs.toml" << 'EOF'
[general]
enabled = true
data_dir = "/data/adb/aubo-rs"
max_memory_mb = 64
max_cpu_percent = 5.0

[filters]
enabled = true
filters_dir = "/data/adb/aubo-rs/filters"
auto_update = true
update_interval = 3600

[hooks]
enabled = true

[stats]
enabled = true
EOF
fi

echo "[$(date '+%Y-%m-%d %H:%M:%S')] post-fs-data: Completed" >> "$LOG_FILE"