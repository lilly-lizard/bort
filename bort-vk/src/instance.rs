use crate::{PhysicalDevice, PhysicalDeviceFeatures, ALLOCATION_CALLBACK_NONE};
use ash::{
    ext::metal_surface,
    khr::{android_surface, surface, wayland_surface, win32_surface, xcb_surface, xlib_surface},
    prelude::VkResult,
    vk::{self, make_api_version},
    Entry,
};
#[cfg(feature = "raw-window-handle-05")]
use raw_window_handle_05::RawDisplayHandle;
#[cfg(feature = "raw-window-handle-06")]
use raw_window_handle_06::RawDisplayHandle;
use std::{
    error,
    ffi::{CStr, CString},
    fmt,
    os::raw::c_char,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    // note: deriving Ord means that the first struct member declared takes precidence, so major is declared first
    pub major: u32,
    pub minor: u32,
}

impl ApiVersion {
    pub const V1_0: Self = Self { major: 1, minor: 0 };
    pub const V1_1: Self = Self { major: 1, minor: 1 };
    pub const V1_2: Self = Self { major: 1, minor: 2 };
    pub const V1_3: Self = Self { major: 1, minor: 3 };
    pub const V1_4: Self = Self { major: 1, minor: 4 };

    pub const fn as_vk_uint(&self) -> u32 {
        make_api_version(0, self.major, self.minor, 0)
    }
}

pub struct Instance {
    inner: ash::Instance,
    /// The maximum version of vulkan that the application is designed to use.
    /// [More info here](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkApplicationInfo.html)
    max_api_version: ApiVersion,
    enabled_extensions: Vec<CString>,
    enabled_layers: Vec<CString>,

    // dependencies
    entry: Arc<Entry>,
}

impl Instance {
    /// This function will figure out the required surface extensions based on `display_handle`
    /// e.g. VK_KHR_surface and platform specific ones like VK_KHR_win32_surface. Will also check
    /// if the display extensions and extension_names are supported.
    pub fn new_with_display_extensions(
        entry: Arc<Entry>,
        max_api_version: ApiVersion,
        display_handle: RawDisplayHandle,
        layer_names: Vec<CString>,
        other_extension_names: Vec<CString>,
    ) -> Result<Self, InstanceError> {
        let display_extension_name_cstrs = Self::required_surface_extensions(display_handle)?;
        let mut display_extension_names: Vec<CString> = display_extension_name_cstrs
            .iter()
            .map(|&cstr| cstr.to_owned())
            .collect();

        let mut extension_names = other_extension_names;
        extension_names.append(&mut display_extension_names);

        let unsupported_display_extensions =
            Self::any_unsupported_extensions(&entry, None, extension_names.clone())
                .map_err(InstanceError::Creation)?;
        if !unsupported_display_extensions.is_empty() {
            return Err(InstanceError::ExtensionsNotPresent(
                unsupported_display_extensions,
            ));
        }

        Self::new(entry, max_api_version, layer_names, extension_names)
    }

    /// Doesn't check for extension/layer support.
    pub fn new(
        entry: Arc<Entry>,
        max_api_version: ApiVersion,
        layer_names: Vec<CString>,
        extension_names: Vec<CString>,
    ) -> Result<Self, InstanceError> {
        let layer_name_ptrs: Vec<*const c_char> =
            layer_names.iter().map(|cstring| cstring.as_ptr()).collect();
        let extension_name_ptrs: Vec<*const c_char> = extension_names
            .iter()
            .map(|cstring| cstring.as_ptr())
            .collect();

        let appinfo = vk::ApplicationInfo::default().api_version(max_api_version.as_vk_uint());
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_name_ptrs)
            .enabled_extension_names(&extension_name_ptrs);

        let instance_inner =
            unsafe { entry.create_instance(&create_info, ALLOCATION_CALLBACK_NONE) }
                .map_err(InstanceError::Creation)?;

