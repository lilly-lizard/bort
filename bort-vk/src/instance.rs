use crate::{string_to_c_string_vec, ALLOCATION_CALLBACK_NONE};
use ash::{
    extensions::{ext, khr},
    prelude::VkResult,
    vk::{self, make_api_version},
    Entry,
};
#[cfg(feature = "raw-window-handle-05")]
use raw_window_handle_05::RawDisplayHandle;
#[cfg(feature = "raw-window-handle-06")]
use raw_window_handle_06::RawDisplayHandle;
use std::{error, ffi::NulError, fmt, os::raw::c_char, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
}

impl ApiVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub const fn as_vk_uint(&self) -> u32 {
        make_api_version(0, self.major, self.minor, 0)
    }
}

pub struct Instance {
    inner: ash::Instance,
    /// The highest version of vulkan that the application is designed to use.
    /// [More info here](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkApplicationInfo.html)
    max_api_version: ApiVersion,

    // dependencies
    entry: Arc<Entry>,
}

impl Instance {
    /// This function will figure out the required surface extensions based on `display_handle`
    /// so there's no need to supply them.
    /// e.g. VK_KHR_surface and platform specific ones like VK_KHR_win32_surface.
    pub fn new<S>(
        entry: Arc<Entry>,
        max_api_version: ApiVersion,
        display_handle: RawDisplayHandle,
        layer_names: impl IntoIterator<Item = S>,
        extension_names: impl IntoIterator<Item = S>,
    ) -> Result<Self, InstanceError>
    where
        S: Into<Vec<u8>>,
    {
        let appinfo = vk::ApplicationInfo::builder().api_version(max_api_version.as_vk_uint());

        let layer_name_cstrings = string_to_c_string_vec(layer_names)
            .map_err(|e| InstanceError::LayerStringConversion(e))?;
        let extension_name_cstrings = string_to_c_string_vec(extension_names)
            .map_err(|e| InstanceError::ExtensionStringConversion(e))?;

        let mut extension_name_ptrs = extension_name_cstrings
            .iter()
            .map(|cstring| cstring.as_ptr())
            .collect::<Vec<_>>();
        let layer_name_ptrs = layer_name_cstrings
            .iter()
            .map(|cstring| cstring.as_ptr())
            .collect::<Vec<_>>();

        let display_extension_names = required_surface_extensions(display_handle)
            .map_err(|e| InstanceError::UnsupportedRawDisplayHandle(e))?;
        extension_name_ptrs.extend_from_slice(display_extension_names);

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_name_ptrs)
            .enabled_extension_names(&extension_name_ptrs);

        let instance_inner =
            unsafe { entry.create_instance(&create_info, ALLOCATION_CALLBACK_NONE) }
                .map_err(|e| InstanceError::Creation(e))?;

        Ok(Self {
            entry,
            inner: instance_inner,
            max_api_version,
        })
    }

    pub unsafe fn new_from_create_info(
        entry: Arc<Entry>,
        create_info_builder: vk::InstanceCreateInfoBuilder,
    ) -> Result<Self, InstanceError> {
        let instance_inner =
            unsafe { entry.create_instance(&create_info_builder, ALLOCATION_CALLBACK_NONE) }
                .map_err(|e| InstanceError::Creation(e))?;

        let max_api_version = if create_info_builder.p_application_info != std::ptr::null() {
            let api_version_combined =
                unsafe { *create_info_builder.p_application_info }.api_version;
            ApiVersion {
                major: vk::api_version_major(api_version_combined),
                minor: vk::api_version_minor(api_version_combined),
            }
        } else {
            ApiVersion { major: 0, minor: 0 }
        };

        Ok(Self {
            inner: instance_inner,
            max_api_version,
            entry,
        })
    }

    /// Vulkan 1.0 features
    pub fn physical_device_features_1_0(
        &self,
        physical_device_handle: vk::PhysicalDevice,
    ) -> vk::PhysicalDeviceFeatures {
        unsafe {
            self.inner()
                .get_physical_device_features(physical_device_handle)
        }
    }

    /// Vulkan 1.1 features. If api version < 1.1, these cannot be populated.
    pub fn physical_device_features_1_1(
        &self,
        physical_device_handle: vk::PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan11Features> {
        if self.max_api_version < ApiVersion::new(1, 1) {
            return None;
        }

        let mut features_1_1 = vk::PhysicalDeviceVulkan11Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::builder().push_next(&mut features_1_1);
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device_handle, &mut features)
        };

        Some(features_1_1)
    }

    /// Vulkan 1.2 features. If api version < 1.2, these cannot be populated.
    pub fn physical_device_features_1_2(
        &self,
        physical_device_handle: vk::PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan12Features> {
        if self.max_api_version < ApiVersion::new(1, 2) {
            return None;
        }

        let mut features_1_2 = vk::PhysicalDeviceVulkan12Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::builder().push_next(&mut features_1_2);
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device_handle, &mut features)
        };

        Some(features_1_2)
    }

    pub fn enumerate_physical_devices(&self) -> VkResult<Vec<vk::PhysicalDevice>> {
        unsafe { self.inner.enumerate_physical_devices() }
    }

    // Getters

    /// Access the `ash::Instance` struct that `self` contains. Allows you to access vulkan instance
    /// functions.
    #[inline]
    pub fn inner(&self) -> &ash::Instance {
        &self.inner
    }

    #[inline]
    pub fn max_api_version(&self) -> ApiVersion {
        self.max_api_version
    }

    #[inline]
    pub fn entry(&self) -> &Arc<Entry> {
        &self.entry
    }
}

