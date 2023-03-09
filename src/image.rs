use crate::{
    transient_image_info, AllocAccess, Device, DeviceOwned, ImageAccess, ImageDimensions,
    ImageProperties, MemoryAllocation, MemoryAllocator, MemoryError,
};
use ash::{prelude::VkResult, vk};
use bort_vma::{Alloc, AllocationCreateInfo};
use std::sync::Arc;

pub struct Image {
    handle: vk::Image,
    image_properties: ImageProperties,
    memory_allocation: MemoryAllocation,
}

impl Image {
    pub fn new(
        alloc_access: Arc<dyn AllocAccess>,
        image_properties: ImageProperties,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, vma_allocation) = unsafe {
            alloc_access
                .vma_allocator()
                .create_image(&image_properties.create_info_builder(), &allocation_info)
        }?;

        let memory_allocation = MemoryAllocation::from_vma_allocation(vma_allocation, alloc_access);

        Ok(Self {
            handle,
            image_properties,
            memory_allocation,
        })
    }

    pub fn new_from_create_info(
        alloc_access: Arc<dyn AllocAccess>,
        image_create_info_builder: vk::ImageCreateInfoBuilder,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let image_create_info = image_create_info_builder.build();
        let image_properties = ImageProperties::from(&image_create_info);

        let (handle, vma_allocation) = unsafe {
            alloc_access
                .vma_allocator()
                .create_image(&image_create_info, &allocation_info)
        }?;

        let memory_allocation = MemoryAllocation::from_vma_allocation(vma_allocation, alloc_access);

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
    pub fn properties(&self) -> &ImageProperties {
        &self.image_properties
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

impl ImageAccess for Image {
    #[inline]
    fn handle(&self) -> vk::Image {
        self.handle
    }

    #[inline]
    fn dimensions(&self) -> ImageDimensions {
        self.image_properties.dimensions
    }
}

impl DeviceOwned for Image {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.memory_allocation.device()
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.alloc_access()
                .clone()
                .vma_allocator()
                .destroy_image(self.handle, self.memory_allocation.inner_mut());
        }
    }
}
