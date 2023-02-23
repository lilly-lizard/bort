use crate::{device::Device, memory::MemoryAllocator};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;
use vk_mem::{Alloc, AllocationCreateInfo};

pub struct Buffer {
    handle: vk::Buffer,
    buffer_properties: BufferProperties,

    memory_allocation: vk_mem::Allocation,
    memory_type: vk::MemoryType,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl Buffer {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        mut buffer_properties: BufferProperties,
        memory_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, memory_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_buffer(&buffer_properties.create_info_builder(), &memory_info)
        }?;

        let memory_info = memory_allocator
            .inner()
            .get_allocation_info(&memory_allocation);

        buffer_properties.size = memory_info.size; // should be the same, but just in case

        let physical_device_mem_props = memory_allocator
            .device()
            .physical_device()
            .memory_properties();

        debug_assert!(memory_info.memory_type < physical_device_mem_props.memory_type_count);
        let memory_type = physical_device_mem_props.memory_types[memory_info.memory_type as usize];

        Ok(Self {
            handle,
            buffer_properties,
            memory_allocation,
            memory_type,
            memory_allocator,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::Buffer {
        self.handle
    }

    pub fn buffer_properties(&self) -> &BufferProperties {
        &self.buffer_properties
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

    pub fn memory_type(&self) -> vk::MemoryType {
        self.memory_type
    }

    pub fn memory_property_flags(&self) -> vk::MemoryPropertyFlags {
        self.memory_type.property_flags
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.memory_allocator.device()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.memory_allocator
                .inner()
                .destroy_buffer(self.handle, &mut self.memory_allocation);
        }
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
