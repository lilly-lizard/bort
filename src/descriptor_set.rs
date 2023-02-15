use crate::{
    descriptor_layout::DescriptorSetLayout, descriptor_pool::DescriptorPool, device::Device,
};
use ash::vk;
use std::sync::Arc;

/// Note: no destructor needed. Just drop pool.
pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    layout: Arc<DescriptorSetLayout>,

    // dependencies
    descriptor_pool: Arc<DescriptorPool>,
}

impl DescriptorSet {
    /// Safetey: make sure `handle` was allocated from `descriptor_pool` using `layout`.
    pub unsafe fn from_handle(
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

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        self.descriptor_pool.device()
    }
}