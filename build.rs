// Build script for aubo-rs Android integration
// This script sets up the build environment for compiling Rust code for Android
// and generates bindings for C interoperability.

use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Get target information
    let target = env::var("TARGET").unwrap();
    let is_android = target.contains("android");
    
    if is_android {
        configure_android_build();
    }

    // Generate C bindings if bindgen feature is enabled
    #[cfg(feature = "bindgen")]
    {
        use std::path::PathBuf;
        generate_bindings();
    }

    // Set up linking for Android
    setup_android_linking();
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