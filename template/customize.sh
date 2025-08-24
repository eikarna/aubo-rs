SKIPUNZIP=1
SONAME=@SONAME@

ui_print "- Installing aubo-rs system-wide ad-blocker"
ui_print "- Version: @VERSION@"

if [ "$ARCH" != "arm64" ]; then
  abort "! Unsupported architecture: $ARCH (arm64 required)"
fi

if [ "$API" -lt 29 ]; then
  abort "! Unsupported Android version (API $API, requires 29+)"
fi

unzip -o "$ZIPFILE" 'module.prop' -d "$MODPATH"
unzip -o "$ZIPFILE" 'post-fs-data.sh' -d "$MODPATH"  
unzip -o "$ZIPFILE" 'service.sh' -d "$MODPATH"
unzip -o "$ZIPFILE" 'zn_modules.txt' -d "$MODPATH"
unzip -o "$ZIPFILE" 'sepolicy.rule' -d "$MODPATH"

mkdir -p "$MODPATH/lib"
unzip -o "$ZIPFILE" "lib/arm64/lib$SONAME.so" -d "$MODPATH"

mkdir -p "/data/adb/$SONAME"
if [ ! -f "/data/adb/$SONAME/$SONAME.toml" ]; then
  unzip -o "$ZIPFILE" "$SONAME.toml" -d "/data/adb/$SONAME"
fi