        Ok(Self {
            entry,
            inner: instance_inner,
            max_api_version,
            enabled_extensions: extension_names,
            enabled_layers: layer_names,
        })
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        entry: Arc<Entry>,
        create_info: vk::InstanceCreateInfo,
        enabled_extensions: Vec<CString>,
        enabled_layers: Vec<CString>,
    ) -> Result<Self, InstanceError> {
        let instance_inner =
            unsafe { entry.create_instance(&create_info, ALLOCATION_CALLBACK_NONE) }
                .map_err(InstanceError::Creation)?;

        let max_api_version = if !create_info.p_application_info.is_null() {
            let api_version_combined = unsafe { *create_info.p_application_info }.api_version;
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
            enabled_extensions,
            enabled_layers,
        })
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceLayerProperties.html>
    pub fn layer_avilable(entry: &Entry, layer_name: CString) -> VkResult<bool> {
        let layer_properties = unsafe { entry.enumerate_instance_layer_properties()? };
        let is_available = layer_properties.iter().any(|layer_prop| {
            let installed_layer_name =
                unsafe { CStr::from_ptr(layer_prop.layer_name.as_ptr()) }.to_owned();
            installed_layer_name == layer_name
        });
        Ok(is_available)
    }

    /// Returns any of the provided `extension_names` that are unsupported by this device.
    ///
    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceExtensionProperties.html>
    pub fn any_unsupported_extensions(
        entry: &Entry,
        layer_name: Option<&CStr>,
        mut extension_names: Vec<CString>,
    ) -> VkResult<Vec<CString>> {
        let extension_properties =
            unsafe { entry.enumerate_instance_extension_properties(layer_name)? };
        extension_names.retain(|extension_name| {
            !Self::extension_name_is_in_properties_list(
                &extension_properties,
                extension_name.clone(),
            )
        });
        Ok(extension_names)
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceExtensionProperties.html>
    pub fn supports_extension(
        entry: &Entry,
        layer_name: Option<&CStr>,
        extension_name: CString,
    ) -> VkResult<bool> {
        let extension_properties =
            unsafe { entry.enumerate_instance_extension_properties(layer_name)? };
        let is_supported =
            Self::extension_name_is_in_properties_list(&extension_properties, extension_name);
        Ok(is_supported)
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkEnumerateInstanceExtensionProperties.html>
    pub fn extension_name_is_in_properties_list(
        extension_properties: &[vk::ExtensionProperties],
        extension_name: CString,
    ) -> bool {
        extension_properties.iter().any(|props| {
            let supported_extension_name =
                unsafe { CStr::from_ptr(props.extension_name.as_ptr()) }.to_owned();
            supported_extension_name == extension_name
        })
    }

    #[inline]
    pub fn physical_device_features(
        &self,
        physical_device: &PhysicalDevice,
    ) -> PhysicalDeviceFeatures {
        PhysicalDeviceFeatures {
            features_1_0: self.physical_device_features_1_0(physical_device),
            features_1_1: self
                .physical_device_features_1_1(physical_device)
                .unwrap_or_default(),
            features_1_2: self
                .physical_device_features_1_2(physical_device)
                .unwrap_or_default(),
            features_1_3: self
                .physical_device_features_1_3(physical_device)
                .unwrap_or_default(),
        }
    }

    /// Vulkan 1.0 features
    pub fn physical_device_features_1_0(
        &self,
        physical_device: &PhysicalDevice,
    ) -> vk::PhysicalDeviceFeatures {
        unsafe {
            self.inner()
                .get_physical_device_features(physical_device.handle())
        }
    }

    /// Vulkan 1.1 features. If api version < 1.1, these cannot be populated.
    pub fn physical_device_features_1_1(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan11Features> {
        if self.max_api_version < ApiVersion::V1_1 {
            return None;
        }

        let mut features_1_1 = vk::PhysicalDeviceVulkan11Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features_1_1);
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device.handle(), &mut features)
        };

        Some(features_1_1)
    }

    /// Vulkan 1.2 features. If api version < 1.2, these cannot be populated.
    pub fn physical_device_features_1_2(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan12Features> {
        if self.max_api_version < ApiVersion::V1_2 {
            return None;
        }

        let mut features_1_2 = vk::PhysicalDeviceVulkan12Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features_1_2);
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device.handle(), &mut features)
        };

        Some(features_1_2)
    }

    /// Vulkan 1.3 features. If api version < 1.3, these cannot be populated.
    pub fn physical_device_features_1_3(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Option<vk::PhysicalDeviceVulkan13Features> {
        if self.max_api_version < ApiVersion::V1_3 {
            return None;
        }

        let mut features_1_3 = vk::PhysicalDeviceVulkan13Features::default();
        let mut features = vk::PhysicalDeviceFeatures2::default().push_next(&mut features_1_3);
        unsafe {
            self.inner
                .get_physical_device_features2(physical_device.handle(), &mut features)
        };

        Some(features_1_3)
    }

    pub fn enumerate_physical_devices(&self) -> VkResult<Vec<vk::PhysicalDevice>> {
        unsafe { self.inner.enumerate_physical_devices() }
    }

    /// Query the required instance extensions for creating a surface from a display handle.
    ///
    /// This [`RawDisplayHandle`] can typically be acquired from a window, but is usually also
    /// accessible earlier through an "event loop" concept to allow querying required instance
    /// extensions and creation of a compatible Vulkan instance prior to creating a window.
    ///
    /// The returned extensions will include all extension dependencies.
    ///
    /// _Note: this function was copied from [ash](https://github.com/ash-rs/ash) to allow for better
    /// dependency control._
    pub fn required_surface_extensions(
        display_handle: RawDisplayHandle,
    ) -> Result<&'static [&'static CStr], InstanceError> {
        let extensions = match display_handle {
            RawDisplayHandle::Windows(_) => &Self::SURFACE_EXTS_WINDOWS,
            RawDisplayHandle::Wayland(_) => &Self::SURFACE_EXTS_WAYLAND,
            RawDisplayHandle::Xlib(_) => &Self::SURFACE_EXTS_XLIB,
            RawDisplayHandle::Xcb(_) => &Self::SURFACE_EXTS_XCB,
            RawDisplayHandle::Android(_) => &Self::SURFACE_EXTS_ANDROID,
            RawDisplayHandle::AppKit(_) | RawDisplayHandle::UiKit(_) => &Self::SURFACE_EXTS_METAL,
            _ => return Err(InstanceError::UnsupportedRawDisplayHandle),
        };
        Ok(extensions)
    }
    pub const SURFACE_EXTS_WINDOWS: [&'static CStr; 2] = [surface::NAME, win32_surface::NAME];
    pub const SURFACE_EXTS_WAYLAND: [&'static CStr; 2] = [surface::NAME, wayland_surface::NAME];
    pub const SURFACE_EXTS_XLIB: [&'static CStr; 2] = [surface::NAME, xlib_surface::NAME];
    pub const SURFACE_EXTS_XCB: [&'static CStr; 2] = [surface::NAME, xcb_surface::NAME];
    pub const SURFACE_EXTS_ANDROID: [&'static CStr; 2] = [surface::NAME, android_surface::NAME];
    pub const SURFACE_EXTS_METAL: [&'static CStr; 2] = [surface::NAME, metal_surface::NAME];

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

    #[inline]
    pub fn enabled_extensions(&self) -> &Vec<CString> {
        &self.enabled_extensions
    }

    #[inline]
    pub fn enabled_layers(&self) -> &Vec<CString> {
        &self.enabled_layers
    }
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
    UnsupportedRawDisplayHandle,
    ExtensionsNotPresent(Vec<CString>),
    Creation(vk::Result),
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRawDisplayHandle => {
                write!(f, "unsupported display handle. could not determine the required surface extensions for this window system")
            }
            Self::ExtensionsNotPresent(extension_names) => {
                write!(
                    f,
                    "the following extensions were reqested but are not present: {:?}",
                    extension_names
                )
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
            Self::UnsupportedRawDisplayHandle => None,
            Self::ExtensionsNotPresent(_) => None,
            Self::Creation(e) => Some(e),
        }
    }
}

// ~~ Tests ~~

#[test]
fn api_version_ordering() {
    assert!(ApiVersion::V1_1 < ApiVersion::V1_2);
}
