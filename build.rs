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
        .or_else(|_| env::var("ANDROID_NDK_HOME"))
        .expect("ANDROID_NDK_ROOT, NDK_HOME, or ANDROID_NDK_HOME must be set");
    
    let toolchain_file = format!("{}/build/cmake/android.toolchain.cmake", android_ndk);
    let cpp_dir = PathBuf::from("src/cpp");
    let build_dir = PathBuf::from("target").join(&target).join("cpp_build");
    
    // Create build directory
    std::fs::create_dir_all(&build_dir).expect("Failed to create C++ build directory");
    
    // Determine architecture
    let android_abi = match target.as_str() {
        "aarch64-linux-android" => "arm64-v8a",
        "armv7-linux-androideabi" => "armeabi-v7a",
        "x86_64-linux-android" => "x86_64",
        "i686-linux-android" => "x86",
        _ => panic!("Unsupported Android target: {}", target),
    };
    
    // Run CMake configure
    let cmake_status = Command::new("cmake")
        .current_dir(&build_dir)
        .arg("-DCMAKE_TOOLCHAIN_FILE=".to_owned() + &toolchain_file)
        .arg("-DANDROID_ABI=".to_owned() + android_abi)
        .arg("-DANDROID_PLATFORM=android-29")
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("../../src/cpp")
        .status();
    
    match cmake_status {
        Ok(status) if status.success() => {
            println!("CMake configuration successful");
        }
        Ok(status) => {
            println!("cargo:warning=CMake configuration failed with exit code: {}", status);
            return; // Don't fail the build, just warn
        }
        Err(e) => {
            println!("cargo:warning=Failed to run CMake (not found?): {}", e);
            return; // Don't fail the build if CMake is not available
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
                std::fs::create_dir_all("lib").ok();
                if let Err(e) = std::fs::copy(&lib_source, &lib_dest) {
                    println!("cargo:warning=Failed to copy C++ module: {}", e);
                } else {
                    println!("C++ module copied to lib/aubo_module.so");
                }
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