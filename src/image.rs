use crate::{
    device::Device,
    image_access::ImageAccess,
    image_properties::{transient_image_info, ImageDimensions, ImageProperties},
    memory::{MemoryAllocation, MemoryAllocator, MemoryError},
};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;
use vk_mem::{Alloc, AllocationCreateInfo};

pub struct Image {
    handle: vk::Image,
    image_properties: ImageProperties,
    memory_allocation: MemoryAllocation,
}

impl Image {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        image_properties: ImageProperties,
        memory_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, vma_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_image(&image_properties.create_info_builder(), &memory_info)
        }?;

        let memory_allocation =
            MemoryAllocation::from_vma_allocation(vma_allocation, memory_allocator);

        Ok(Self {
            handle,
            image_properties,
            memory_allocation,
        })
    }

    /// Create a transient, lazily allocated image.
    pub fn new_tranient(
        memory_allocator: Arc<MemoryAllocator>,
        dimensions: ImageDimensions,
        format: vk::Format,
        additional_usage: vk::ImageUsageFlags,
    ) -> VkResult<Self> {
        let (image_properties, allocation_info) =
            transient_image_info(dimensions, format, additional_usage);

        Self::new(memory_allocator, image_properties, allocation_info)
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
    pub fn image_properties(&self) -> &ImageProperties {
        &self.image_properties
    }

    #[inline]
    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocation.memory_allocator()
    }

    #[inline]
    pub fn memory_allocation(&self) -> &MemoryAllocation {
        &self.memory_allocation
    }
}

impl ImageAccess for Image {
    #[inline]
    fn handle(&self) -> vk::Image {
        self.handle
    }

    #[inline]
    fn dimensions(&self) -> ImageDimensions {
        self.image_properties.dimensions
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.memory_allocation.device()
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.memory_allocator()
                .clone()
                .inner()
                .destroy_image(self.handle, self.memory_allocation.inner_mut());
        }
    }
}
