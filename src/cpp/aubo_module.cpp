#include <android/log.h>
#include <dlfcn.h>
#include <unistd.h>
#include <string>
#include <cstring>
#include <sys/socket.h>
#include <netdb.h>
#include <netinet/in.h>

#include "zygisk_next_api.h"

#define LOGD(...) __android_log_print(ANDROID_LOG_DEBUG, "aubo-rs", __VA_ARGS__)
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, "aubo-rs", __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, "aubo-rs", __VA_ARGS__)

// Function pointers for Rust library functions
typedef int (*aubo_initialize_fn)(const char* config_path);
typedef int (*aubo_shutdown_fn)();
typedef int (*aubo_should_block_request_fn)(const char* url, const char* request_type, const char* origin);

// Global state
static ZygiskNextAPI api_table;
static void* handle = nullptr;
static void* rust_lib_handle = nullptr;
static aubo_initialize_fn aubo_initialize = nullptr;
static aubo_shutdown_fn aubo_shutdown = nullptr;
static aubo_should_block_request_fn aubo_should_block_request = nullptr;

// Hook function prototypes
static int (*old_connect)(int sockfd, const struct sockaddr *addr, socklen_t addrlen) = nullptr;
static struct hostent* (*old_gethostbyname)(const char *name) = nullptr;
static int (*old_getaddrinfo)(const char *node, const char *service, const struct addrinfo *hints, struct addrinfo **res) = nullptr;

// Network request logging and blocking
static int my_connect(int sockfd, const struct sockaddr *addr, socklen_t addrlen) {
    // Extract connection information for analysis
    if (addr && aubo_should_block_request) {
        // For demonstration, we'll just log the connection attempt
        LOGD("connect() intercepted - sockfd: %d", sockfd);
    }
    
    // Call original function
    return old_connect(sockfd, addr, addrlen);
}

static struct hostent* my_gethostbyname(const char *name) {
    if (name && aubo_should_block_request) {
        LOGD("gethostbyname() intercepted - hostname: %s", name);
        
        // Check if this hostname should be blocked
        if (aubo_should_block_request(name, "dns", "gethostbyname")) {
            LOGI("Blocked DNS resolution for: %s", name);
            // Return NULL to simulate DNS failure for blocked domains
            return nullptr;
        }
    }
    
    return old_gethostbyname(name);
}

static int my_getaddrinfo(const char *node, const char *service, const struct addrinfo *hints, struct addrinfo **res) {
    if (node && aubo_should_block_request) {
        LOGD("getaddrinfo() intercepted - node: %s, service: %s", node, service ? service : "null");
        
        // Check if this hostname should be blocked
        if (aubo_should_block_request(node, "dns", "getaddrinfo")) {
            LOGI("Blocked DNS resolution for: %s", node);
            // Return error code to simulate DNS failure
            return EAI_NONAME;
        }
    }
    
    return old_getaddrinfo(node, service, hints, res);
}

static bool load_rust_library() {
    // Try different possible library locations
    const char* lib_paths[] = {
        "/data/adb/modules/aubo_rs/lib/libaubo_rs.so",
        "/data/adb/modules/aubo_rs/lib/aubo_rs.so",
        "libaubo_rs.so"
    };
    
    for (const char* path : lib_paths) {
        rust_lib_handle = dlopen(path, RTLD_LAZY);
        if (rust_lib_handle) {
            LOGI("Successfully loaded Rust library from: %s", path);
            break;
        } else {
            LOGD("Failed to load Rust library from %s: %s", path, dlerror());
        }
    }
    
    if (!rust_lib_handle) {
        LOGE("Failed to load Rust library from any location");
        return false;
    }
    
    // Load function symbols
    aubo_initialize = (aubo_initialize_fn)dlsym(rust_lib_handle, "aubo_initialize");
    aubo_shutdown = (aubo_shutdown_fn)dlsym(rust_lib_handle, "aubo_shutdown");
    aubo_should_block_request = (aubo_should_block_request_fn)dlsym(rust_lib_handle, "aubo_should_block_request");
    
    if (!aubo_initialize || !aubo_shutdown || !aubo_should_block_request) {
        LOGE("Failed to load required symbols from Rust library");
        LOGE("aubo_initialize: %p", aubo_initialize);
        LOGE("aubo_shutdown: %p", aubo_shutdown);
        LOGE("aubo_should_block_request: %p", aubo_should_block_request);
        dlclose(rust_lib_handle);
        rust_lib_handle = nullptr;
        return false;
    }
    
    LOGI("All Rust library symbols loaded successfully");
    return true;
}

