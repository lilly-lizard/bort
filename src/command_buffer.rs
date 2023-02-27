use std::sync::Arc;

use crate::{command_pool::CommandPool, device::Device};
use ash::{prelude::VkResult, vk};

pub struct CommandBuffer {
    handle: vk::CommandBuffer,
    level: vk::CommandBufferLevel,

    // dependencies
    command_pool: Arc<CommandPool>,
}

impl CommandBuffer {
    pub fn new(command_pool: Arc<CommandPool>, level: vk::CommandBufferLevel) -> VkResult<Self> {
        let allocate_info_builder = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .level(level);

        let mut command_buffers = command_pool.allocate_command_buffers(allocate_info_builder)?;
        Ok(command_buffers.remove(0))
    }

    /// Safety: make sure `handle` was allocated from `descriptor_pool` of type `level`.
    pub(crate) unsafe fn from_handle(
        handle: vk::CommandBuffer,
        level: vk::CommandBufferLevel,
        command_pool: Arc<CommandPool>,
    ) -> Self {
        Self {
            handle,
            level,
            command_pool,
        }
    }

    // Getters

    pub fn handle(&self) -> vk::CommandBuffer {
        self.handle
    }

    pub fn level(&self) -> vk::CommandBufferLevel {
        self.level
    }

    #[inline]
    pub fn command_pool(&self) -> &Arc<CommandPool> {
        &self.command_pool
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        self.command_pool.device()
    }
}
