# aubo-rs Build Script for Windows
# PowerShell version of the build automation

param(
    [switch]$Clean,
    [switch]$Deploy,
    [switch]$Debug,
    [switch]$Benchmarks,
    [switch]$Help
)

# Configuration
$PROJECT_NAME = "aubo-rs"
$BUILD_TYPE = if ($Debug) { "debug" } else { "release" }
$TARGET_ARCH = "aarch64-linux-android"
$ANDROID_API_LEVEL = "29"
$NDK_VERSION = "27.1.12297006"

# Logging functions
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Blue
}

function Write-Success {
    param([string]$Message)
    Write-Host "[SUCCESS] $Message" -ForegroundColor Green
}

function Write-Warning {
    param([string]$Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Show-Help {
    Write-Host "Usage: .\build.ps1 [options]"
    Write-Host "Options:"
    Write-Host "  -Clean       Clean previous builds"
    Write-Host "  -Deploy      Deploy to connected device"
    Write-Host "  -Debug       Build in debug mode"
    Write-Host "  -Benchmarks  Run benchmarks"
    Write-Host "  -Help        Show this help"
}

function Test-Prerequisites {
    Write-Info "Checking prerequisites..."
    
    # Check Rust
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "Rust/Cargo not found. Please install Rust."
        exit 1
    }
    
    # Check Android target
    $targets = & rustup target list --installed
    if ($targets -notcontains $TARGET_ARCH) {
        Write-Info "Adding Android target: $TARGET_ARCH"
        & rustup target add $TARGET_ARCH
    }
    
    # Check Android NDK
    if (-not $env:ANDROID_NDK_HOME) {
        Write-Error "ANDROID_NDK_HOME not set. Please set it to your NDK installation."
        exit 1
    }
    
    # Check NDK tools
    $linker = Join-Path $env:ANDROID_NDK_HOME "toolchains\llvm\prebuilt\windows-x86_64\bin\aarch64-linux-android$ANDROID_API_LEVEL-clang.exe"
    if (-not (Test-Path $linker)) {
        Write-Error "Android NDK linker not found at: $linker"
        exit 1
    }
    
    Write-Success "Prerequisites check passed"
}

function Invoke-CleanBuild {
    Write-Info "Cleaning previous builds..."
    
    & cargo clean
    
    if (Test-Path "build") { Remove-Item -Recurse -Force "build" }
    if (Test-Path "release") { Remove-Item -Recurse -Force "release" }
    
    Write-Success "Build directories cleaned"
}

function Set-BuildEnvironment {
    Write-Info "Setting up build environment..."
    
    $ndk_path = $env:ANDROID_NDK_HOME
    $toolchain_path = "$ndk_path\toolchains\llvm\prebuilt\windows-x86_64\bin"
    
    $env:CC_aarch64_linux_android = "$toolchain_path\aarch64-linux-android$ANDROID_API_LEVEL-clang.exe"
    $env:AR_aarch64_linux_android = "$toolchain_path\llvm-ar.exe"
    $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $env:CC_aarch64_linux_android
    
    # Android-specific flags
    $env:CFLAGS = "-D__ANDROID_API__=$ANDROID_API_LEVEL"
    $env:CXXFLAGS = "-D__ANDROID_API__=$ANDROID_API_LEVEL"
    
    Write-Success "Environment configured for Android cross-compilation"
}

function Invoke-BuildRust {
    Write-Info "Building Rust library for Android..."
    
    $cargo_args = @("build", "--target", $TARGET_ARCH)
    if ($BUILD_TYPE -eq "release") {
        $cargo_args += "--release"
    }
    
    & cargo @cargo_args
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to build Rust library"
        exit 1
    }
    
    $lib_path = "target\$TARGET_ARCH\$BUILD_TYPE\aubo_rs.dll"
    if (-not (Test-Path $lib_path)) {
        Write-Error "Rust library not found at: $lib_path"
        exit 1
    }
    
    Write-Success "Rust library built successfully"
}

function Invoke-Tests {
    Write-Info "Running tests..."
    
    # Unit tests
    & cargo test --lib
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Unit tests failed"
        exit 1
    }
    
    # Integration tests
    & cargo test --test integration_tests
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Integration tests failed"
        exit 1
    }
    
    # Benchmarks (optional)
    if ($Benchmarks) {
        Write-Info "Running benchmarks..."
        & cargo bench
    }
    
    Write-Success "All tests passed"
}

function New-VersionInfo {
    Write-Info "Generating version information..."
    
    try {
        $git_hash = & git rev-parse --short HEAD 2>$null
        $git_count = & git rev-list --count HEAD 2>$null
    } catch {
        $git_hash = "dev"
        $git_count = "1"
    }
    
    $version = "1.0.0"
    $build_date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    
    $script:VERSION_NAME = $version
    $script:VERSION_CODE = $git_count
    $script:GIT_HASH = $git_hash
    $script:BUILD_DATE = $build_date
    
    Write-Info "Version: $VERSION_NAME ($VERSION_CODE-$GIT_HASH)"
}

