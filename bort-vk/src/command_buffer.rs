use crate::{
    Buffer, CommandPool, DescriptorSet, Device, DeviceOwned, ImageAccess, PipelineAccess,
    PipelineLayout,
};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::{error::Error, sync::Arc};

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

    /// # Safety make sure `handle` was allocated from `descriptor_pool` of type `level`.
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

    /// Note: this will fail if the command pool wasn't created with `vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER`
    /// set.
    ///
    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkResetCommandBuffer.html>
    pub fn reset(&self, reset_flags: vk::CommandBufferResetFlags) -> VkResult<()> {
        unsafe {
            self.device()
                .inner()
                .reset_command_buffer(self.handle, reset_flags)
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

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkBeginCommandBuffer.html>
    pub fn begin(&self, begin_info: &vk::CommandBufferBeginInfo) -> VkResult<()> {
        unsafe {
            self.device()
                .inner()
                .begin_command_buffer(self.handle, begin_info)
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkEndCommandBuffer.html>
    pub fn end(&self) -> VkResult<()> {
        unsafe { self.device().inner().end_command_buffer(self.handle) }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdBeginRenderPass.html>
    pub fn begin_render_pass(
        &self,
        begin_info: &vk::RenderPassBeginInfo,
        subpass_contents: vk::SubpassContents,
    ) {
        unsafe {
            self.device()
                .inner()
                .cmd_begin_render_pass(self.handle, begin_info, subpass_contents)
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdNextSubpass.html>
    pub fn next_subpass(&self, subpass_contents: vk::SubpassContents) {
        unsafe {
            self.device()
                .inner()
                .cmd_next_subpass(self.handle, subpass_contents)
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdEndRenderPass.html>
    pub fn end_render_pass(&self) {
        unsafe { self.device().inner().cmd_end_render_pass(self.handle) }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdBindPipeline.html>
    pub fn bind_pipeline(&self, pipeline: &dyn PipelineAccess) {
        unsafe {
            self.device().inner().cmd_bind_pipeline(
                self.handle,
                pipeline.bind_point(),
                pipeline.handle(),
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdBindDescriptorSets.html>
    pub fn bind_descriptor_sets<'a>(
        &self,
        pipeline_bind_point: vk::PipelineBindPoint,
        pipeline_layout: &PipelineLayout,
        first_set: u32,
        descriptor_sets: impl IntoIterator<Item = &'a DescriptorSet>,
        dynamic_offsets: &[u32],
    ) {
        let descriptor_set_handles: Vec<vk::DescriptorSet> = descriptor_sets
            .into_iter()
            .map(|descriptor_set| descriptor_set.handle())
            .collect();
        unsafe {
            self.device().inner().cmd_bind_descriptor_sets(
                self.handle,
                pipeline_bind_point,
                pipeline_layout.handle(),
                first_set,
                &descriptor_set_handles,
                dynamic_offsets,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdBindVertexBuffers.html>
    pub fn bind_vertex_buffers<'a>(
        &self,
        first_binding: u32,
        buffers: impl IntoIterator<Item = &'a Buffer>,
        offsets: &[vk::DeviceSize],
    ) {
        let buffer_handles: Vec<vk::Buffer> =
            buffers.into_iter().map(|buffer| buffer.handle()).collect();
        unsafe {
            self.device().inner().cmd_bind_vertex_buffers(
                self.handle,
                first_binding,
                &buffer_handles,
                offsets,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdBindIndexBuffer.html>
    pub fn bind_index_buffer(
        &self,
        buffer: &Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) {
        unsafe {
            self.device().inner().cmd_bind_index_buffer(
                self.handle,
                buffer.handle(),
                offset,
                index_type,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdSetViewport.html>
    pub fn set_viewport(&self, first_viewport: u32, viewports: &[vk::Viewport]) {
        unsafe {
            self.device()
                .inner()
                .cmd_set_viewport(self.handle, first_viewport, viewports)
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdSetScissor.html>
    pub fn set_scissor(&self, first_scissor: u32, scissors: &[vk::Rect2D]) {
        unsafe {
            self.device()
                .inner()
                .cmd_set_scissor(self.handle, first_scissor, scissors)
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdDraw.html>
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

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdDrawIndexed.html>
    pub fn draw_indexed(
        &self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) {
        unsafe {
            self.device().inner().cmd_draw_indexed(
                self.handle,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdDrawIndexedIndirect.html>
    pub fn draw_indexed_indirect(
        &self,
        buffer: &Buffer,
        offset: vk::DeviceSize,
        draw_count: u32,
        stride: u32,
    ) {
        unsafe {
            self.device().inner().cmd_draw_indexed_indirect(
                self.handle,
                buffer.handle(),
                offset,
                draw_count,
                stride,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdExecuteCommands.html>
    pub fn execute_commands(
        &self,
        secondary_command_buffers: &[&CommandBuffer],
    ) -> Result<(), CommandError> {
        let any_primary_buffers = secondary_command_buffers
            .iter()
            .any(|command_buffer| command_buffer.level == vk::CommandBufferLevel::PRIMARY);
        if any_primary_buffers {
            return Err(CommandError::CantExecutePrimaryCommandBuffer);
        }

        let secondary_command_buffer_handles: Vec<vk::CommandBuffer> = secondary_command_buffers
            .iter()
            .map(|command_buffer| command_buffer.handle())
            .collect();

        unsafe {
            self.device()
                .inner()
                .cmd_execute_commands(self.handle, &secondary_command_buffer_handles);
        }

        Ok(())
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdCopyBuffer.html>
    pub fn copy_buffer(
        &self,
        src_buffer: &Buffer,
        dst_buffer: &Buffer,
        regions: &[vk::BufferCopy],
    ) {
        unsafe {
            self.device().inner().cmd_copy_buffer(
                self.handle,
                src_buffer.handle(),
                dst_buffer.handle(),
                regions,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdCopyBufferToImage.html>
    pub fn copy_buffer_to_image(
        &self,
        src_buffer: &Buffer,
        dst_image: &dyn ImageAccess,
        dst_image_layout: vk::ImageLayout,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device().inner().cmd_copy_buffer_to_image(
                self.handle,
                src_buffer.handle(),
                dst_image.handle(),
                dst_image_layout,
                regions,
            )
        }
    }

    /// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/vkCmdPushConstants.html>
    pub fn push_constants(
        &self,
        pipeline_layout: &PipelineLayout,
        stage_flags: vk::ShaderStageFlags,
        offset: u32,
        constants: &[u8],
    ) {
        unsafe {
            self.device().inner().cmd_push_constants(
                self.handle,
                pipeline_layout.handle(),
                stage_flags,
                offset,
                constants,
            )
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

// ~~ Errors ~~

#[derive(Clone, Copy, Debug)]
pub enum CommandError {
    CantExecutePrimaryCommandBuffer,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CantExecutePrimaryCommandBuffer => write!(
                f,
                "attempted to call vkCmdExecuteCommands on a primary command buffer"
            ),
        }
    }
}

impl Error for CommandError {}
