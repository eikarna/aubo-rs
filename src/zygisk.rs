//! ZygiskNext API bindings for Rust
//!
//! This module provides safe Rust bindings for the ZygiskNext API,
//! enabling system-level process injection and hooking capabilities.

use std::ffi::{c_char, c_int, c_void, CString};

use std::ptr;

use crate::error::{Result, ZygiskError};

/// ZygiskNext API version
pub const ZYGISK_NEXT_API_VERSION_1: c_int = 3;

/// Success return code
pub const ZN_SUCCESS: c_int = 0;
/// Failure return code  
pub const ZN_FAILED: c_int = 1;

/// Opaque type for symbol resolver
#[repr(C)]
pub struct ZnSymbolResolver {
    _private: [u8; 0],
}

/// ZygiskNext API function table
#[repr(C)]
pub struct ZygiskNextAPI {
    /// PLT hook function
    pub plt_hook: unsafe extern "C" fn(
        base_addr: *mut c_void,
        symbol: *const c_char,
        hook_handler: *mut c_void,
        original: *mut *mut c_void,
    ) -> c_int,

    /// Inline hook function
    pub inline_hook: unsafe extern "C" fn(
        target: *mut c_void,
        addr: *mut c_void,
        original: *mut *mut c_void,
    ) -> c_int,

    /// Inline unhook function
    pub inline_unhook: unsafe extern "C" fn(target: *mut c_void) -> c_int,

    /// Create new symbol resolver
    pub new_symbol_resolver: unsafe extern "C" fn(
        path: *const c_char,
        base_addr: *mut c_void,
    ) -> *mut ZnSymbolResolver,

    /// Free symbol resolver
    pub free_symbol_resolver: unsafe extern "C" fn(resolver: *mut ZnSymbolResolver),

    /// Get base address of library
    pub get_base_address: unsafe extern "C" fn(resolver: *mut ZnSymbolResolver) -> *mut c_void,

    /// Symbol lookup function
    pub symbol_lookup: unsafe extern "C" fn(
        resolver: *mut ZnSymbolResolver,
        name: *const c_char,
        prefix: bool,
        size: *mut usize,
    ) -> *mut c_void,

    /// Iterate through symbols
    pub for_each_symbols: unsafe extern "C" fn(
        resolver: *mut ZnSymbolResolver,
        callback: unsafe extern "C" fn(
            name: *const c_char,
            addr: *mut c_void,
            size: usize,
            data: *mut c_void,
        ) -> bool,
        data: *mut c_void,
    ),

    /// Connect to companion module
    pub connect_companion: unsafe extern "C" fn(handle: *mut c_void) -> c_int,
}

/// ZygiskNext module callbacks
#[repr(C)]
pub struct ZygiskNextModule {
    /// Target API version
    pub target_api_version: c_int,
    /// Module loaded callback
    pub on_module_loaded: unsafe extern "C" fn(self_handle: *mut c_void, api: *const ZygiskNextAPI),
}

/// ZygiskNext companion module callbacks
#[repr(C)]
pub struct ZygiskNextCompanionModule {
    /// Target API version
    pub target_api_version: c_int,
    /// Companion loaded callback
    pub on_companion_loaded: unsafe extern "C" fn(),
    /// Module connected callback
    pub on_module_connected: unsafe extern "C" fn(fd: c_int),
}

/// Safe wrapper for ZygiskNext API
pub struct ZygiskApi {
    api: *const ZygiskNextAPI,
    handle: *mut c_void,
}

unsafe impl Send for ZygiskApi {}
unsafe impl Sync for ZygiskApi {}

impl ZygiskApi {
    /// Create new ZygiskApi wrapper
    /// 
    /// # Safety
    /// The API pointer must be valid and the handle must be valid for the lifetime of this object
    pub unsafe fn new(api: *const ZygiskNextAPI, handle: *mut c_void) -> Self {
        Self { api, handle }
    }

    /// Install PLT hook
    pub fn plt_hook(
        &self,
        base_addr: *mut c_void,
        symbol: &str,
        hook_handler: *mut c_void,
    ) -> Result<*mut c_void> {
        let symbol_cstr = CString::new(symbol).map_err(|_| {
            ZygiskError::IpcError {
                reason: "Invalid symbol name".to_string(),
            }
        })?;

        let mut original: *mut c_void = ptr::null_mut();
        
        let result = unsafe {
            ((*self.api).plt_hook)(
                base_addr,
                symbol_cstr.as_ptr(),
                hook_handler,
                &mut original,
            )
        };

        if result == ZN_SUCCESS {
            Ok(original)
        } else {
            Err(ZygiskError::InjectionFailed {
                process: "unknown".to_string(),
                reason: format!("PLT hook failed for symbol: {}", symbol),
            }
            .into())
        }
    }

    /// Install inline hook
    pub fn inline_hook(
        &self,
        target: *mut c_void,
        hook_handler: *mut c_void,
    ) -> Result<*mut c_void> {
        let mut original: *mut c_void = ptr::null_mut();
        
        let result = unsafe {
            ((*self.api).inline_hook)(target, hook_handler, &mut original)
        };

        if result == ZN_SUCCESS {
            Ok(original)
        } else {
            Err(ZygiskError::InjectionFailed {
                process: "unknown".to_string(),
                reason: "Inline hook installation failed".to_string(),
            }
            .into())
        }
    }

    /// Remove inline hook
    pub fn inline_unhook(&self, original_fn: *mut c_void) -> Result<()> {
        let result = unsafe {
            ((*self.api).inline_unhook)(original_fn)
        };

        if result == ZN_SUCCESS {
            Ok(())
        } else {
            Err(ZygiskError::InjectionFailed {
                process: "unknown".to_string(),
                reason: "Inline hook removal failed".to_string(),
            }
            .into())
        }
    }

