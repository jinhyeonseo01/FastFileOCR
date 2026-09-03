//! Driver-only probes; no GPU engine or CUDA toolkit is needed to choose a download.
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct Hardware {
    pub cuda: bool,
    pub vulkan: bool,
}
impl Hardware {
    pub fn detect() -> Self {
        #[cfg(windows)]
        {
            Self {
                cuda: windows::cuda().unwrap_or(false),
                vulkan: windows::vulkan().unwrap_or(false),
            }
        }
        #[cfg(not(windows))]
        {
            Self::default()
        }
    }
    pub fn candidates(self, requested: &str) -> Vec<&str> {
        match requested {
            "auto" if self.cuda => vec!["cuda", "cpu"],
            "auto" if self.vulkan => vec!["vulkan", "cpu"],
            "auto" => vec!["cpu"],
            other => vec![other],
        }
    }
}
#[cfg(windows)]
mod windows {
    use libloading::Library;
    use std::{
        ffi::{c_char, c_void},
        path::PathBuf,
        ptr,
    };
    fn driver(name: &str) -> Option<Library> {
        let mut path = [0u16; 32768];
        let n = unsafe {
            windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW(
                path.as_mut_ptr(),
                path.len() as u32,
            )
        } as usize;
        if n == 0 || n >= path.len() {
            return None;
        }
        let path = PathBuf::from(String::from_utf16_lossy(&path[..n])).join(name);
        unsafe { Library::new(path).ok() }
    }
    pub(super) fn cuda() -> Option<bool> {
        let library = driver("nvcuda.dll")?;
        unsafe {
            let init = library
                .get::<unsafe extern "system" fn(u32) -> i32>(b"cuInit\0")
                .ok()?;
            let count = library
                .get::<unsafe extern "system" fn(*mut i32) -> i32>(b"cuDeviceGetCount\0")
                .ok()?;
            let version = library
                .get::<unsafe extern "system" fn(*mut i32) -> i32>(b"cuDriverGetVersion\0")
                .ok()?;
            let get_device = library
                .get::<unsafe extern "system" fn(*mut i32, i32) -> i32>(b"cuDeviceGet\0")
                .ok()?;
            let attribute = library
                .get::<unsafe extern "system" fn(*mut i32, i32, i32) -> i32>(
                    b"cuDeviceGetAttribute\0",
                )
                .ok()?;
            let (mut devices, mut driver_version) = (0, 0);
            if init(0) != 0
                || count(&mut devices) != 0
                || version(&mut driver_version) != 0
                || driver_version < super::super::manifest().cuda_driver_minimum
            {
                return Some(false);
            }
            // CUDA 13 removes pre-Turing GPU support (compute capability < 7.5).
            Some((0..devices).any(|ordinal| {
                let mut device = 0;
                if get_device(&mut device, ordinal) != 0 {
                    return false;
                }
                let (mut major, mut minor) = (0, 0);
                attribute(&mut major, 75, device) == 0
                    && attribute(&mut minor, 76, device) == 0
                    && (major > 7 || (major == 7 && minor >= 5))
            }))
        }
    }
    #[repr(C)]
    struct ApplicationInfo {
        kind: u32,
        next: *const c_void,
        name: *const c_char,
        version: u32,
        engine: *const c_char,
        engine_version: u32,
        api_version: u32,
    }
    #[repr(C)]
    struct InstanceInfo {
        kind: u32,
        next: *const c_void,
        flags: u32,
        application: *const ApplicationInfo,
        layer_count: u32,
        layers: *const *const c_char,
        extension_count: u32,
        extensions: *const *const c_char,
    }
    pub(super) fn vulkan() -> Option<bool> {
        let library = driver("vulkan-1.dll")?;
        unsafe {
            let create = library
                .get::<unsafe extern "system" fn(
                    *const InstanceInfo,
                    *const c_void,
                    *mut *mut c_void,
                ) -> i32>(b"vkCreateInstance\0")
                .ok()?;
            let destroy = library
                .get::<unsafe extern "system" fn(*mut c_void, *const c_void)>(
                    b"vkDestroyInstance\0",
                )
                .ok()?;
            let enumerate = library
                .get::<unsafe extern "system" fn(*mut c_void, *mut u32, *mut *mut c_void) -> i32>(
                    b"vkEnumeratePhysicalDevices\0",
                )
                .ok()?;
            let application = ApplicationInfo {
                kind: 0,
                next: ptr::null(),
                name: ptr::null(),
                version: 0,
                engine: ptr::null(),
                engine_version: 0,
                api_version: (1 << 22) | (2 << 12),
            };
            let info = InstanceInfo {
                kind: 1,
                next: ptr::null(),
                flags: 0,
                application: &application,
                layer_count: 0,
                layers: ptr::null(),
                extension_count: 0,
                extensions: ptr::null(),
            };
            let mut instance = ptr::null_mut();
            if create(&info, ptr::null(), &mut instance) != 0 || instance.is_null() {
                return Some(false);
            }
            let mut count = 0;
            let result = enumerate(instance, &mut count, ptr::null_mut());
            destroy(instance, ptr::null());
            Some(result == 0 && count > 0)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_mode_downloads_at_most_one_gpu_backend() {
        assert_eq!(
            Hardware {
                cuda: true,
                vulkan: true
            }
            .candidates("auto"),
            ["cuda", "cpu"]
        );
        assert_eq!(
            Hardware {
                cuda: false,
                vulkan: true
            }
            .candidates("auto"),
            ["vulkan", "cpu"]
        );
        assert_eq!(Hardware::default().candidates("auto"), ["cpu"]);
        assert_eq!(Hardware::default().candidates("cuda"), ["cuda"]);
    }
}
