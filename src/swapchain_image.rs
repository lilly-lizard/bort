use crate::{
    device::Device,
    image_base::ImageBase,
    image_properties::{
        default_component_mapping, default_subresource_range, ImageDimensions, ImageViewProperties,
    },
    memory::ALLOCATION_CALLBACK_NONE,
    swapchain::Swapchain,
};
use ash::vk;
use std::{error, fmt, sync::Arc};

pub struct SwapchainImage {
    image_handle: vk::Image,
    image_view_handle: vk::ImageView,
    image_view_properties: ImageViewProperties,
    dimensions: ImageDimensions,

    // dependencies
    swapchain: Arc<Swapchain>,
}

impl SwapchainImage {
    pub fn from_swapchain(swapchain: Arc<Swapchain>) -> Result<Vec<Self>, SwapchainImageError> {
        swapchain
            .get_swapchain_images()
            .map_err(|e| SwapchainImageError::GetSwapchainImages(e))?
            .into_iter()
            .map(|image_handle| Self::from_image_handle(swapchain.clone(), image_handle))
            .collect::<Result<Vec<_>, _>>()
    }

    fn from_image_handle(
        swapchain: Arc<Swapchain>,
        image_handle: vk::Image,
    ) -> Result<Self, SwapchainImageError> {
        let device = swapchain.device();

        let format = swapchain.properties().surface_format.format;
        let component_mapping = default_component_mapping();

        let layer_count = swapchain.properties().array_layers;
        let view_type = if layer_count > 1 {
            vk::ImageViewType::TYPE_2D_ARRAY
        } else {
            vk::ImageViewType::TYPE_2D
        };
        let subresource_range = vk::ImageSubresourceRange {
            layer_count,
            ..default_subresource_range(vk::ImageAspectFlags::COLOR)
        };

        let image_view_properties = ImageViewProperties {
            format,
            view_type,
            component_mapping,
            subresource_range,
            ..ImageViewProperties::default()
        };

        let image_view_create_info_builder =
            image_view_properties.create_info_builder(image_handle);
        let image_view_handle = unsafe {
            device
                .inner()
                .create_image_view(&image_view_create_info_builder, ALLOCATION_CALLBACK_NONE)
        }
        .map_err(|e| SwapchainImageError::CreateImageView(e))?;

        Ok(Self {
            image_handle,
            image_view_handle,
            image_view_properties,
            dimensions: swapchain.properties().dimensions(),

            swapchain,
        })
    }

    // Getters

    pub fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    pub fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    pub fn image_view_properties(&self) -> ImageViewProperties {
        self.image_view_properties
    }

    pub fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    pub fn layer_count(&self) -> u32 {
        self.image_view_properties.subresource_range.layer_count
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.swapchain.device()
    }

    pub fn swapchain(&self) -> &Arc<Swapchain> {
        &self.swapchain
    }
}

impl ImageBase for SwapchainImage {
    fn image_handle(&self) -> vk::Image {
        self.image_handle
    }

    fn image_view_handle(&self) -> vk::ImageView {
        self.image_view_handle
    }

    fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    fn image_view_properties(&self) -> ImageViewProperties {
        self.image_view_properties
    }
}

impl Drop for SwapchainImage {
    fn drop(&mut self) {
        // note we shouldn't destroy the swapchain images. that'll be handled by the `Swapchain`.
        unsafe {
            self.device()
                .inner()
                .destroy_image_view(self.image_view_handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Debug, Clone)]
pub enum SwapchainImageError {
    CreateImageView(vk::Result),
    GetSwapchainImages(vk::Result),
}

impl fmt::Display for SwapchainImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateImageView(e) => write!(f, "failed to create image view: {}", e),
            Self::GetSwapchainImages(e) => {
                write!(f, "call to vkGetSwapchainImagesKHR failed: {}", e)
            }
        }
    }
}

impl error::Error for SwapchainImageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::CreateImageView(e) => Some(e),
            Self::GetSwapchainImages(e) => Some(e),
        }
    }
}
