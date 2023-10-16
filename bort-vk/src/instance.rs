use crate::{string_to_c_string_vec, ALLOCATION_CALLBACK_NONE};
use ash::{
    vk::{self, make_api_version},
    Entry,
};
use raw_window_handle::RawDisplayHandle;
use std::{
    error,
    ffi::{CString, NulError},
    fmt,
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ApiVersion {
    pub major: u32,
    pub minor: u32,
}

impl ApiVersion {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn as_vk_uint(&self) -> u32 {
        make_api_version(0, self.major, self.minor, 0)
    }
}

pub struct Instance {
    inner: ash::Instance,
    api_version: ApiVersion,

    // dependencies
    entry: Arc<Entry>,
}

impl Instance {
    pub fn new<S>(
        entry: Arc<Entry>,
        api_version: ApiVersion,
        app_name: &str,
        display_handle: RawDisplayHandle,
        layer_names: impl IntoIterator<Item = S>,
        extension_names: impl IntoIterator<Item = S>,
    ) -> Result<Self, InstanceError>
    where
        S: Into<Vec<u8>>,
    {
        let app_name =
            CString::new(app_name).map_err(|e| InstanceError::AppNameStringConversion(e))?;
        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&app_name)
            .engine_version(0)
            .api_version(vk::make_api_version(
                0,
                api_version.major,
                api_version.minor,
                0,
            ));

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

        let display_extension_names = ash_window::enumerate_required_extensions(display_handle)
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
            api_version,
        })
    }

    pub unsafe fn new_from_create_info(
        entry: Arc<Entry>,
        create_info_builder: vk::InstanceCreateInfoBuilder,
    ) -> Result<Self, InstanceError> {
        let instance_inner =
            unsafe { entry.create_instance(&create_info_builder, ALLOCATION_CALLBACK_NONE) }
                .map_err(|e| InstanceError::Creation(e))?;

        let api_version = if create_info_builder.p_application_info != std::ptr::null() {
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
            api_version,
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
        if self.api_version < ApiVersion::new(1, 1) {
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
        if self.api_version < ApiVersion::new(1, 2) {
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

    // Getters

    /// Access the `ash::Instance` struct that `self` contains. Allows you to access vulkan instance
    /// functions.
    #[inline]
    pub fn inner(&self) -> &ash::Instance {
        &self.inner
    }

    #[inline]
    pub fn api_version(&self) -> ApiVersion {
        self.api_version
    }

    #[inline]
    pub fn entry(&self) -> &Arc<Entry> {
        &self.entry
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.inner.destroy_instance(ALLOCATION_CALLBACK_NONE);
        }
    }
}

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

// Tests

#[test]
fn api_version_ordering() {
    let ver_1_1 = ApiVersion::new(1, 1);
    let ver_1_2 = ApiVersion::new(1, 2);
    assert!(ver_1_1 < ver_1_2);
}
