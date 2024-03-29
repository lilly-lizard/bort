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
    pub fn new(device: Arc<Device>, mut properties: PipelineLayoutProperties) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_pipeline_layout(&properties.create_info_builder(), ALLOCATION_CALLBACK_NONE)
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
    // because these need to be stored for the lifetime duration of self
    set_layouts_vk: Vec<vk::DescriptorSetLayout>,
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
            set_layouts_vk: Vec::new(),
        }
    }

    pub fn create_info_builder(&mut self) -> vk::PipelineLayoutCreateInfoBuilder {
        self.set_layouts_vk = self
            .set_layouts
            .iter()
            .map(|layout| layout.handle())
            .collect();

        vk::PipelineLayoutCreateInfo::builder()
            .flags(self.flags)
            .set_layouts(&self.set_layouts_vk)
            .push_constant_ranges(&self.push_constant_ranges)
    }
}
