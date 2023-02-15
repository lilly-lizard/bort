use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct DescriptorPool {
    handle: vk::DescriptorPool,
    properties: DescriptorPoolProperties,

    // dependencies
    device: Arc<Device>,
}

impl DescriptorPool {
    pub fn new(device: Arc<Device>, properties: DescriptorPoolProperties) -> anyhow::Result<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_descriptor_pool(&properties.create_info_builder(), ALLOCATION_CALLBACK_NONE)
        }
        .context("creating descriptor set pool")?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::DescriptorPool {
        self.handle
    }

    pub fn properties(&self) -> &DescriptorPoolProperties {
        &self.properties
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_descriptor_pool(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

#[derive(Clone)]
pub struct DescriptorPoolProperties {
    pub create_flags: vk::DescriptorPoolCreateFlags,
    pub max_sets: u32,
    pub pool_sizes: Vec<vk::DescriptorPoolSize>,
}

impl DescriptorPoolProperties {
    pub fn create_info_builder(&self) -> vk::DescriptorPoolCreateInfoBuilder {
        vk::DescriptorPoolCreateInfo::builder()
            .flags(self.create_flags)
            .max_sets(self.max_sets)
            .pool_sizes(self.pool_sizes.as_slice())
    }
}
