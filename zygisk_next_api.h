#pragma once

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

#define ZYGISK_NEXT_API_VERSION_1 3

#define ZN_SUCCESS 0
#define ZN_FAILED 1

struct ZnSymbolResolver;

struct ZygiskNextAPI {
    int (*pltHook)(void* base_addr, const char* symbol, void* hook_handler, void** original);
    int (*inlineHook)(void* target, void* addr, void** original);
    int (*inlineUnhook)(void* target);
    struct ZnSymbolResolver* (*newSymbolResolver)(const char* path, void* base_addr);
    void (*freeSymbolResolver)(struct ZnSymbolResolver* resolver);
    void* (*getBaseAddress)(struct ZnSymbolResolver* resolver);
    void* (*symbolLookup)(struct ZnSymbolResolver* resolver, const char* name, bool prefix, size_t* size);
    void (*forEachSymbols)(struct ZnSymbolResolver* resolver,
                           bool (*callback)(const char* name, void* addr, size_t size, void* data),
                           void* data);
    int (*connectCompanion)(void* handle);
};

struct ZygiskNextModule {
    int target_api_version;
    void (*onModuleLoaded)(void* self_handle, const struct ZygiskNextAPI* api);
};

struct ZygiskNextCompanionModule {
    int target_api_version;
    void (*onCompanionLoaded)();
    void (*onModuleConnected)(int fd);
};

extern struct ZygiskNextModule zn_module;
extern struct ZygiskNextCompanionModule zn_companion_module;

#ifdef __cplusplus
}
#endif