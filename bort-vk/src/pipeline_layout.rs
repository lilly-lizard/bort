use crate::{DescriptorSetLayout, Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct PipelineLayout {
    handle: vk::PipelineLayout,
    properties: PipelineLayoutProperties,

    // dependencies
    device: Arc<Device>,
}

impl PipelineLayout {
    pub fn new(device: Arc<Device>, properties: PipelineLayoutProperties) -> VkResult<Self> {
        let mut vk_set_layouts_storage: Vec<vk::DescriptorSetLayout> = Vec::new();
        let create_info = properties.create_info(&mut vk_set_layouts_storage);
        let handle = unsafe {
            device
                .inner()
                .create_pipeline_layout(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::PipelineLayout {
        self.handle
    }

    pub fn properties(&self) -> &PipelineLayoutProperties {
        &self.properties
    }
}

impl DeviceOwned for PipelineLayout {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_pipeline_layout(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Default, Clone)]
pub struct PipelineLayoutProperties {
    pub flags: vk::PipelineLayoutCreateFlags,
    pub set_layouts: Vec<Arc<DescriptorSetLayout>>,
    pub push_constant_ranges: Vec<vk::PushConstantRange>,
}

impl PipelineLayoutProperties {
    pub fn new(
        set_layouts: Vec<Arc<DescriptorSetLayout>>,
        push_constant_ranges: Vec<vk::PushConstantRange>,
    ) -> Self {
        Self {
            flags: vk::PipelineLayoutCreateFlags::empty(),
            set_layouts,
            push_constant_ranges,
        }
    }

    /// Clears and populates `vk_set_layouts_storage`
    /// with data pointed to by the returned create info. `vk_set_layouts_storage`
    /// must outlive the returned create info.
    pub fn create_info<'a>(
        &'a self,
        vk_set_layouts_storage: &'a mut Vec<vk::DescriptorSetLayout>,
    ) -> vk::PipelineLayoutCreateInfo<'a> {
        *vk_set_layouts_storage = self.vk_set_layouts();
        vk::PipelineLayoutCreateInfo::default()
            .flags(self.flags)
            .set_layouts(vk_set_layouts_storage)
            .push_constant_ranges(&self.push_constant_ranges)
    }

    pub fn vk_set_layouts(&self) -> Vec<vk::DescriptorSetLayout> {
        self.set_layouts
            .iter()
            .map(|layout| layout.handle())
            .collect()
    }
}