static bool install_network_hooks() {
    // Create symbol resolver for libc
    auto resolver = api_table.newSymbolResolver("libc.so", nullptr);
    if (!resolver) {
        LOGE("Failed to create symbol resolver for libc.so");
        return false;
    }
    
    bool success = true;
    
    // Hook connect()
    size_t size;
    auto connect_addr = api_table.symbolLookup(resolver, "connect", false, &size);
    if (connect_addr) {
        if (api_table.inlineHook(connect_addr, (void*)my_connect, (void**)&old_connect) == ZN_SUCCESS) {
            LOGI("Successfully hooked connect() at %p", connect_addr);
        } else {
            LOGE("Failed to hook connect()");
            success = false;
        }
    } else {
        LOGE("Failed to find connect() symbol");
        success = false;
    }
    
    // Hook gethostbyname()
    auto gethostbyname_addr = api_table.symbolLookup(resolver, "gethostbyname", false, &size);
    if (gethostbyname_addr) {
        if (api_table.inlineHook(gethostbyname_addr, (void*)my_gethostbyname, (void**)&old_gethostbyname) == ZN_SUCCESS) {
            LOGI("Successfully hooked gethostbyname() at %p", gethostbyname_addr);
        } else {
            LOGE("Failed to hook gethostbyname()");
            success = false;
        }
    } else {
        LOGE("Failed to find gethostbyname() symbol");
        success = false;
    }
    
    // Hook getaddrinfo()
    auto getaddrinfo_addr = api_table.symbolLookup(resolver, "getaddrinfo", false, &size);
    if (getaddrinfo_addr) {
        if (api_table.inlineHook(getaddrinfo_addr, (void*)my_getaddrinfo, (void**)&old_getaddrinfo) == ZN_SUCCESS) {
            LOGI("Successfully hooked getaddrinfo() at %p", getaddrinfo_addr);
        } else {
            LOGE("Failed to hook getaddrinfo()");
            success = false;
        }
    } else {
        LOGE("Failed to find getaddrinfo() symbol");
        success = false;
    }
    
    api_table.freeSymbolResolver(resolver);
    return success;
}

// ZygiskNext module lifecycle callbacks
static void onModuleLoaded(void* self_handle, const struct ZygiskNextAPI* api) {
    LOGI("aubo-rs ZygiskNext module loading...");
    
    // Copy API table
    memcpy(&api_table, api, sizeof(struct ZygiskNextAPI));
    handle = self_handle;
    
    // Load Rust library
    if (!load_rust_library()) {
        LOGE("Failed to load Rust library - module initialization failed");
        return;
    }
    
    // Initialize Rust module
    const char* config_path = "/data/adb/aubo-rs/aubo-rs.toml";
    if (aubo_initialize(config_path) != 0) {
        LOGE("Failed to initialize Rust module");
        return;
    }
    
    // Install network hooks
    if (!install_network_hooks()) {
        LOGE("Failed to install network hooks");
        return;
    }
    
    LOGI("aubo-rs module loaded successfully - ad-blocking active");
    
    // Write to dmesg for debugging
    FILE* kmsg = fopen("/dev/kmsg", "w");
    if (kmsg) {
        fprintf(kmsg, "<6>aubo-rs: ZygiskNext module loaded and initialized successfully\n");
        fclose(kmsg);
    }
}

static void onCompanionLoaded() {
    LOGI("aubo-rs companion module loaded");
}

static void onModuleConnected(int fd) {
    LOGI("aubo-rs module connected with fd: %d", fd);
    // For now, just close the connection
    close(fd);
}

// Export ZygiskNext module structure
__attribute__((visibility("default"), unused))
struct ZygiskNextModule zn_module = {
    .target_api_version = ZYGISK_NEXT_API_VERSION_1,
    .onModuleLoaded = onModuleLoaded,
};

// Export ZygiskNext companion module structure
__attribute__((visibility("default"), unused))
struct ZygiskNextCompanionModule zn_companion_module = {
    .target_api_version = ZYGISK_NEXT_API_VERSION_1,
    .onCompanionLoaded = onCompanionLoaded,
    .onModuleConnected = onModuleConnected,
};