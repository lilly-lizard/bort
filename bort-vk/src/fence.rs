use std::sync::Arc;

use crate::{Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};

pub struct Fence {
    handle: vk::Fence,

    // dependencies
    device: Arc<Device>,
}

impl Fence {
    pub fn new_signalled(device: Arc<Device>) -> VkResult<Self> {
        let create_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        unsafe { Self::new_from_create_info(device, create_info) }
    }

    pub fn new_unsignalled(device: Arc<Device>) -> VkResult<Self> {
        let create_info = vk::FenceCreateInfo::builder();
        unsafe { Self::new_from_create_info(device, create_info) }
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        device: Arc<Device>,
        create_info_builder: vk::FenceCreateInfoBuilder,
    ) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_fence(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self { handle, device })
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkWaitForFences.html>
    pub fn wait(&self, timeout_nanoseconds: u64) -> VkResult<()> {
        unsafe {
            self.device
                .inner()
                .wait_for_fences(&[self.handle], true, timeout_nanoseconds)
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkResetFences.html>
    pub fn reset(&self) -> VkResult<()> {
        unsafe { self.device.inner().reset_fences(&[self.handle]) }
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::Fence {
        self.handle
    }
}

impl DeviceOwned for Fence {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_fence(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}
