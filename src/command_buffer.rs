use crate::{command_pool::CommandPool, device::Device};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct CommandBuffer {
    handle: vk::CommandBuffer,
    level: vk::CommandBufferLevel,

    // dependencies
    command_pool: Arc<CommandPool>,
}

impl CommandBuffer {
    /// Allocates a single command buffer. To allocate multiple at a time, use `CommandPool::allocate_command_buffers`.
    pub fn new(command_pool: Arc<CommandPool>, level: vk::CommandBufferLevel) -> VkResult<Self> {
        let mut command_buffers = command_pool.allocate_command_buffers(level, 1)?;
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
