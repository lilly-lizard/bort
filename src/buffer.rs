use crate::{AllocAccess, Device, DeviceOwned, MemoryAllocation, MemoryError};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use bort_vma::{Alloc, AllocationCreateInfo};
use std::sync::Arc;

pub struct Buffer {
    handle: vk::Buffer,
    properties: BufferProperties,
    memory_allocation: MemoryAllocation,
}

impl Buffer {
    pub fn new(
        alloc_access: Arc<dyn AllocAccess>,
        buffer_properties: BufferProperties,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let create_info_builder = buffer_properties.create_info_builder();

        let (handle, vma_allocation) = unsafe {
            alloc_access
                .vma_allocator()
                .create_buffer(&create_info_builder, &allocation_info)
        }?;

        Ok(Self::from_handle_and_allocation(
            alloc_access,
            buffer_properties,
            handle,
            vma_allocation,
        ))
    }

    pub fn new_from_create_info_builder(
        alloc_access: Arc<dyn AllocAccess>,
        buffer_create_info_builder: vk::BufferCreateInfoBuilder,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let properties = BufferProperties::from_create_info_builder(&buffer_create_info_builder);

        let (handle, vma_allocation) = unsafe {
            alloc_access
                .vma_allocator()
                .create_buffer(&buffer_create_info_builder, &allocation_info)
        }?;

        Ok(Self::from_handle_and_allocation(
            alloc_access,
            properties,
            handle,
            vma_allocation,
        ))
    }

    fn from_handle_and_allocation(
        alloc_access: Arc<dyn AllocAccess>,
        properties: BufferProperties,
        handle: vk::Buffer,
        vma_allocation: bort_vma::Allocation,
    ) -> Self {
        let memory_allocation = MemoryAllocation::from_vma_allocation(vma_allocation, alloc_access);

        Self {
            handle,
            properties,
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
    pub fn properties(&self) -> &BufferProperties {
        &self.properties
    }

    #[inline]
    pub fn alloc_access(&self) -> &Arc<dyn AllocAccess> {
        &self.memory_allocation.alloc_access()
    }

    #[inline]
    pub fn memory_allocation(&self) -> &MemoryAllocation {
        &self.memory_allocation
    }
}

impl DeviceOwned for Buffer {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.alloc_access().device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.alloc_access()
                .clone()
                .vma_allocator()
                .destroy_buffer(self.handle, self.memory_allocation.inner_mut());
        }
    }
}

// Properties

/// Note: default values for `size`, and `usage` are nothing!
#[derive(Clone)]
pub struct BufferProperties {
    pub flags: vk::BufferCreateFlags,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
}

impl Default for BufferProperties {
    fn default() -> Self {
        Self {
            flags: vk::BufferCreateFlags::empty(),
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),

            // nonsense defaults. make sure you override these!
            size: 0,
            usage: vk::BufferUsageFlags::empty(),
        }
    }
}

impl BufferProperties {
    pub fn new_default(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> Self {
        Self {
            size,
            usage,
            ..Default::default()
        }
    }

    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::BufferCreateInfoBuilder<'a>,
    ) -> vk::BufferCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .size(self.size)
            .usage(self.usage)
            .sharing_mode(self.sharing_mode)
            .queue_family_indices(&self.queue_family_indices)
    }

    pub fn create_info_builder(&self) -> vk::BufferCreateInfoBuilder {
        self.write_create_info_builder(vk::BufferCreateInfo::builder())
    }

    pub fn from_create_info_builder(value: &vk::BufferCreateInfoBuilder) -> Self {
        let mut queue_family_indices = Vec::<u32>::new();
        for i in 0..value.queue_family_index_count {
            let queue_family_index = unsafe { *value.p_queue_family_indices.offset(i as isize) };
            queue_family_indices.push(queue_family_index);
        }

        Self {
            flags: value.flags,
            size: value.size,
            usage: value.usage,
            sharing_mode: value.sharing_mode,
            queue_family_indices: queue_family_indices,
        }
    }
}
