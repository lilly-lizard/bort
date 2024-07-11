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
        let create_info = properties.create_info();

        let handle = unsafe {
            device
                .inner()
                .create_command_pool(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        device: Arc<Device>,
        create_info: vk::CommandPoolCreateInfo,
    ) -> VkResult<Self> {
        let properties = CommandPoolProperties::from_create_info(&create_info);

        let handle = unsafe {
            device
                .inner()
                .create_command_pool(&create_info, ALLOCATION_CALLBACK_NONE)
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
        let allocate_info = vk::CommandBufferAllocateInfo::default()
            .level(level)
            .command_buffer_count(command_buffer_count)
            .command_pool(self.handle);

        unsafe { self.allocate_command_buffers_from_allocate_info(allocate_info) }
    }

    #[inline]
    pub fn allocate_command_buffer(
        self: &Arc<Self>,
        level: vk::CommandBufferLevel,
    ) -> VkResult<CommandBuffer> {
        let mut vec = self.allocate_command_buffers(level, 1)?;
        Ok(vec.remove(0))
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn allocate_command_buffers_from_allocate_info(
        self: &Arc<Self>,
        allocate_info: vk::CommandBufferAllocateInfo,
    ) -> VkResult<Vec<CommandBuffer>> {
        let level = allocate_info.level;

        let command_buffer_handles = unsafe {
            self.device()
                .inner()
                .allocate_command_buffers(&allocate_info)
        }?;

        let command_buffers: Vec<CommandBuffer> = command_buffer_handles
            .into_iter()
            .map(|handle| unsafe { CommandBuffer::from_handle(handle, level, self.clone()) })
            .collect();
        Ok(command_buffers)
    }

    pub fn reset(&self, reset_flags: vk::CommandPoolResetFlags) -> VkResult<()> {
        unsafe {
            self.device
                .inner()
                .reset_command_pool(self.handle, reset_flags)
        }
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

    pub fn write_create_info<'a>(
        &self,
        create_info: vk::CommandPoolCreateInfo<'a>,
    ) -> vk::CommandPoolCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .queue_family_index(self.queue_family_index)
    }

    pub fn create_info(&self) -> vk::CommandPoolCreateInfo {
        self.write_create_info(vk::CommandPoolCreateInfo::default())
    }

    pub fn from_create_info(value: &vk::CommandPoolCreateInfo) -> Self {
        Self {
            flags: value.flags,

            // nonsense defaults. make sure you override these!
            queue_family_index: value.queue_family_index,
        }
    }
}
