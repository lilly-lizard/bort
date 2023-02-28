use crate::{command_buffer::CommandBuffer, device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct CommandPool {
    handle: vk::CommandPool,
    properties: CommandPoolProperties,

    // dependencies
    device: Arc<Device>,
}

impl CommandPool {
    pub fn new(device: Arc<Device>, properties: CommandPoolProperties) -> VkResult<Self> {
        let create_info_builder = properties.create_info_builder();

        let handle = unsafe {
            device
                .inner()
                .create_command_pool(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    pub fn from_create_info(
        device: Arc<Device>,
        create_info_builder: vk::CommandPoolCreateInfoBuilder,
    ) -> VkResult<Self> {
        let flags = create_info_builder.flags;
        let queue_family_index = create_info_builder.queue_family_index;

        let handle = unsafe {
            device
                .inner()
                .create_command_pool(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties: CommandPoolProperties {
                flags,
                queue_family_index,
            },
            device,
        })
    }

    pub fn allocate_command_buffers(
        self: &Arc<Self>,
        level: vk::CommandBufferLevel,
        command_buffer_count: u32,
    ) -> VkResult<Vec<CommandBuffer>> {
        let allocate_info_builder = vk::CommandBufferAllocateInfo::builder()
            .level(level)
            .command_buffer_count(command_buffer_count)
            .command_pool(self.handle);

        self.allocate_command_buffers_from_allocate_info(allocate_info_builder)
    }

    pub fn allocate_command_buffers_from_allocate_info(
        self: &Arc<Self>,
        allocate_info_builder: vk::CommandBufferAllocateInfoBuilder,
    ) -> VkResult<Vec<CommandBuffer>> {
        let level = allocate_info_builder.level;

        let command_buffer_handles = unsafe {
            self.device()
                .inner()
                .allocate_command_buffers(&allocate_info_builder)
        }?;

        let command_buffers = command_buffer_handles
            .into_iter()
            .map(|handle| unsafe { CommandBuffer::from_handle(handle, level, self.clone()) })
            .collect::<Vec<_>>();
        Ok(command_buffers)
    }

    // Getters

    pub fn handle(&self) -> vk::CommandPool {
        self.handle
    }

    pub fn properties(&self) -> CommandPoolProperties {
        self.properties
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_command_pool(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Default, Clone, Copy)]
pub struct CommandPoolProperties {
    pub flags: vk::CommandPoolCreateFlags,
    pub queue_family_index: u32,
}

impl CommandPoolProperties {
    pub fn create_info_builder(&self) -> vk::CommandPoolCreateInfoBuilder {
        vk::CommandPoolCreateInfo::builder()
            .flags(self.flags)
            .queue_family_index(self.queue_family_index)
    }
}
