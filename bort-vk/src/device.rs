use crate::{
    string_to_c_string_vec, ApiVersion, DebugCallback, Instance, PhysicalDevice, Queue,
    ALLOCATION_CALLBACK_NONE,
};
use ash::vk::{self, ExtendsDeviceCreateInfo};
use std::{error, ffi::NulError, fmt, sync::Arc};

pub trait DeviceOwned {
    fn device(&self) -> &Arc<Device>;
    fn handle_raw(&self) -> u64;
}

pub struct Device {
    inner: ash::Device,
    debug_callback_ref: Option<Arc<DebugCallback>>,

    // dependencies
    physical_device: Arc<PhysicalDevice>,
}

impl Device {
    /// `features_1_1` and `features_1_2` might get ignored depending on the `instance` api version.
    pub fn new<'a>(
        physical_device: Arc<PhysicalDevice>,
        queue_create_infos: &'a [vk::DeviceQueueCreateInfo],
        features_1_0: vk::PhysicalDeviceFeatures,
        features_1_1: vk::PhysicalDeviceVulkan11Features,
        features_1_2: vk::PhysicalDeviceVulkan12Features,
        extension_names: impl IntoIterator<Item = String>,
        layer_names: impl IntoIterator<Item = String>,
        debug_callback_ref: Option<Arc<DebugCallback>>,
    ) -> Result<Self, DeviceError> {
        unsafe {
            Self::new_with_p_next_chain(
                physical_device,
                queue_create_infos,
                features_1_0,
                features_1_1,
                features_1_2,
                extension_names,
                layer_names,
                debug_callback_ref,
                Vec::<vk::PhysicalDeviceFeatures2>::new(),
            )
        }
    }

    /// `features_1_1` and `features_1_2` might get ignored depending on the `instance` api version.
    ///
    /// Note that each member of `p_next_structs` can only be one type (known at compile time) due
    /// to the way `ash::DeviceCreateInfoBuilder::push_next` takes its input. Just treat it like an
    /// `Option` (either 0 or 1 element) and create your own p_next chain in the one element you
    /// pass to this.
    ///
    /// Safety:
    /// No busted pointers in the last element of `p_next_structs`.
    pub unsafe fn new_with_p_next_chain<'a>(
        physical_device: Arc<PhysicalDevice>,
        queue_create_infos: &'a [vk::DeviceQueueCreateInfo],
        features_1_0: vk::PhysicalDeviceFeatures,
        mut features_1_1: vk::PhysicalDeviceVulkan11Features,
        mut features_1_2: vk::PhysicalDeviceVulkan12Features,
        extension_names: impl IntoIterator<Item = String>,
        layer_names: impl IntoIterator<Item = String>,
        debug_callback_ref: Option<Arc<DebugCallback>>,
        mut p_next_structs: Vec<impl ExtendsDeviceCreateInfo>,
    ) -> Result<Self, DeviceError> {
        let instance = physical_device.instance();

        let extension_name_cstrings = string_to_c_string_vec(extension_names)
            .map_err(|e| DeviceError::ExtensionStringConversion(e))?;
        let layer_name_cstrings = string_to_c_string_vec(layer_names)
            .map_err(|e| DeviceError::LayerStringConversion(e))?;

        let extension_name_ptrs = extension_name_cstrings
            .iter()
            .map(|cstring| cstring.as_ptr())
            .collect::<Vec<_>>();
        let layer_name_ptrs = layer_name_cstrings
            .iter()
            .map(|cstring| cstring.as_ptr())
            .collect::<Vec<_>>();

        #[allow(deprecated)] // backward compatability
        let mut device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(queue_create_infos)
            .enabled_extension_names(&extension_name_ptrs)
            .enabled_layer_names(&layer_name_ptrs);

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

        for p_next_struct in &mut p_next_structs {
            device_create_info = device_create_info.push_next(p_next_struct);
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
            debug_callback_ref,
            physical_device,
        })
    }

    pub unsafe fn new_from_create_info(
        physical_device: Arc<PhysicalDevice>,
        create_info_builder: vk::DeviceCreateInfoBuilder,
        debug_callback_ref: Option<Arc<DebugCallback>>,
    ) -> Result<Self, DeviceError> {
        let inner = unsafe {
            physical_device.instance().inner().create_device(
                physical_device.handle(),
                &create_info_builder,
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .map_err(|vk_res| DeviceError::Creation(vk_res))?;

        Ok(Self {
            inner,
            debug_callback_ref,
            physical_device,
        })
    }

    /// Store a reference to a debug callback. The means that the debug callback won't be dropped
    /// (and destroyed) until this device is! Handy to make sure that you still get validation
    /// while device resources are being dropped/destroyed.
    pub fn set_debug_callback_ref(&mut self, debug_callback_ref: Option<Arc<DebugCallback>>) {
        self.debug_callback_ref = debug_callback_ref;
    }

    pub fn wait_idle(&self) -> Result<(), DeviceError> {
        let res = unsafe { self.inner.device_wait_idle() };
        res.map_err(|vk_res| DeviceError::WaitIdle(vk_res))
    }

    pub fn queue_wait_idle(&self, queue: &Queue) -> Result<(), DeviceError> {
        let res = unsafe { self.inner.queue_wait_idle(queue.handle()) };
        res.map_err(|vk_res| DeviceError::WaitIdle(vk_res))
    }

    // Getters

    /// Access the `ash::Device` struct that `self` contains. Allows you to access vulkan device
    /// functions.
    #[inline]
    pub fn inner(&self) -> &ash::Device {
        &self.inner
    }

    #[inline]
    pub fn physical_device(&self) -> &Arc<PhysicalDevice> {
        &self.physical_device
    }

    #[inline]
    pub fn instance(&self) -> &Arc<Instance> {
        self.physical_device.instance()
    }

    #[inline]
    pub fn debug_callback_ref(&self) -> &Option<Arc<DebugCallback>> {
        &self.debug_callback_ref
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
