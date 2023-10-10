use crate::{DescriptorPool, DescriptorSetLayout, Device, DeviceOwned};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

/// Note: no destructor needed. Just drop pool.
pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    layout: Arc<DescriptorSetLayout>,

    // dependencies
    descriptor_pool: Arc<DescriptorPool>,
}

impl DescriptorSet {
    pub fn new(
        descriptor_pool: Arc<DescriptorPool>,
        layout: Arc<DescriptorSetLayout>,
    ) -> VkResult<Self> {
        descriptor_pool.allocate_descriptor_set(layout)
    }

    /// Safetey: make sure `handle` was allocated from `descriptor_pool` using `layout`.
    pub(crate) unsafe fn from_handle(
        handle: vk::DescriptorSet,
        layout: Arc<DescriptorSetLayout>,
        descriptor_pool: Arc<DescriptorPool>,
    ) -> Self {
        Self {
            handle,
            layout,
            descriptor_pool,
        }
    }

    // Getters

    pub fn handle(&self) -> vk::DescriptorSet {
        self.handle
    }

    pub fn layout(&self) -> &Arc<DescriptorSetLayout> {
        &self.layout
    }

    #[inline]
    pub fn descriptor_pool(&self) -> &Arc<DescriptorPool> {
        &self.descriptor_pool
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            let _res = self
                .device()
                .inner()
                .free_descriptor_sets(self.descriptor_pool.handle(), &[self.handle]);
        }
    }
}

impl DeviceOwned for DescriptorSet {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.descriptor_pool.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}
