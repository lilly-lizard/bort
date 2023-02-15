use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::vk;
use std::sync::Arc;

pub struct Pipeline {
    handle: vk::Pipeline,
    properties: PipelineProperties,

    pipeline_layout_handle: vk::PipelineLayout,
    pipeline_layout_properties: PipelineLayoutProperties,

    // dependencies
    device: Arc<Device>,
}

impl Pipeline {
    // Getters

    pub fn handle(&self) -> vk::Pipeline {
        self.handle
    }

    pub fn properties(&self) -> &PipelineProperties {
        &self.properties
    }

    pub fn pipeline_layout_handle(&self) -> vk::PipelineLayout {
        self.pipeline_layout_handle
    }

    pub fn pipeline_layout_properties(&self) -> &PipelineLayoutProperties {
        &self.pipeline_layout_properties
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_pipeline(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

pub struct PipelineLayoutProperties {
    pub create_flags: vk::PipelineLayoutCreateFlags,
}

#[derive(Clone)]
pub struct GraphicsPipelineProperties {
    pub create_flags: vk::PipelineCreateFlags,
}

#[derive(Clone)]
pub struct ComputePipelineProperties {
    pub create_flags: vk::PipelineCreateFlags,
}

#[derive(Clone)]
pub enum PipelineProperties {
    Compute(ComputePipelineProperties),
    Graphics(GraphicsPipelineProperties),
}
