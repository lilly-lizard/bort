use crate::{
    descriptor_layout::DescriptorSetLayout, descriptor_set::DescriptorSet, device::Device,
    memory::ALLOCATION_CALLBACK_NONE,
};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct DescriptorPool {
    handle: vk::DescriptorPool,
    properties: DescriptorPoolProperties,

    // dependencies
    device: Arc<Device>,
}

impl DescriptorPool {
    pub fn new(device: Arc<Device>, properties: DescriptorPoolProperties) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_descriptor_pool(&properties.create_info_builder(), ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    pub fn allocate_descriptor_set(
        self: &Arc<Self>,
        layout: Arc<DescriptorSetLayout>,
    ) -> VkResult<DescriptorSet> {
        let layout_handles = [layout.handle()];
        let create_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.handle)
            .set_layouts(&layout_handles);

        let descriptor_set_handle =
            unsafe { self.device().inner().allocate_descriptor_sets(&create_info) }?[0];

        Ok(unsafe { DescriptorSet::from_handle(descriptor_set_handle, layout, self.clone()) })
    }

    pub fn allocate_descriptor_sets(
        self: &Arc<Self>,
        layouts: Vec<Arc<DescriptorSetLayout>>,
    ) -> VkResult<Vec<DescriptorSet>> {
        let layout_handles = layouts.iter().map(|l| l.handle()).collect::<Vec<_>>();
        let create_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.handle)
            .set_layouts(layout_handles.as_slice());

        let descriptor_set_handles =
            unsafe { self.device().inner().allocate_descriptor_sets(&create_info) }?;

        let mut descriptor_sets = Vec::<DescriptorSet>::new();
        for i in 0..layouts.len() {
            let descriptor_set = unsafe {
                DescriptorSet::from_handle(
                    descriptor_set_handles[i],
                    layouts[i].clone(),
                    self.clone(),
                )
            };
            descriptor_sets.push(descriptor_set);
        }

        Ok(descriptor_sets)
    }

    // Getters

    pub fn handle(&self) -> vk::DescriptorPool {
        self.handle
    }

    pub fn properties(&self) -> &DescriptorPoolProperties {
        &self.properties
    }

    #[inline]
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