/// Query the required instance extensions for creating a surface from a display handle.
///
/// _Note: this function was copied from [ash](https://github.com/ash-rs/ash) to allow for better
/// dependency control._
///
/// This [`RawDisplayHandle`] can typically be acquired from a window, but is usually also
/// accessible earlier through an "event loop" concept to allow querying required instance
/// extensions and creation of a compatible Vulkan instance prior to creating a window.
///
/// The returned extensions will include all extension dependencies.
pub fn required_surface_extensions(
    display_handle: RawDisplayHandle,
) -> VkResult<&'static [*const c_char]> {
    let extensions = match display_handle {
        RawDisplayHandle::Windows(_) => {
            const WINDOWS_EXTS: [*const c_char; 2] = [
                khr::Surface::name().as_ptr(),
                khr::Win32Surface::name().as_ptr(),
            ];
            &WINDOWS_EXTS
        }

        RawDisplayHandle::Wayland(_) => {
            const WAYLAND_EXTS: [*const c_char; 2] = [
                khr::Surface::name().as_ptr(),
                khr::WaylandSurface::name().as_ptr(),
            ];
            &WAYLAND_EXTS
        }

        RawDisplayHandle::Xlib(_) => {
            const XLIB_EXTS: [*const c_char; 2] = [
                khr::Surface::name().as_ptr(),
                khr::XlibSurface::name().as_ptr(),
            ];
            &XLIB_EXTS
        }

        RawDisplayHandle::Xcb(_) => {
            const XCB_EXTS: [*const c_char; 2] = [
                khr::Surface::name().as_ptr(),
                khr::XcbSurface::name().as_ptr(),
            ];
            &XCB_EXTS
        }

        RawDisplayHandle::Android(_) => {
            const ANDROID_EXTS: [*const c_char; 2] = [
                khr::Surface::name().as_ptr(),
                khr::AndroidSurface::name().as_ptr(),
            ];
            &ANDROID_EXTS
        }

        RawDisplayHandle::AppKit(_) | RawDisplayHandle::UiKit(_) => {
            const METAL_EXTS: [*const c_char; 2] = [
                khr::Surface::name().as_ptr(),
                ext::MetalSurface::name().as_ptr(),
            ];
            &METAL_EXTS
        }

        _ => return Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT),
    };

    Ok(extensions)
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_instance(ALLOCATION_CALLBACK_NONE);
        }
    }
}

// ~~ Error ~~

#[derive(Debug, Clone)]
pub enum InstanceError {
    UnsupportedRawDisplayHandle(vk::Result),
    AppNameStringConversion(NulError),
    ExtensionStringConversion(NulError),
    LayerStringConversion(NulError),
    Creation(vk::Result),
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRawDisplayHandle(e) => {
                write!(f, "unsupported raw display handle type! return from ash_window::enumerate_required_extensions: {}", e)
            }
            Self::AppNameStringConversion(e) => {
                write!(f, "failed to convert app name string to c string: {}", e)
            }
            Self::ExtensionStringConversion(e) => {
                write!(
                    f,
                    "failed to convert extension name string to c string: {}",
                    e
                )
            }
            Self::LayerStringConversion(e) => {
                write!(f, "failed to convert layer name string to c string: {}", e)
            }
            Self::Creation(e) => {
                write!(f, "failed to create device {}", e)
            }
        }
    }
}

impl error::Error for InstanceError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::UnsupportedRawDisplayHandle(e) => Some(e),
            Self::AppNameStringConversion(e) => Some(e),
            Self::ExtensionStringConversion(e) => Some(e),
            Self::LayerStringConversion(e) => Some(e),
            Self::Creation(e) => Some(e),
        }
    }
}

// ~~ Tests ~~

#[test]
fn api_version_ordering() {
    let ver_1_1 = ApiVersion::new(1, 1);
    let ver_1_2 = ApiVersion::new(1, 2);
    assert!(ver_1_1 < ver_1_2);
}
