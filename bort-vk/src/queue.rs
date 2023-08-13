use crate::{Device, DeviceOwned};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct Queue {
    handle: vk::Queue,
    family_index: u32,
    queue_index: u32,

    // dependencies
    device: Arc<Device>,
}

impl Queue {
    pub fn new(device: Arc<Device>, family_index: u32, queue_index: u32) -> Self {
        let handle = unsafe { device.inner().get_device_queue(family_index, queue_index) };

        Self {
            handle,
            family_index,
            queue_index,
            device,
        }
    }

    pub fn submit(
        &self,
        submit_infos: &[vk::SubmitInfo],
        fence_handle: Option<vk::Fence>,
    ) -> VkResult<()> {
        unsafe {
            self.device.inner().queue_submit(
                self.handle,
                submit_infos,
                fence_handle.unwrap_or_default(),
            )
        }
    }

    // Getters

    pub fn handle(&self) -> vk::Queue {
        self.handle
    }

    pub fn famliy_index(&self) -> u32 {
        self.family_index
    }

    pub fn queue_index(&self) -> u32 {
        self.queue_index
    }
}

impl DeviceOwned for Queue {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}
