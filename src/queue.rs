use crate::device::Device;
use ash::vk;
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

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}
