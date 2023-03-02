use crate::{
    device::Device,
    memory::{MemoryAllocation, MemoryAllocator, MemoryError},
};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;
use vk_mem::{Alloc, AllocationCreateInfo};

pub struct Buffer {
    handle: vk::Buffer,
    buffer_properties: BufferProperties,
    memory_allocation: MemoryAllocation,
}

impl Buffer {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        buffer_properties: BufferProperties,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, vma_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_buffer(&buffer_properties.create_info_builder(), &allocation_info)
        }?;

        Ok(Self::from_handle_and_allocation(
            memory_allocator,
            buffer_properties,
            handle,
            vma_allocation,
        ))
    }

    fn from_handle_and_allocation(
        memory_allocator: Arc<MemoryAllocator>,
        buffer_properties: BufferProperties,
        handle: vk::Buffer,
        vma_allocation: vk_mem::Allocation,
    ) -> Self {
        let memory_allocation =
            MemoryAllocation::from_vma_allocation(vma_allocation, memory_allocator);

        Self {
            handle,
            buffer_properties,
            memory_allocation,
        }
    }

    /// Note that if memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` writing will fail
    pub fn write_struct<T>(&mut self, data: T, mem_offset: usize) -> Result<(), MemoryError> {
        self.memory_allocation.write_struct(data, mem_offset)
    }

    /// Note that if memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` writing will fail
    pub fn write_iter<I, T>(&mut self, data: I, mem_offset: usize) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.memory_allocation.write_iter(data, mem_offset)
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::Buffer {
        self.handle
    }

    #[inline]
    pub fn buffer_properties(&self) -> &BufferProperties {
        &self.buffer_properties
    }

    #[inline]
    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocation.memory_allocator()
    }

    #[inline]
    pub fn memory_allocation(&self) -> &MemoryAllocation {
        &self.memory_allocation
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.memory_allocator().device()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.memory_allocator()
                .clone()
                .inner()
                .destroy_buffer(self.handle, self.memory_allocation.inner_mut());
        }
    }
}

#[derive(Clone)]
pub struct BufferProperties {
    pub create_flags: vk::BufferCreateFlags,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
}

impl Default for BufferProperties {
    fn default() -> Self {
        Self {
            create_flags: vk::BufferCreateFlags::empty(),
            size: 0,
            usage: vk::BufferUsageFlags::empty(),
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),
        }
    }
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
