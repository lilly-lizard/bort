use crate::{AllocationAccess, AllocatorAccess, Device, DeviceOwned, MemoryAllocation};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use bort_vma::{Alloc, AllocationCreateInfo};
use std::sync::Arc;

/// Contains a [VkBuffer](https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkBuffer.html)
/// and a memory allocation.
pub struct Buffer {
    handle: vk::Buffer,
    properties: BufferProperties,
    memory_allocation: MemoryAllocation,
}

impl Buffer {
    pub fn new(
        alloc_access: Arc<dyn AllocatorAccess>,
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

    pub unsafe fn new_from_create_info(
        alloc_access: Arc<dyn AllocatorAccess>,
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
        alloc_access: Arc<dyn AllocatorAccess>,
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
    pub fn allocator_access(&self) -> &Arc<dyn AllocatorAccess> {
        &self.memory_allocation.allocator_access()
    }

    #[inline]
    pub fn memory_allocation(&self) -> &MemoryAllocation {
        &self.memory_allocation
    }
}

impl AllocationAccess for Buffer {
    fn memory_allocation_mut(&mut self) -> &mut MemoryAllocation {
        &mut self.memory_allocation
    }
}

impl DeviceOwned for Buffer {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.allocator_access().device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.allocator_access()
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
