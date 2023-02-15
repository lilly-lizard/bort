use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::vk;
use std::sync::Arc;

pub trait Pipeline {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout_handle(&self) -> vk::PipelineLayout;
    fn pipeline_layout_properties(&self) -> &PipelineLayoutProperties;
    fn device(&self) -> &Arc<Device>;
}

#[derive(Clone)]
pub struct PipelineLayoutProperties {
    pub create_flags: vk::PipelineLayoutCreateFlags,
}

// Graphics Pipeline

pub struct GraphicsPipeline {
    handle: vk::Pipeline,
    properties: GraphicsPipelineProperties,

    pipeline_layout_handle: vk::PipelineLayout,
    pipeline_layout_properties: PipelineLayoutProperties,

    // dependencies
    device: Arc<Device>,
}

impl GraphicsPipeline {
    pub fn properties(&self) -> &GraphicsPipelineProperties {
        &self.properties
    }
}

impl Pipeline for GraphicsPipeline {
    fn handle(&self) -> vk::Pipeline {
        self.handle
    }

    fn pipeline_layout_handle(&self) -> vk::PipelineLayout {
        self.pipeline_layout_handle
    }

    fn pipeline_layout_properties(&self) -> &PipelineLayoutProperties {
        &self.pipeline_layout_properties
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_pipeline(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

#[derive(Clone)]
pub struct GraphicsPipelineProperties {
    pub create_flags: vk::PipelineCreateFlags,
}

// Compute Pipeline

pub struct ComputePipeline {
    handle: vk::Pipeline,
    properties: ComputePipelineProperties,

    pipeline_layout_handle: vk::PipelineLayout,
    pipeline_layout_properties: PipelineLayoutProperties,

    // dependencies
    device: Arc<Device>,
}

impl ComputePipeline {
    pub fn properties(&self) -> &ComputePipelineProperties {
        &self.properties
    }
}

impl Pipeline for ComputePipeline {
    fn handle(&self) -> vk::Pipeline {
        self.handle
    }

    fn pipeline_layout_handle(&self) -> vk::PipelineLayout {
        self.pipeline_layout_handle
    }

    fn pipeline_layout_properties(&self) -> &PipelineLayoutProperties {
        &self.pipeline_layout_properties
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_pipeline(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

#[derive(Clone)]
pub struct ComputePipelineProperties {
    pub create_flags: vk::PipelineCreateFlags,
}
