#include <android/log.h>
#include <dlfcn.h>
#include <unistd.h>
#include <string>
#include <cstring>
#include <sys/socket.h>
#include <netdb.h>
#include <netinet/in.h>
#include <errno.h>

// For memfd_create and ashmem
#ifdef __NR_memfd_create
#include <sys/syscall.h>
#ifndef MFD_CLOEXEC
#define MFD_CLOEXEC 0x0001U
#endif
#endif

// For ashmem fallback
#include <sys/ioctl.h>
#ifndef ASHMEM_SET_SIZE
#define ASHMEM_SET_SIZE _IOW('d', 3, size_t)
#endif

// For Android versions that might not have memfd_create
#ifndef __NR_memfd_create
#ifdef __aarch64__
#define __NR_memfd_create 279
#elif defined(__arm__)
#define __NR_memfd_create 385
#else
#define __NR_memfd_create -1  // Not supported
#endif
#endif
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>

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

// Load library using memfd to bypass SELinux restrictions
static void* load_library_via_memfd(const char* path) {
    LOGI("Attempting memfd loading for: %s", path);
    
    // Open source file
    int source_fd = open(path, O_RDONLY);
    if (source_fd < 0) {
        LOGD("Failed to open source file %s: errno %d", path, errno);
        return nullptr;
    }
    
    // Get file size
    struct stat st;
    if (fstat(source_fd, &st) < 0) {
        LOGE("Failed to get file size for %s: errno %d", path, errno);
        close(source_fd);
        return nullptr;
    }
    
    off_t file_size = st.st_size;
    LOGD("Source file size: %ld bytes", file_size);
    
    // Create memory file descriptor
    int memfd = -1;
    
    // Try memfd_create first (Android 8+)
    if (__NR_memfd_create != -1) {
        memfd = syscall(__NR_memfd_create, "aubo_rs_lib", MFD_CLOEXEC);
        if (memfd >= 0) {
            LOGD("Created memfd using memfd_create: fd %d", memfd);
        } else {
            LOGD("memfd_create failed: errno %d", errno);
        }
    } else {
        LOGD("memfd_create not supported on this architecture");
    }
    
    // Fallback to anonymous mmap + ashmem if memfd_create failed
    if (memfd < 0) {
        // Try opening ashmem
        memfd = open("/dev/ashmem", O_RDWR);
        if (memfd >= 0) {
            // Set size using ioctl (ashmem specific)
            if (ioctl(memfd, ASHMEM_SET_SIZE, file_size) < 0) {
                LOGE("Failed to set ashmem size: errno %d", errno);
                close(memfd);
                close(source_fd);
                return nullptr;
            }
            LOGD("Created memfd using ashmem: fd %d", memfd);
        }
    }
    
    if (memfd < 0) {
        LOGE("Failed to create memory file descriptor: errno %d", errno);
        close(source_fd);
        return nullptr;
    }
    
    // For regular memfd, set the size
    if (ftruncate(memfd, file_size) < 0) {
        LOGE("Failed to set memfd size: errno %d", errno);
        close(memfd);
        close(source_fd);
        return nullptr;
    }
    
    // Copy file contents to memory fd
    char buffer[8192];
    off_t copied = 0;
    ssize_t bytes_read, bytes_written;
    
    while (copied < file_size) {
        bytes_read = read(source_fd, buffer, sizeof(buffer));
        if (bytes_read <= 0) {
            if (bytes_read < 0) {
                LOGE("Failed to read from source file: errno %d", errno);
            }
            break;
        }
        
        bytes_written = write(memfd, buffer, bytes_read);
        if (bytes_written != bytes_read) {
            LOGE("Failed to write to memfd: expected %ld, wrote %ld, errno %d", 
                 bytes_read, bytes_written, errno);
            break;
        }
        
        copied += bytes_written;
    }
    
    close(source_fd);
    
    if (copied != file_size) {
        LOGE("Incomplete copy: %ld/%ld bytes", copied, file_size);
        close(memfd);
        return nullptr;
    }
    
    LOGI("Successfully copied %ld bytes to memfd", copied);
    
    // Create path for dlopen
    std::string memfd_path = "/proc/self/fd/" + std::to_string(memfd);
    LOGD("Loading library via: %s", memfd_path.c_str());
    
    // Load library from memory fd
    void* handle = dlopen(memfd_path.c_str(), RTLD_NOW);
    
    if (!handle) {
        const char* error = dlerror();
        LOGE("Failed to dlopen memfd: %s", error ? error : "unknown error");
        close(memfd);
        return nullptr;
    }
    
    LOGI("Successfully loaded library via memfd");
    
    // Keep memfd open - it will be closed when the process exits
    // Don't close(memfd) here as dlopen needs it to remain valid
    
    return handle;
}

static bool load_rust_library() {
    // Try different possible library locations
    const char* lib_paths[] = {
        "/data/adb/modules/aubo_rs/lib/libaubo_rs.so",  // Primary location
        "/data/adb/aubo-rs/lib/libaubo_rs.so",          // Data directory fallback
        "/system/lib64/libaubo_rs.so",                  // System fallback
        "/vendor/lib64/libaubo_rs.so"                   // Vendor fallback
    };
    
    for (const char* path : lib_paths) {
        // Check if file exists and is readable
        if (access(path, R_OK) != 0) {
            LOGD("File not accessible: %s (errno: %d)", path, errno);
            continue;
        }
        
        LOGI("Found library file: %s, attempting memfd loading", path);
        
        // Try memfd loading first (bypass SELinux)
        rust_lib_handle = load_library_via_memfd(path);
        if (rust_lib_handle) {
            LOGI("Successfully loaded Rust library via memfd from: %s", path);
            break;
        }
        
        // Fallback to direct loading (may fail due to SELinux)
        LOGD("Memfd loading failed, trying direct dlopen for: %s", path);
        rust_lib_handle = dlopen(path, RTLD_NOW);
        if (rust_lib_handle) {
            LOGI("Successfully loaded Rust library via direct dlopen from: %s", path);
            break;
        } else {
            const char* error = dlerror();
            LOGD("Direct dlopen failed for %s: %s", path, error ? error : "unknown error");
        }
    }
    
    if (!rust_lib_handle) {
        LOGE("Failed to load Rust library from any location using any method");
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
extern "C" __attribute__((visibility("default"), used))
struct ZygiskNextModule zn_module = {
    .target_api_version = ZYGISK_NEXT_API_VERSION_1,
    .onModuleLoaded = onModuleLoaded,
};

// Export ZygiskNext companion module structure
extern "C" __attribute__((visibility("default"), used))
struct ZygiskNextCompanionModule zn_companion_module = {
    .target_api_version = ZYGISK_NEXT_API_VERSION_1,
    .onCompanionLoaded = onCompanionLoaded,
    .onModuleConnected = onModuleConnected,
};