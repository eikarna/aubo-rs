import org.apache.tools.ant.filters.ReplaceTokens
import java.security.MessageDigest

plugins {
    id("com.android.application") version "8.2.0" apply false
}

val moduleId = "aubo_rs"
val moduleName = "aubo-rs"
val verCode = 1
val verName = "1.0.0"
val commitHash = "dev"
val abiList = listOf("arm64-v8a")
val androidMinSdkVersion = 29

// Rust build tasks
tasks.register<Exec>("buildRust") {
    group = "rust"
    description = "Build Rust library for Android"
    
    workingDir(projectDir)
    commandLine("cargo", "build", "--target", "aarch64-linux-android", "--release")
    
    // Set environment variables for Android NDK
    environment("CC_aarch64_linux_android", "$androidSdkDirectory/ndk/27.1.12297006/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android29-clang.exe")
    environment("AR_aarch64_linux_android", "$androidSdkDirectory/ndk/27.1.12297006/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar.exe")
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", "$androidSdkDirectory/ndk/27.1.12297006/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android29-clang.exe")
}

tasks.register<Copy>("copyRustLib") {
    group = "rust"
    description = "Copy Rust library to module output"
    dependsOn("buildRust")
    
    from("target/aarch64-linux-android/release/libaubo_rs.so")
    into("module/lib/arm64")
    rename { "lib$moduleId.so" }
}

tasks.register<Sync>("prepareModule") {
    group = "module"
    description = "Prepare Magisk module files"
    dependsOn("copyRustLib")
    
    into("build/module")
    
    from("README.md")
    from("template") {
        exclude("module.prop", "customize.sh", "service.sh", "post-fs-data.sh")
    }
    
    from("template") {
        include("module.prop")
        expand(
            "moduleId" to moduleId,
            "moduleName" to moduleName,
            "versionName" to "$verName ($verCode-$commitHash)",
            "versionCode" to verCode
        )
    }
    
    from("template") {
        include("customize.sh", "service.sh", "post-fs-data.sh")
        val tokens = mapOf(
            "DEBUG" to "false",
            "SONAME" to moduleId,
            "SUPPORTED_ABIS" to "arm64",
            "MIN_SDK" to androidMinSdkVersion.toString()
        )
        filter<ReplaceTokens>("tokens" to tokens)
    }
    
    from("module/lib")
}

tasks.register<Zip>("zipModule") {
    group = "module"
    description = "Create Magisk module ZIP"
    dependsOn("prepareModule")
    
    from("build/module")
    archiveFileName.set("$moduleName-$verName-$verCode.zip")
    destinationDirectory.set(file("release"))
}