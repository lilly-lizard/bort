use crate::{c_string_to_string, ApiVersion, Instance};
use ash::vk::{self, api_version_major, api_version_minor};
use std::{
    error,
    ffi::{CStr, CString},
    fmt,
    str::Utf8Error,
    sync::Arc,
};

#[derive(Clone)]
pub struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    properties: vk::PhysicalDeviceProperties,
    name: String,

    queue_family_properties: Vec<vk::QueueFamilyProperties>,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    extension_properties: Vec<ExtensionProperties>,

    // dependencies
    instance: Arc<Instance>,
}

impl PhysicalDevice {
    pub fn new(
        instance: Arc<Instance>,
        handle: vk::PhysicalDevice,
    ) -> Result<Self, PhysicalDeviceError> {
        let properties = unsafe { instance.inner().get_physical_device_properties(handle) };
        let name = unsafe { c_string_to_string(properties.device_name.as_ptr()) }
            .map_err(PhysicalDeviceError::NameStringConversion)?;

        let queue_family_properties = unsafe {
            instance
                .inner()
                .get_physical_device_queue_family_properties(handle)
        };

        let memory_properties = unsafe {
            instance
                .inner()
                .get_physical_device_memory_properties(handle)
        };

        let vk_extension_properties = unsafe {
            instance
                .inner()
                .enumerate_device_extension_properties(handle)
        }
        .map_err(PhysicalDeviceError::EnumerateExtensionProperties)?;

        let extension_properties: Vec<ExtensionProperties> = vk_extension_properties
            .into_iter()
            .map(ExtensionProperties::new)
            .collect();

        Ok(Self {
            handle,
            properties,
            name,

            queue_family_properties,
            memory_properties,
            extension_properties,

            instance,
        })
    }

    pub fn supports_min_api_ver(&self, api_version: ApiVersion) -> bool {
        let supported_major = api_version_major(self.properties.api_version);
        let supported_minor = api_version_minor(self.properties.api_version);
        if supported_major < api_version.major {
            return false;
        }
        if supported_minor < api_version.minor {
            return false;
        }
        true
    }

    /// Returns any of the provided `extension_names` that are unsupported by this device.
    pub fn any_unsupported_extensions(&self, mut extension_names: Vec<CString>) -> Vec<CString> {
        extension_names.retain(|extension_name| !self.supports_extension(extension_name.clone()));
        extension_names
    }

    pub fn supports_extension(&self, extension_name: CString) -> bool {
        self.extension_properties
            .iter()
            .any(|props| props.extension_name == extension_name)
    }

    // Getters

    pub fn handle(&self) -> vk::PhysicalDevice {
        self.handle
    }

    pub fn properties(&self) -> &vk::PhysicalDeviceProperties {
        &self.properties
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn queue_family_properties(&self) -> &Vec<vk::QueueFamilyProperties> {
        &self.queue_family_properties
    }

    pub fn memory_properties(&self) -> &vk::PhysicalDeviceMemoryProperties {
        &self.memory_properties
    }

    pub fn extension_properties(&self) -> &Vec<ExtensionProperties> {
        &self.extension_properties
    }

    pub fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }
}

/// Properties of an extension in the loader or a physical device.
#[derive(Clone, Debug)]
pub struct ExtensionProperties {
    pub extension_name: CString,
    pub spec_version: u32,
}

impl ExtensionProperties {
    fn new(value: vk::ExtensionProperties) -> Self {
        let c_str = unsafe { CStr::from_ptr(value.extension_name.as_ptr()) };
        Self {
            extension_name: c_str.to_owned(),
            spec_version: value.spec_version,
        }
    }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct PhysicalDeviceFeatures {
    pub features_1_0: vk::PhysicalDeviceFeatures,
    pub features_1_1: vk::PhysicalDeviceVulkan11Features,
    pub features_1_2: vk::PhysicalDeviceVulkan12Features,
    pub features_1_3: vk::PhysicalDeviceVulkan13Features,
}

// ~~ Errors ~~

#[derive(Debug, Clone)]
pub enum PhysicalDeviceError {
    NameStringConversion(Utf8Error),
    EnumerateExtensionProperties(vk::Result),
}

impl fmt::Display for PhysicalDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameStringConversion(e) => {
                write!(f, "failed to convert device name to string: {}", e)
            }
            Self::EnumerateExtensionProperties(e) => write!(
                f,
                "call to vkEnumerateInstanceExtensionProperties failed: {}",
                e
            ),
        }
    }
}

impl error::Error for PhysicalDeviceError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::NameStringConversion(e) => Some(e),
            Self::EnumerateExtensionProperties(e) => Some(e),
        }
    }
}
