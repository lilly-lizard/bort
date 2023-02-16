use crate::{
    device::Device,
    image_properties::{transient_image_info, ImageDimensions, ImageProperties},
    memory::MemoryAllocator,
};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;
use vk_mem::{Alloc, AllocationCreateInfo};

pub trait ImageAccess {
    fn handle(&self) -> vk::Image;
    fn dimensions(&self) -> ImageDimensions;
    fn device(&self) -> &Arc<Device>;
}

pub struct Image {
    handle: vk::Image,
    properties: ImageProperties,
    memory_allocation: vk_mem::Allocation,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl Image {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        properties: ImageProperties,
        memory_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, memory_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_image(&properties.create_info_builder(), &memory_info)
        }?;

        Ok(Self {
            handle,
            properties,
            memory_allocation,
            memory_allocator,
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

    // Getters

    pub fn properties(&self) -> &ImageProperties {
        &self.properties
    }

    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocator
    }

    pub fn memory_allocation(&self) -> &vk_mem::Allocation {
        &self.memory_allocation
    }
}

impl ImageAccess for Image {
    fn handle(&self) -> vk::Image {
        self.handle
    }

    fn dimensions(&self) -> ImageDimensions {
        self.properties.dimensions
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.memory_allocator.device()
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.memory_allocator
                .inner()
                .destroy_image(self.handle, &mut self.memory_allocation);
        }
    }
}
