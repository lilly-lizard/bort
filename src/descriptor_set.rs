use crate::{descriptor_pool::DescriptorPool, device::Device};
use ash::vk;
use std::sync::Arc;

/// Note: no destructor needed. Just drop pool.
pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    properties: DescriptorSetProperties,

    // dependencies
    descriptor_pool: Arc<DescriptorPool>,
}

impl DescriptorSet {
    // Getters

    pub fn handle(&self) -> vk::DescriptorSet {
        self.handle
    }

    pub fn properties(&self) -> &DescriptorSetProperties {
        &self.properties
    }

    pub fn descriptor_pool(&self) -> &Arc<DescriptorPool> {
        &self.descriptor_pool
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        self.descriptor_pool.device()
    }
}

#[derive(Clone)]
pub struct DescriptorSetProperties {}
