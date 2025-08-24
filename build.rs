// Build script for aubo-rs Android integration
// This script sets up the build environment for compiling Rust code for Android
// and generates bindings for C interoperability.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=src/cpp/aubo_module.cpp");
    println!("cargo:rerun-if-changed=src/cpp/CMakeLists.txt");

    // Get target information
    let target = env::var("TARGET").unwrap();
    let is_android = target.contains("android");
    
    if is_android {
        configure_android_build();
        build_cpp_module();
    }

    // Generate C bindings if bindgen feature is enabled
    #[cfg(feature = "bindgen")]
    {
        generate_bindings();
    }

    // Set up linking for Android
    setup_android_linking();
}

fn build_cpp_module() {
    let target = env::var("TARGET").unwrap();
    let android_ndk = env::var("ANDROID_NDK_ROOT")
        .or_else(|_| env::var("NDK_HOME"))
        .or_else(|_| env::var("ANDROID_NDK_HOME"));
    
    // Only try to build C++ module if NDK is available
    let ndk_path = match android_ndk {
        Ok(path) => path,
        Err(_) => {
            println!("cargo:warning=Android NDK not found, skipping C++ module build");
            return;
        }
    };
    
    let toolchain_file = format!("{}/build/cmake/android.toolchain.cmake", ndk_path);
    if !std::path::Path::new(&toolchain_file).exists() {
        println!("cargo:warning=CMake toolchain file not found, skipping C++ module build");
        return;
    }
    
    let cpp_dir = PathBuf::from("src/cpp");
    if !cpp_dir.exists() {
        println!("cargo:warning=C++ source directory not found, skipping C++ module build");
        return;
    }
    
    let build_dir = PathBuf::from("target").join(&target).join("cpp_build");
    
    // Create build directory
    if let Err(e) = std::fs::create_dir_all(&build_dir) {
        println!("cargo:warning=Failed to create C++ build directory: {}", e);
        return;
    }
    
    // Determine architecture
    let android_abi = match target.as_str() {
        "aarch64-linux-android" => "arm64-v8a",
        "armv7-linux-androideabi" => "armeabi-v7a",
        "x86_64-linux-android" => "x86_64",
        "i686-linux-android" => "x86",
        _ => {
            println!("cargo:warning=Unsupported Android target for C++ build: {}", target);
            return;
        }
    };
    
    // Check if cmake is available
    let cmake_available = Command::new("cmake")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);
    
    if !cmake_available {
        println!("cargo:warning=CMake not found, skipping C++ module build");
        println!("cargo:warning=Install CMake to build the full ZygiskNext module");
        return;
    }
    
    // Run CMake configure
    let cmake_status = Command::new("cmake")
        .current_dir(&build_dir)
        .arg("-DCMAKE_TOOLCHAIN_FILE=".to_owned() + &toolchain_file)
        .arg("-DANDROID_ABI=".to_owned() + android_abi)
        .arg("-DANDROID_PLATFORM=android-29")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("../../../src/cpp")
        .status();
    
    match cmake_status {
        Ok(status) if status.success() => {
            println!("CMake configuration successful");
        }
        Ok(status) => {
            println!("cargo:warning=CMake configuration failed with exit code: {}", status);
            return;
        }
        Err(e) => {
            println!("cargo:warning=Failed to run CMake: {}", e);
            return;
        }
    }
    
    // Run CMake build
    let build_status = Command::new("cmake")
        .current_dir(&build_dir)
        .arg("--build")
        .arg(".")
        .arg("--config")
        .arg("Release")
        .status();
    
    match build_status {
        Ok(status) if status.success() => {
            println!("C++ module build successful");
            
            // Copy the built library to the lib directory
            let lib_source = build_dir.join("libaubo_module.so");
            let lib_dest = PathBuf::from("lib").join("aubo_module.so");
            
            if lib_source.exists() {
                if let Err(e) = std::fs::create_dir_all("lib") {
                    println!("cargo:warning=Failed to create lib directory: {}", e);
                    return;
                }
                
                if let Err(e) = std::fs::copy(&lib_source, &lib_dest) {
                    println!("cargo:warning=Failed to copy C++ module: {}", e);
                } else {
                    println!("C++ module copied to lib/aubo_module.so");
                }
            } else {
                println!("cargo:warning=C++ module not found after build: {:?}", lib_source);
            }
        }
        Ok(status) => {
            println!("cargo:warning=C++ module build failed with exit code: {}", status);
        }
        Err(e) => {
            println!("cargo:warning=Failed to build C++ module: {}", e);
        }
    }
}

fn configure_android_build() {
    println!("cargo:rustc-env=TARGET_OS=android");
    
    // Android-specific configuration
    println!("cargo:rustc-link-lib=log");
    println!("cargo:rustc-link-lib=dl");
    println!("cargo:rustc-link-lib=c");
    
    // Set minimum API level
    println!("cargo:rustc-env=ANDROID_MIN_API=29");
}

#[cfg(feature = "bindgen")]
fn generate_bindings() {
    use bindgen;
    
    // Generate bindings for ZygiskNext API
    let bindings = bindgen::Builder::default()
        .header("zygisk_next_api.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

fn setup_android_linking() {
    let target = env::var("TARGET").unwrap();
    
    match target.as_str() {
        "aarch64-linux-android" => {
            println!("cargo:rustc-link-search=native=/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/14.0.7/lib/linux/aarch64");
        }
        "armv7-linux-androideabi" => {
            println!("cargo:rustc-link-search=native=/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/14.0.7/lib/linux/arm");
        }
        "x86_64-linux-android" => {
            println!("cargo:rustc-link-search=native=/opt/android-ndk/toolchains/llvm/prebuilt/linux-x86_64/lib64/clang/14.0.7/lib/linux/x86_64");
        }
        _ => {}
    }
}