function New-ModuleStructure {
    Write-Info "Preparing Magisk module structure..."
    
    $module_dir = "build\module"
    New-Item -ItemType Directory -Force -Path "$module_dir\lib\arm64" | Out-Null
    
    # Copy Rust library
    $src_lib = "target\$TARGET_ARCH\$BUILD_TYPE\aubo_rs.dll"
    $dst_lib = "$module_dir\lib\arm64\libaubo_rs.so"
    Copy-Item $src_lib $dst_lib
    
    # Copy template files
    Copy-Item -Recurse -Force "template\*" $module_dir
    
    # Copy configuration and docs
    Copy-Item "aubo-rs.toml" $module_dir
    Copy-Item "README.md" $module_dir
    
    # Process template variables
    $module_prop = Get-Content "$module_dir\module.prop" -Raw
    $module_prop = $module_prop -replace '\$\{moduleId\}', 'aubo_rs'
    $module_prop = $module_prop -replace '\$\{moduleName\}', $PROJECT_NAME
    $module_prop = $module_prop -replace '\$\{versionName\}', "$VERSION_NAME ($VERSION_CODE-$GIT_HASH)"
    $module_prop = $module_prop -replace '\$\{versionCode\}', $VERSION_CODE
    Set-Content "$module_dir\module.prop" $module_prop
    
    # Process shell scripts
    $customize = Get-Content "$module_dir\customize.sh" -Raw
    $customize = $customize -replace '@DEBUG@', 'false'
    $customize = $customize -replace '@SONAME@', 'aubo_rs'
    $customize = $customize -replace '@SUPPORTED_ABIS@', 'arm64'
    $customize = $customize -replace '@MIN_SDK@', $ANDROID_API_LEVEL
    Set-Content "$module_dir\customize.sh" $customize
    
    Write-Success "Module structure prepared"
}

function New-ModuleZip {
    Write-Info "Creating Magisk module ZIP..."
    
    $output_dir = "release"
    $zip_name = "$PROJECT_NAME-$VERSION_NAME-$VERSION_CODE-$GIT_HASH.zip"
    
    New-Item -ItemType Directory -Force -Path $output_dir | Out-Null
    
    # Create ZIP file
    $zip_path = "$output_dir\$zip_name"
    Compress-Archive -Path "build\module\*" -DestinationPath $zip_path -Force
    
    # Generate checksums
    $hash_sha256 = Get-FileHash $zip_path -Algorithm SHA256
    $hash_md5 = Get-FileHash $zip_path -Algorithm MD5
    
    "$($hash_sha256.Hash.ToLower())  $zip_name" | Out-File "$zip_path.sha256" -Encoding ascii
    "$($hash_md5.Hash.ToLower())  $zip_name" | Out-File "$zip_path.md5" -Encoding ascii
    
    Write-Success "Module ZIP created: $zip_path"
    
    # Display file information
    $file_size = [math]::Round((Get-Item $zip_path).Length / 1MB, 2)
    Write-Info "Module size: $file_size MB"
    Write-Info "SHA256: $($hash_sha256.Hash.ToLower())"
}

function Test-ModuleStructure {
    Write-Info "Validating module structure..."
    
    $module_dir = "build\module"
    $required_files = @(
        "module.prop",
        "customize.sh",
        "service.sh",
        "post-fs-data.sh",
        "zn_modules.txt",
        "sepolicy.rule",
        "lib\arm64\libaubo_rs.so",
        "aubo-rs.toml"
    )
    
    foreach ($file in $required_files) {
        $full_path = Join-Path $module_dir $file
        if (-not (Test-Path $full_path)) {
            Write-Error "Required file missing: $file"
            exit 1
        }
    }
    
    Write-Success "Module structure validation passed"
}

function New-BuildReport {
    Write-Info "Generating build report..."
    
    $report_file = "build\build-report.txt"
    
    $report = @"
aubo-rs Build Report
==================

Build Information:
- Project: $PROJECT_NAME
- Version: $VERSION_NAME  
- Version Code: $VERSION_CODE
- Git Hash: $GIT_HASH
- Build Date: $BUILD_DATE
- Build Type: $BUILD_TYPE
- Target: $TARGET_ARCH

Environment:
- PowerShell Version: $($PSVersionTable.PSVersion)
- OS: $($env:OS)
- Rust Version: $(& rustc --version)
- Cargo Version: $(& cargo --version)
- Android API Level: $ANDROID_API_LEVEL
- NDK Version: $NDK_VERSION

Files Generated:
$(Get-ChildItem release\ -Recurse -File | ForEach-Object { $_.FullName } | Sort-Object)

Module Structure:
$(Get-ChildItem build\module -Recurse -File | ForEach-Object { $_.FullName } | Sort-Object)
"@

    Set-Content $report_file $report
    Write-Success "Build report generated: $report_file"
}

function Invoke-Deploy {
    if (-not $Deploy) { return }
    
    Write-Info "Deploying module to device..."
    
    $zip_file = Get-ChildItem release\*.zip | Select-Object -First 1
    if (-not $zip_file) {
        Write-Error "No module ZIP found for deployment"
        exit 1
    }
    
    # Check if device is connected
    $devices = & adb devices 2>$null | Where-Object { $_ -match "device$" }
    if (-not $devices) {
        Write-Error "No Android device connected"
        exit 1
    }
    
    # Push module to device
    & adb push $zip_file.FullName /data/local/tmp/
    
    # Install via Magisk
    $zip_name = $zip_file.Name
    & adb shell "su -c 'magisk --install-module /data/local/tmp/$zip_name'"
    
    Write-Success "Module deployed to device"
    Write-Warning "Reboot required to activate the module"
}

# Main execution
function Main {
    if ($Help) {
        Show-Help
        return
    }
    
    Write-Info "Starting aubo-rs build process..."
    
    Test-Prerequisites
    
    if ($Clean) {
        Invoke-CleanBuild
    }
    
    New-VersionInfo
    Set-BuildEnvironment
    Invoke-Tests
    Invoke-BuildRust
    New-ModuleStructure
    Test-ModuleStructure
    New-ModuleZip
    New-BuildReport
    Invoke-Deploy
    
    Write-Success "Build completed successfully!"
    
    $zip_file = Get-ChildItem release\*.zip | Select-Object -First 1
    Write-Info "Module ready for installation: $($zip_file.FullName)"
}

# Execute main function
Main