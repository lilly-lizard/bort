use crate::{
    device::Device,
    image::ImageBase,
    image_properties::ImageDimensions,
    image_view::{default_component_mapping, default_subresource_range, ImageViewProperties},
    swapchain::Swapchain,
};
use ash::vk;
use std::{error, fmt, sync::Arc};

pub struct SwapchainImage {
    handle: vk::Image,
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
            .map(|image_handle| unsafe { Self::from_image_handle(swapchain.clone(), image_handle) })
            .collect::<Result<Vec<_>, _>>()
    }

    /// Safety: make sure image 'handle' was retreived from 'swapchain'
    unsafe fn from_image_handle(
        swapchain: Arc<Swapchain>,
        handle: vk::Image,
    ) -> Result<Self, SwapchainImageError> {
        Ok(Self {
            handle,
            dimensions: swapchain.properties().dimensions(),

            swapchain,
        })
    }

    pub fn image_view_properties(&self) -> ImageViewProperties {
        let format = self.swapchain.properties().surface_format.format;
        let component_mapping = default_component_mapping();

        let layer_count = self.swapchain.properties().array_layers;
        let view_type = if layer_count > 1 {
            vk::ImageViewType::TYPE_2D_ARRAY
        } else {
            vk::ImageViewType::TYPE_2D
        };
        let subresource_range = vk::ImageSubresourceRange {
            layer_count,
            ..default_subresource_range(vk::ImageAspectFlags::COLOR)
        };

        ImageViewProperties {
            format,
            view_type,
            component_mapping,
            subresource_range,
            ..ImageViewProperties::default()
        }
    }
    // Getters

    pub fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    pub fn swapchain(&self) -> &Arc<Swapchain> {
        &self.swapchain
    }
}

impl ImageBase for SwapchainImage {
    fn handle(&self) -> vk::Image {
        self.handle
    }

    fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.swapchain.device()
    }
}

#[derive(Debug, Clone)]
pub enum SwapchainImageError {
    GetSwapchainImages(vk::Result),
}

impl fmt::Display for SwapchainImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetSwapchainImages(e) => {
                write!(f, "call to vkGetSwapchainImagesKHR failed: {}", e)
            }
        }
    }
}

impl error::Error for SwapchainImageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::GetSwapchainImages(e) => Some(e),
        }
    }
}
