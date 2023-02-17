use crate::{
    device::Device, memory::ALLOCATION_CALLBACK_NONE, pipeline_access::PipelineAccess,
    pipeline_layout::PipelineLayout,
};
use ash::vk;
use std::sync::Arc;

pub struct ComputePipeline {
    handle: vk::Pipeline,
    properties: ComputePipelineProperties,

    // dependencies
    pipeline_layout: Arc<PipelineLayout>,
    // note: we don't need to store references to `ShaderModule` or `PipelineCache` as per https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#fundamentals-objectmodel-lifetime
}

impl ComputePipeline {
    pub fn properties(&self) -> &ComputePipelineProperties {
        &self.properties
    }
}

impl PipelineAccess for ComputePipeline {
    fn handle(&self) -> vk::Pipeline {
        self.handle
    }

    fn pipeline_layout(&self) -> &Arc<PipelineLayout> {
        &self.pipeline_layout
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.pipeline_layout.device()
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device()
                .inner()
                .destroy_pipeline(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

#[derive(Clone)]
pub struct ComputePipelineProperties {
    pub create_flags: vk::PipelineCreateFlags,
}

impl ComputePipelineProperties {
    pub fn create_info_builder(&self) -> vk::ComputePipelineCreateInfoBuilder {
        vk::ComputePipelineCreateInfo::builder().flags(self.create_flags)
    }
}
