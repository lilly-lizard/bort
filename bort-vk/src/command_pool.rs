use crate::{CommandBuffer, Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
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

    pub fn new_from_create_info(
        device: Arc<Device>,
        create_info_builder: vk::CommandPoolCreateInfoBuilder,
    ) -> VkResult<Self> {
        let properties = CommandPoolProperties::from_create_info_builder(&create_info_builder);

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
}

impl DeviceOwned for CommandPool {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
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

// Properties

/// Note: default value for `queue_family_index` is nothing!
#[derive(Default, Clone, Copy)]
pub struct CommandPoolProperties {
    pub flags: vk::CommandPoolCreateFlags,
    pub queue_family_index: u32,
}

impl CommandPoolProperties {
    pub fn new_default(queue_family_index: u32) -> Self {
        Self {
            queue_family_index,
            ..Default::default()
        }
    }

    pub fn write_create_info_builder<'a>(
        &self,
        builder: vk::CommandPoolCreateInfoBuilder<'a>,
    ) -> vk::CommandPoolCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .queue_family_index(self.queue_family_index)
    }

    pub fn create_info_builder(&self) -> vk::CommandPoolCreateInfoBuilder {
        self.write_create_info_builder(vk::CommandPoolCreateInfo::builder())
    }

    pub fn from_create_info_builder(value: &vk::CommandPoolCreateInfoBuilder) -> Self {
        Self {
            flags: value.flags,

            // nonsense defaults. make sure you override these!
            queue_family_index: value.queue_family_index,
        }
    }
}
