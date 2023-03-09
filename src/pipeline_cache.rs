use crate::{Device, ALLOCATION_CALLBACK_NONE};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct PipelineCache {
    handle: vk::PipelineCache,

    // dependencies
    device: Arc<Device>,
}

impl PipelineCache {
    pub fn new(
        device: Arc<Device>,
        create_info: vk::PipelineCacheCreateInfoBuilder,
    ) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_pipeline_cache(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self { handle, device })
    }

    // Getters

    pub fn handle(&self) -> vk::PipelineCache {
        self.handle
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for PipelineCache {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_pipeline_cache(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}
