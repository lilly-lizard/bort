use crate::{string_to_c_string_vec, ALLOCATION_CALLBACK_NONE};
use ash::{extensions::ext::DebugUtils, vk, Entry};
use raw_window_handle::RawDisplayHandle;
use std::{
    error,
    ffi::{CString, NulError},
    fmt,
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
}

#[test]
fn api_version_ordering() {
    let ver_1_1 = ApiVersion::new(1, 1);
    let ver_1_2 = ApiVersion::new(1, 2);
    assert!(ver_1_1 < ver_1_2);
}

pub struct Instance {
    inner: ash::Instance,
    api_version: ApiVersion,
}

impl Instance {
    /// No need to specify display extensions or debug validation layer/extension, this function will figure that out for you.
    pub fn new(
        entry: &Entry,
        api_version: ApiVersion,
        app_name: &str,
        display_handle: RawDisplayHandle,
        enable_debug_validation: bool,
        additional_layer_names: impl IntoIterator<Item = String>,
        additional_extension_names: impl IntoIterator<Item = String>,
    ) -> Result<Self, InstanceError> {
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

        let mut layer_names_raw = string_to_c_string_vec(additional_layer_names)
            .map_err(|e| InstanceError::LayerStringConversion(e))?;
        let mut extension_names_raw = string_to_c_string_vec(additional_extension_names)
            .map_err(|e| InstanceError::ExtensionStringConversion(e))?;

        // in this case, error just means no extensions.
        if let Ok(display_extension_names) =
            ash_window::enumerate_required_extensions(display_handle)
        {
            extension_names_raw.extend_from_slice(display_extension_names);
        }

        let validation_layer_name =
            CString::new("VK_LAYER_KHRONOS_validation").expect("no nulls in str");
        if enable_debug_validation {
            layer_names_raw.push(validation_layer_name.as_ptr());
        }
        if enable_debug_validation {
            extension_names_raw.push(DebugUtils::name().as_ptr());
        }

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layer_names_raw)
            .enabled_extension_names(&extension_names_raw);

        let instance = unsafe { entry.create_instance(&create_info, ALLOCATION_CALLBACK_NONE) }
            .map_err(|e| InstanceError::Creation(e))?;

        Ok(Self {
            inner: instance,
            api_version,
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
    AppNameStringConversion(NulError),
    ExtensionStringConversion(NulError),
    LayerStringConversion(NulError),
    Creation(vk::Result),
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::AppNameStringConversion(e) => Some(e),
            Self::ExtensionStringConversion(e) => Some(e),
            Self::LayerStringConversion(e) => Some(e),
            Self::Creation(e) => Some(e),
        }
    }
}
