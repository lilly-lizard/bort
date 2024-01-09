use crate::{Device, DeviceOwned, ImageAccess, ImageProperties, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

/// Unifies image views with different types of images
pub trait ImageViewAccess: DeviceOwned + Send + Sync {
    fn handle(&self) -> vk::ImageView;
    fn image_access(&self) -> Arc<dyn ImageAccess>;
}

pub struct ImageView<I: ImageAccess + 'static> {
    handle: vk::ImageView,
    properties: ImageViewProperties,

    // dependencies
    image: Arc<I>,
}

impl<I: ImageAccess + 'static> ImageView<I> {
    pub fn new(image: Arc<I>, properties: ImageViewProperties) -> VkResult<Self> {
        let create_info_builder = properties.create_info_builder(image.handle());

        let handle = unsafe {
            image
                .device()
                .inner()
                .create_image_view(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            image,
        })
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        image: Arc<I>,
        create_info_builder: vk::ImageViewCreateInfoBuilder,
    ) -> VkResult<Self> {
        let properties = ImageViewProperties::from_create_info_builder(&create_info_builder);

        let handle = unsafe {
            image
                .device()
                .inner()
                .create_image_view(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            image,
        })
    }

    // Getters

    pub fn properties(&self) -> &ImageViewProperties {
        &self.properties
    }

    pub fn image(&self) -> &Arc<I> {
        &self.image
    }
}

impl<I: ImageAccess + 'static> ImageViewAccess for ImageView<I> {
    fn handle(&self) -> vk::ImageView {
        self.handle
    }

    fn image_access(&self) -> Arc<dyn ImageAccess> {
        self.image.clone()
    }
}

impl<I: ImageAccess + 'static> DeviceOwned for ImageView<I> {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.image.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl<I: ImageAccess + 'static> Drop for ImageView<I> {
    fn drop(&mut self) {
        unsafe {
            self.device()
                .inner()
                .destroy_image_view(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

/// WARNING `default()` values for `format`, `view_type` are nothing!
#[derive(Debug, Copy, Clone)]
pub struct ImageViewProperties {
    pub flags: vk::ImageViewCreateFlags,
    pub view_type: vk::ImageViewType,
    pub component_mapping: vk::ComponentMapping,
    pub format: vk::Format,
    pub subresource_range: vk::ImageSubresourceRange,
}

impl Default for ImageViewProperties {
    fn default() -> Self {
        Self {
            component_mapping: default_component_mapping(),
            flags: vk::ImageViewCreateFlags::empty(),
            subresource_range: default_subresource_range(vk::ImageAspectFlags::COLOR),

            // nonsense defaults. make sure you override these!
            format: vk::Format::default(),
            view_type: vk::ImageViewType::default(),
        }
    }
}

impl ImageViewProperties {
    pub fn from_image_properties_default(image_properties: &ImageProperties) -> Self {
        let format = image_properties.format;
        let view_type = image_properties.dimensions.default_image_view_type();
        let subresource_range = image_properties.subresource_range();

        Self {
            format,
            subresource_range,
            view_type,
            ..Self::default()
        }
    }

    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::ImageViewCreateInfoBuilder<'a>,
        image_handle: vk::Image,
    ) -> vk::ImageViewCreateInfoBuilder {
        builder
            .flags(self.flags)
            .image(image_handle)
            .view_type(self.view_type)
            .format(self.format)
            .components(self.component_mapping)
            .subresource_range(self.subresource_range)
    }

    pub fn create_info_builder(&self, image_handle: vk::Image) -> vk::ImageViewCreateInfoBuilder {
        self.write_create_info_builder(vk::ImageViewCreateInfo::builder(), image_handle)
    }

    pub fn from_create_info_builder(create_info: &vk::ImageViewCreateInfoBuilder) -> Self {
        Self {
            flags: create_info.flags,
            view_type: create_info.view_type,
            component_mapping: create_info.components,
            format: create_info.format,
            subresource_range: create_info.subresource_range,
        }
    }
}

// Helper Functions

pub fn default_component_mapping() -> vk::ComponentMapping {
    vk::ComponentMapping {
        r: vk::ComponentSwizzle::R,
        g: vk::ComponentSwizzle::G,
        b: vk::ComponentSwizzle::B,
        a: vk::ComponentSwizzle::A,
    }
}

pub fn default_subresource_range(aspect_mask: vk::ImageAspectFlags) -> vk::ImageSubresourceRange {
    vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
    }
}

pub fn default_subresource_layers(aspect_mask: vk::ImageAspectFlags) -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
    }
}
