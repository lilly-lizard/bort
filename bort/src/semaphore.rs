use crate::{Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::prelude::VkResult;
use ash::vk::{self, Handle};
use std::sync::Arc;

pub struct Semaphore {
    handle: vk::Semaphore,

    // dependencies
    device: Arc<Device>,
}

impl Semaphore {
    pub fn new(device: Arc<Device>) -> VkResult<Self> {
        let create_info_builder = vk::SemaphoreCreateInfo::builder();
        Self::new_from_create_info_builder(device, create_info_builder)
    }

    pub fn new_from_create_info_builder(
        device: Arc<Device>,
        create_info_builder: vk::SemaphoreCreateInfoBuilder,
    ) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_semaphore(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self { handle, device })
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::Semaphore {
        self.handle
    }
}

impl DeviceOwned for Semaphore {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_semaphore(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}
