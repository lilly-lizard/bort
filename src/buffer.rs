use crate::{device::Device, memory::MemoryAllocator};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;
use vk_mem::{Alloc, AllocationCreateInfo};

pub struct Buffer {
    handle: vk::Buffer,
    properties: BufferProperties,
    memory_allocation: vk_mem::Allocation,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl Buffer {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        properties: BufferProperties,
        memory_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, memory_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_buffer(&properties.create_info_builder(), &memory_info)
        }?;

        Ok(Self {
            handle,
            properties,
            memory_allocation,
            memory_allocator,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::Buffer {
        self.handle
    }

    pub fn properties(&self) -> &BufferProperties {
        &self.properties
    }

    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocator
    }

    pub fn memory_allocation(&self) -> &vk_mem::Allocation {
        &self.memory_allocation
    }

    pub fn memory_allocation_mut(&mut self) -> &mut vk_mem::Allocation {
        &mut self.memory_allocation
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.memory_allocator.device()
    }
}

#[derive(Clone, Default)]
pub struct BufferProperties {
    pub create_flags: vk::BufferCreateFlags,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
}

impl BufferProperties {
    pub fn create_info_builder(&self) -> vk::BufferCreateInfoBuilder {
        vk::BufferCreateInfo::builder()
            .flags(self.create_flags)
            .size(self.size)
            .usage(self.usage)
            .sharing_mode(self.sharing_mode)
            .queue_family_indices(self.queue_family_indices.as_slice())
    }
}
