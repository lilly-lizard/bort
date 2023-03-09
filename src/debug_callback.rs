use crate::{Instance, ALLOCATION_CALLBACK_NONE};
use ash::{extensions::ext::DebugUtils, prelude::VkResult, vk, Entry};
use std::sync::Arc;

pub struct DebugCallback {
    handle: vk::DebugUtilsMessengerEXT,
    debug_utils_loader: DebugUtils,

    // dependencies
    instance: Arc<Instance>,
}

impl DebugCallback {
    pub fn new(
        entry: &Entry,
        instance: Arc<Instance>,
        debug_callback: vk::PFN_vkDebugUtilsMessengerCallbackEXT,
    ) -> VkResult<Self> {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(debug_callback);

        let debug_utils_loader = DebugUtils::new(entry, instance.inner());
        let handle = unsafe {
            debug_utils_loader.create_debug_utils_messenger(&debug_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            debug_utils_loader,

            instance,
        })
    }

    // Getters

    pub fn handle(&self) -> &vk::DebugUtilsMessengerEXT {
        &self.handle
    }

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
