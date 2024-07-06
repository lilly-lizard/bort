use crate::{Instance, ALLOCATION_CALLBACK_NONE};
use ash::{ext::debug_utils, prelude::VkResult, vk};
use std::sync::Arc;

pub struct DebugCallback {
    handle: vk::DebugUtilsMessengerEXT,
    debug_utils_loader: debug_utils::Instance,
    properties: DebugCallbackProperties,

    // dependencies
    instance: Arc<Instance>,
}

impl DebugCallback {
    pub fn new(
        instance: Arc<Instance>,
        debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
        properties: DebugCallbackProperties,
    ) -> VkResult<Self> {
        let create_info = properties.create_info(debug_callback);
        let debug_utils_loader = debug_utils::Instance::new(instance.entry(), &instance.inner());
        let handle = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;
        Ok(Self {
            handle,
            debug_utils_loader,
            properties,

            instance,
        })
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        instance: Arc<Instance>,
        create_info: vk::DebugUtilsMessengerCreateInfoEXT,
    ) -> VkResult<Self> {
        let debug_utils_loader = debug_utils::Instance::new(instance.entry(), &instance.inner());
        let handle = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;
        let properties = DebugCallbackProperties::from_create_info(&create_info);
        Ok(Self {
            handle,
            debug_utils_loader,
            properties,

            instance,
        })
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> &vk::DebugUtilsMessengerEXT {
        &self.handle
    }

    #[inline]
    pub fn properties(&self) -> DebugCallbackProperties {
        self.properties
    }

    #[inline]
    pub fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }
}

impl Drop for DebugCallback {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

// Properties

#[derive(Clone, Copy, Debug)]
pub struct DebugCallbackProperties {
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
}

impl Default for DebugCallbackProperties {
    fn default() -> Self {
        Self {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        }
    }
}

impl DebugCallbackProperties {
    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::DebugUtilsMessengerCreateInfoEXT<'a>,
    ) -> vk::DebugUtilsMessengerCreateInfoEXT {
        create_info
            .message_severity(self.message_severity)
            .message_type(self.message_type)
    }

    pub fn create_info(
        &self,
        debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
    ) -> vk::DebugUtilsMessengerCreateInfoEXT {
        let create_info =
            vk::DebugUtilsMessengerCreateInfoEXT::default().pfn_user_callback(debug_callback);
        self.write_create_info(create_info)
    }

    pub fn from_create_info(value: &vk::DebugUtilsMessengerCreateInfoEXT) -> Self {
        Self {
            message_severity: value.message_severity,
            message_type: value.message_type,
        }
    }
}
