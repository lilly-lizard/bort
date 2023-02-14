use crate::{
    device::Device,
    image_base::ImageBase,
    image_properties::{ImageDimensions, ImageProperties, ImageViewProperties},
    memory::{MemoryAllocator, ALLOCATION_CALLBACK_NONE},
};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;
use vk_mem::{Alloc, AllocationCreateInfo};

pub struct Image {
    image_handle: vk::Image,
    image_properties: ImageProperties,

    image_view_handle: vk::ImageView,
    image_view_properties: ImageViewProperties,

    memory_allocator: Arc<MemoryAllocator>,
    memory_allocation: vk_mem::Allocation,

    // dependencies
    device: Arc<Device>,
}

impl Image {
    pub fn new(
        device: Arc<Device>,
        memory_allocator: Arc<MemoryAllocator>,
        image_properties: ImageProperties,
        image_view_properties: ImageViewProperties,
        memory_info: AllocationCreateInfo,
    ) -> anyhow::Result<Self> {
        let image_info = image_properties.create_info_builder().build();

        let (image_handle, memory_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_image(&image_info, &memory_info)
        }
        .context("creating image and memory allocation")?;

        let image_view_handle = unsafe {
            device.inner().create_image_view(
                &image_view_properties.create_info_builder(image_handle),
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .context("creating image view")?;

        Ok(Self {
            image_handle,
            image_properties,

            image_view_handle,
            image_view_properties,

            memory_allocator,
            memory_allocation,

            device,
        })
    }

    // Getters

    pub fn image_properties(&self) -> &ImageProperties {
        &self.image_properties
    }

    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocator
    }

    pub fn memory_allocation(&self) -> &vk_mem::Allocation {
        &self.memory_allocation
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl ImageBase for Image {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn dimensions(&self) -> ImageDimensions {
        self.image_properties.dimensions
    }

    fn image_view_properties(&self) -> ImageViewProperties {
        self.image_view_properties
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_image_view(self.image_view_handle, ALLOCATION_CALLBACK_NONE);

            self.memory_allocator
                .inner()
                .destroy_image(self.image_handle, &mut self.memory_allocation);
        }
    }
}
