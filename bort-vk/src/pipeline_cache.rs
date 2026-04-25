use crate::{Device, DeviceOwned, Refc, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};

pub struct PipelineCache {
    handle: vk::PipelineCache,

    // dependencies
    device: Refc<Device>,
}

impl PipelineCache {
    pub fn new(device: Refc<Device>, create_info: vk::PipelineCacheCreateInfo) -> VkResult<Self> {
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
}

impl DeviceOwned for PipelineCache {
    #[inline]
    fn device(&self) -> &Refc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
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
