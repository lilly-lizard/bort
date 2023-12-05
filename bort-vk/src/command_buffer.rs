use crate::{CommandPool, Device, DeviceOwned, PipelineAccess};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
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

    // Commands

    /// vkCmdBeginCommandBuffer
    pub fn begin(&self, begin_info: &vk::CommandBufferBeginInfoBuilder) -> VkResult<()> {
        unsafe {
            self.device()
                .inner()
                .begin_command_buffer(self.handle, begin_info)
        }
    }

    /// vkCmdEndCommandBuffer
    pub fn end(&self) -> VkResult<()> {
        unsafe { self.device().inner().end_command_buffer(self.handle) }
    }

    /// vkCmdBeginRenderPass
    pub fn begin_render_pass(
        &self,
        begin_info: &vk::RenderPassBeginInfoBuilder,
        subpass_contents: vk::SubpassContents,
    ) {
        unsafe {
            self.device()
                .inner()
                .cmd_begin_render_pass(self.handle, &begin_info, subpass_contents)
        }
    }

    /// vkCmdEndRenderPass
    pub fn end_render_pass(&self) {
        unsafe { self.device().inner().cmd_end_render_pass(self.handle) }
    }

    /// vkCmdBindPipeline
    pub fn bind_pipeline(&self, pipeline: &dyn PipelineAccess) {
        unsafe {
            self.device().inner().cmd_bind_pipeline(
                self.handle,
                pipeline.bind_point(),
                pipeline.handle(),
            )
        }
    }

    /// vkCmdSetViewport
    pub fn set_viewports(&self, viewports: &[vk::Viewport], first_viewport: u32) {
        unsafe {
            self.device()
                .inner()
                .cmd_set_viewport(self.handle, first_viewport, viewports)
        }
    }

    /// vkCmdSetScissor
    pub fn set_scissors(&self, scissors: &[vk::Rect2D], first_scissor: u32) {
        unsafe {
            self.device()
                .inner()
                .cmd_set_scissor(self.handle, first_scissor, scissors)
        }
    }

    /// vkCmdDraw
    pub fn draw(
        &self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) {
        unsafe {
            self.device().inner().cmd_draw(
                self.handle,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        }
    }

    /// Note: this will fail if the command pool wasn't created with `vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER`
    /// set.
    ///
    /// vkResetCommandBuffer
    pub fn reset(&self, reset_flags: vk::CommandBufferResetFlags) -> VkResult<()> {
        unsafe {
            self.device()
                .inner()
                .reset_command_buffer(self.handle, reset_flags)
        }
    }
}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device()
                .inner()
                .free_command_buffers(self.command_pool.handle(), &[self.handle])
        }
    }
}

impl DeviceOwned for CommandBuffer {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.command_pool.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}
