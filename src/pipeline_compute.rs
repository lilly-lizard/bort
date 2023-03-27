use crate::{
    Device, DeviceOwned, PipelineAccess, PipelineCache, PipelineLayout, ShaderStage,
    ALLOCATION_CALLBACK_NONE,
};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct ComputePipeline {
    handle: vk::Pipeline,
    properties: ComputePipelineProperties,

    // dependencies
    pipeline_layout: Arc<PipelineLayout>,
    // note: we don't need to store references to `ShaderModule` or `PipelineCache` as per https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#fundamentals-objectmodel-lifetime
}

impl ComputePipeline {
    pub fn new(
        pipeline_layout: Arc<PipelineLayout>,
        properties: ComputePipelineProperties,
        shader_stage: &ShaderStage,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Self> {
        let shader_stage_vk = shader_stage.create_info_builder().build();

        let create_info_builder = properties
            .create_info_builder()
            .stage(shader_stage_vk)
            .layout(pipeline_layout.handle());

        let cache_handle = if let Some(pipeline_cache) = pipeline_cache {
            pipeline_cache.handle()
        } else {
            vk::PipelineCache::null()
        };

        let handles = unsafe {
            pipeline_layout.device().inner().create_compute_pipelines(
                cache_handle,
                &[create_info_builder.build()],
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .map_err(|(_pipelines, err_code)| err_code)?;
        let handle = handles[0];

        Ok(Self {
            handle,
            properties,
            pipeline_layout,
        })
    }

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
}

impl DeviceOwned for ComputePipeline {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.pipeline_layout.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
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
    pub flags: vk::PipelineCreateFlags,
}

impl ComputePipelineProperties {
    pub fn create_info_builder(&self) -> vk::ComputePipelineCreateInfoBuilder {
        vk::ComputePipelineCreateInfo::builder().flags(self.flags)
    }
}
