use crate::{
    common::string_to_c_string_vec,
    instance::{ApiVersion, Instance},
    memory::ALLOCATION_CALLBACK_NONE,
    physical_device::PhysicalDevice,
};
use ash::vk;
use std::{error, ffi::NulError, fmt, sync::Arc};

pub struct Device {
    inner: ash::Device,

    // dependencies
    physical_device: Arc<PhysicalDevice>,
}

impl Device {
    /// `features_1_1` and `features_1_2` may be ignored depending on the `instance` api version.
    pub fn new<'a>(
        physical_device: Arc<PhysicalDevice>,
        queue_create_infos: &'a [vk::DeviceQueueCreateInfo],
        features_1_0: vk::PhysicalDeviceFeatures,
        mut features_1_1: vk::PhysicalDeviceVulkan11Features,
        mut features_1_2: vk::PhysicalDeviceVulkan12Features,
        extension_names: impl IntoIterator<Item = String>,
        layer_names: impl IntoIterator<Item = String>,
    ) -> Result<Self, DeviceError> {
        let instance = physical_device.instance();

        let extension_names_raw = string_to_c_string_vec(extension_names)
            .map_err(|e| DeviceError::ExtensionStringConversion(e))?;
        let layer_names_raw = string_to_c_string_vec(layer_names)
            .map_err(|e| DeviceError::LayerStringConversion(e))?;

        let mut device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(queue_create_infos)
            .enabled_extension_names(extension_names_raw.as_slice())
            .enabled_layer_names(layer_names_raw.as_slice());

        let mut features_2 = vk::PhysicalDeviceFeatures2::builder();
        if instance.api_version() <= ApiVersion::new(1, 0) {
            device_create_info = device_create_info.enabled_features(&features_1_0);
        } else {
            features_2 = features_2.features(features_1_0);
            device_create_info = device_create_info.push_next(&mut features_2);

            if instance.api_version() >= ApiVersion::new(1, 1) {
                device_create_info = device_create_info.push_next(&mut features_1_1)
            }

            if instance.api_version() >= ApiVersion::new(1, 2) {
                device_create_info = device_create_info.push_next(&mut features_1_2);
            }
        }

        let inner = unsafe {
            instance.inner().create_device(
                physical_device.handle(),
                &device_create_info,
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .map_err(|vk_res| DeviceError::Creation(vk_res))?;

        Ok(Self {
            inner,
            physical_device,
        })
    }

    pub fn wait_idle(&self) -> Result<(), DeviceError> {
        let res = unsafe { self.inner.device_wait_idle() };
        res.map_err(|vk_res| DeviceError::WaitIdle(vk_res))
    }

    // Getters

    pub fn inner(&self) -> &ash::Device {
        &self.inner
    }

    pub fn physical_device(&self) -> &Arc<PhysicalDevice> {
        &self.physical_device
    }

    pub fn instance(&self) -> &Arc<Instance> {
        self.physical_device.instance()
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.wait_idle().expect("vkDeviceWaitIdle");
        unsafe {
            self.inner.destroy_device(ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Debug, Clone)]
pub enum DeviceError {
    ExtensionStringConversion(NulError),
    LayerStringConversion(NulError),
    Creation(vk::Result),
    WaitIdle(vk::Result),
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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
            Self::Creation(e) => write!(f, "failed to create device: {}", e),
            Self::WaitIdle(e) => write!(f, "vkDeviceWaitIdle call failed: {}", e),
        }
    }
}

impl error::Error for DeviceError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::ExtensionStringConversion(e) => Some(e),
            Self::LayerStringConversion(e) => Some(e),
            Self::Creation(e) => Some(e),
            Self::WaitIdle(e) => Some(e),
        }
    }
}