    /// Create symbol resolver
    pub fn new_symbol_resolver(&self, library_path: &str) -> Result<SymbolResolver> {
        let path_cstr = CString::new(library_path).map_err(|_| {
            ZygiskError::IpcError {
                reason: "Invalid library path".to_string(),
            }
        })?;

        let resolver = unsafe {
            ((*self.api).new_symbol_resolver)(path_cstr.as_ptr(), ptr::null_mut())
        };

        if resolver.is_null() {
            Err(ZygiskError::ModuleLoadFailed {
                reason: format!("Failed to create symbol resolver for: {}", library_path),
            }
            .into())
        } else {
            Ok(SymbolResolver::new(resolver, self.api))
        }
    }

    /// Connect to companion module
    pub fn connect_companion(&self) -> Result<i32> {
        let fd = unsafe { ((*self.api).connect_companion)(self.handle) };

        if fd < 0 {
            Err(ZygiskError::CompanionConnectionFailed {
                reason: "Failed to connect to companion module".to_string(),
            }
            .into())
        } else {
            Ok(fd)
        }
    }
}

/// Safe wrapper for symbol resolver
pub struct SymbolResolver {
    resolver: *mut ZnSymbolResolver,
    api: *const ZygiskNextAPI,
}

impl SymbolResolver {
    fn new(resolver: *mut ZnSymbolResolver, api: *const ZygiskNextAPI) -> Self {
        Self { resolver, api }
    }

    /// Look up symbol address
    pub fn lookup_symbol(&self, symbol: &str) -> Result<Option<(*mut c_void, usize)>> {
        let symbol_cstr = CString::new(symbol).map_err(|_| {
            ZygiskError::IpcError {
                reason: "Invalid symbol name".to_string(),
            }
        })?;

        let mut size: usize = 0;
        let addr = unsafe {
            ((*self.api).symbol_lookup)(
                self.resolver,
                symbol_cstr.as_ptr(),
                false,
                &mut size,
            )
        };

        if addr.is_null() {
            Ok(None)
        } else {
            Ok(Some((addr, size)))
        }
    }

    /// Get library base address
    pub fn get_base_address(&self) -> *mut c_void {
        unsafe { ((*self.api).get_base_address)(self.resolver) }
    }
}

impl Drop for SymbolResolver {
    fn drop(&mut self) {
        unsafe {
            ((*self.api).free_symbol_resolver)(self.resolver);
        }
    }
}

/// Global storage for the ZygiskNext API
static mut GLOBAL_ZYGISK_API: Option<ZygiskApi> = None;

/// Get the global ZygiskNext API instance
pub fn get_zygisk_api() -> Option<&'static ZygiskApi> {
    unsafe { GLOBAL_ZYGISK_API.as_ref() }
}

/// Initialize the global ZygiskNext API
/// 
/// # Safety
/// This should only be called once during module initialization
pub unsafe fn init_zygisk_api(api: *const ZygiskNextAPI, handle: *mut c_void) {
    unsafe {
        GLOBAL_ZYGISK_API = Some(ZygiskApi::new(api, handle));
    }
}

/// Module loaded callback implementation
unsafe extern "C" fn on_module_loaded(self_handle: *mut c_void, api: *const ZygiskNextAPI) {
    // Initialize the global API
    unsafe {
        init_zygisk_api(api, self_handle);
    }
    
    // Log to dmesg for debugging
    log_dmesg("aubo-rs: ZygiskNext module loaded, API initialized");

    // Call the Rust initialization
    if let Err(e) = crate::initialize_from_zygisk() {
        log::error!("Failed to initialize aubo-rs from ZygiskNext: {}", e);
        log_dmesg(&format!("aubo-rs: CRITICAL - Initialization failed: {}", e));
    } else {
        log_dmesg("aubo-rs: Module loaded and initialized successfully");
    }
}

/// Companion loaded callback implementation
unsafe extern "C" fn on_companion_loaded() {
    log::info!("aubo-rs companion module loaded");
    log_dmesg("aubo-rs: Companion module loaded");
}

/// Module connected callback implementation
unsafe extern "C" fn on_module_connected(fd: c_int) {
    log_dmesg(&format!("aubo-rs: Module connected with fd: {}", fd));
    if let Err(e) = crate::handle_companion_connection(fd) {
        log::error!("Failed to handle companion connection: {}", e);
        log_dmesg(&format!("aubo-rs: Companion connection failed: {}", e));
    }
}

/// Simple dmesg logging function
fn log_dmesg(message: &str) {
    use std::process::Command;
    
    // Try multiple logging methods
    let _ = Command::new("log")
        .arg("-t")
        .arg("aubo-rs")
        .arg(message)
        .output();
        
    // Also try direct syslog
    let _ = Command::new("logger")
        .arg("-t")
        .arg("aubo-rs")
        .arg(message)
        .output();
}

/// Export the ZygiskNext module structure
#[no_mangle]
pub static zn_module: ZygiskNextModule = ZygiskNextModule {
    target_api_version: ZYGISK_NEXT_API_VERSION_1,
    on_module_loaded,
};

/// Export the ZygiskNext companion module structure
#[no_mangle]
pub static zn_companion_module: ZygiskNextCompanionModule = ZygiskNextCompanionModule {
    target_api_version: ZYGISK_NEXT_API_VERSION_1,
    on_companion_loaded,
    on_module_connected,
};