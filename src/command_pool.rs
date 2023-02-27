use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct CommandPool {
    handle: vk::CommandPool,

    // dependencies
    device: Arc<Device>,
}

impl CommandPool {
    pub fn new(device: Arc<Device>) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_command_pool(create_info, ALLOCATION_CALLBACK_NONE)?
        };

        Ok(Self { handle, device })
    }
}
