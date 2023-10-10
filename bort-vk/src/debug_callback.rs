use crate::{Instance, ALLOCATION_CALLBACK_NONE};
use ash::{extensions::ext::DebugUtils, prelude::VkResult, vk};
use std::sync::Arc;

pub struct DebugCallback {
    handle: vk::DebugUtilsMessengerEXT,
    debug_utils_loader: DebugUtils,
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
        let create_info_builder = properties.create_info_builder(debug_callback);

        let debug_utils_loader = DebugUtils::new(instance.entry(), instance.inner());

        let handle = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            debug_utils_loader,
            properties,

            instance,
        })
    }

    pub unsafe fn new_from_create_info(
        instance: Arc<Instance>,
        create_info_builder: vk::DebugUtilsMessengerCreateInfoEXTBuilder,
    ) -> VkResult<Self> {
        let debug_utils_loader = DebugUtils::new(instance.entry(), instance.inner());

        let handle = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        let properties = DebugCallbackProperties::from_create_info_builder(&create_info_builder);

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
    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::DebugUtilsMessengerCreateInfoEXTBuilder<'a>,
    ) -> vk::DebugUtilsMessengerCreateInfoEXTBuilder {
        builder
            .message_severity(self.message_severity)
            .message_type(self.message_type)
    }

    pub fn create_info_builder(
        &self,
        debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
    ) -> vk::DebugUtilsMessengerCreateInfoEXTBuilder {
        let builder =
            vk::DebugUtilsMessengerCreateInfoEXT::builder().pfn_user_callback(debug_callback);
        self.write_create_info_builder(builder)
    }

    pub fn from_create_info_builder(value: &vk::DebugUtilsMessengerCreateInfoEXTBuilder) -> Self {
        Self {
            message_severity: value.message_severity,
            message_type: value.message_type,
        }
    }
}
