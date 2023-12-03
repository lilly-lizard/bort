use crate::{
    default_component_mapping, default_subresource_range, extent_2d_from_width_height, Device,
    DeviceOwned, Fence, ImageAccess, ImageDimensions, ImageViewProperties, Semaphore, Surface,
    ALLOCATION_CALLBACK_NONE,
};
use ash::{
    extensions::khr,
    prelude::VkResult,
    vk::{self, Handle},
};
use std::{
    cmp::{max, min},
    error, fmt,
    sync::Arc,
};

// Swapchain

pub struct Swapchain {
    handle: vk::SwapchainKHR,
    swapchain_loader: khr::Swapchain,
    properties: SwapchainProperties,
    swapchain_images: Vec<Arc<SwapchainImage>>,

    // dependencies
    device: Arc<Device>,
    surface: Arc<Surface>,
}

impl Swapchain {
    pub fn new(
        device: Arc<Device>,
        surface: Arc<Surface>,
        properties: SwapchainProperties,
    ) -> Result<Self, SwapchainError> {
        let swapchain_loader = khr::Swapchain::new(device.instance().inner(), device.inner());

        let swapchain_create_info_builder =
            properties.create_info_builder(surface.handle(), vk::SwapchainKHR::null());
        let handle = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info_builder, ALLOCATION_CALLBACK_NONE)
        }
        .map_err(|e| SwapchainError::Creation(e))?;

        let vk_swapchain_images = unsafe { swapchain_loader.get_swapchain_images(handle) }
            .map_err(|e| SwapchainError::GetSwapchainImages(e))?;

        let swapchain_images = vk_swapchain_images
            .into_iter()
            .map(|image_handle| unsafe {
                Arc::new(SwapchainImage::from_image_handle(
                    device.clone(),
                    image_handle,
                    &properties,
                ))
            })
            .collect::<Vec<_>>();

        Ok(Self {
            handle,
            swapchain_loader,
            properties,
            swapchain_images,

            device,
            surface,
        })
    }

    /// On success, returns the next image's index and whether the swapchain is suboptimal for the surface.
    pub fn aquire_next_image(
        &self,
        timeout: u64,
        semaphore: Option<&Semaphore>,
        fence: Option<&Fence>,
    ) -> VkResult<(u32, bool)> {
        let semaphore_handle = if let Some(semaphore) = semaphore {
            semaphore.handle()
        } else {
            vk::Semaphore::null()
        };
        let fence_handle = if let Some(fence) = fence {
            fence.handle()
        } else {
            vk::Fence::null()
        };

        unsafe {
            self.swapchain_loader.acquire_next_image(
                self.handle,
                timeout,
                semaphore_handle,
                fence_handle,
            )
        }
    }

    /// Also destroys the old swapchain so make sure any resources depending on the swapchain and
    /// swapchain images are dropped before calling this! E.g. swapchain image views and framebuffers...
    pub fn recreate(&mut self, properties: SwapchainProperties) -> Result<(), SwapchainError> {
        let (new_handle, swapchain_images) = Self::recreate_common(&self, &properties)?;

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.handle, ALLOCATION_CALLBACK_NONE)
        };

        self.handle = new_handle;
        self.properties = properties;
        self.swapchain_images = swapchain_images;

        Ok(())
    }

    /// Same as `Self::recreate` but consumes an immutable (reference counted) `Swapchain` and
    /// returns a new `Swapchain`.
    ///
    /// Unlike `Self::recreate` this doesn't destroy the old swapchain. If you want it to be cleaned
    /// up, it must be dropped which requires any resources depending on the swapchain/swapchain images
    /// to be dropped e.g. swapchain image views and framebuffers...
    pub fn recreate_replace(
        self: &Arc<Self>,
        properties: SwapchainProperties,
    ) -> Result<Arc<Self>, SwapchainError> {
        let (new_handle, swapchain_images) = Self::recreate_common(&self, &properties)?;

        Ok(Arc::new(Self {
            handle: new_handle,
            swapchain_loader: self.swapchain_loader.clone(),
            properties,
            swapchain_images,
            device: self.device.clone(),
            surface: self.surface.clone(),
        }))
    }

    fn recreate_common(
        self: &Self,
        properties: &SwapchainProperties,
    ) -> Result<(vk::SwapchainKHR, Vec<Arc<SwapchainImage>>), SwapchainError> {
        let swapchain_create_info_builder =
            properties.create_info_builder(self.surface.handle(), self.handle);

        let new_handle = unsafe {
            self.swapchain_loader
                .create_swapchain(&swapchain_create_info_builder, ALLOCATION_CALLBACK_NONE)
        }
        .map_err(|e| SwapchainError::Creation(e))?;

        let vk_swapchain_images = unsafe { self.swapchain_loader.get_swapchain_images(new_handle) }
            .map_err(|e| SwapchainError::GetSwapchainImages(e))?;

        let swapchain_images = vk_swapchain_images
            .into_iter()
            .map(|image_handle| unsafe {
                Arc::new(SwapchainImage::from_image_handle(
                    self.device.clone(),
                    image_handle,
                    &properties,
                ))
            })
            .collect::<Vec<_>>();

        Ok((new_handle, swapchain_images))
    }

    pub fn image_view_properties(&self) -> ImageViewProperties {
        let format = self.properties().surface_format.format;
        let component_mapping = default_component_mapping();

        let layer_count = self.properties().array_layers;
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

    #[inline]
    pub fn handle(&self) -> vk::SwapchainKHR {
        self.handle
    }

    #[inline]
    pub fn swapchain_loader(&self) -> &khr::Swapchain {
        &self.swapchain_loader
    }

    #[inline]
    pub fn properties(&self) -> &SwapchainProperties {
        &self.properties
    }

    #[inline]
    pub fn surface(&self) -> &Arc<Surface> {
        &self.surface
    }

    #[inline]
    pub fn swapchain_images(&self) -> &Vec<Arc<SwapchainImage>> {
        &self.swapchain_images
    }
}

impl DeviceOwned for Swapchain {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.handle, ALLOCATION_CALLBACK_NONE)
        };
    }
}

// Swapchain Properties

/// WARNING when using `default()` the following values should be overridden:
/// - `surface_format`
/// - `dimensions`
/// - `image_usage`
/// - `pre_transform`
/// - `composite_alpha`
#[derive(Debug, Clone)]
pub struct SwapchainProperties {
    pub flags: vk::SwapchainCreateFlagsKHR,
    pub image_count: u32,
    pub pre_transform: vk::SurfaceTransformFlagsKHR,
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    pub present_mode: vk::PresentModeKHR,
    pub clipping_enabled: bool,

    // image properties
    pub surface_format: vk::SurfaceFormatKHR,
    pub width_height: [u32; 2],
    pub array_layers: u32,
    pub image_usage: vk::ImageUsageFlags,
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
}

impl Default for SwapchainProperties {
    fn default() -> Self {
        Self {
            image_count: 1,
            array_layers: 1,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),
            clipping_enabled: true,
            present_mode: vk::PresentModeKHR::MAILBOX,
            flags: vk::SwapchainCreateFlagsKHR::empty(),

            // nonsense defaults. make sure you override these!
            surface_format: vk::SurfaceFormatKHR::default(),
            width_height: [1, 1],
            image_usage: vk::ImageUsageFlags::empty(),
            pre_transform: vk::SurfaceTransformFlagsKHR::default(),
            composite_alpha: vk::CompositeAlphaFlagsKHR::empty(),
        }
    }
}

impl SwapchainProperties {
    /// Prefers the following settings:
    /// - present mode = `vk::PresentModeKHR::MAILBOX`
    /// - pre-transform = `vk::SurfaceTransformFlagsKHR::IDENTITY`
    ///
    /// `preferred_image_count` is clamped based on `vk::SurfaceCapabilitiesKHR`.
    ///
    /// `surface_format`, `composite_alpha` and `image_usage` are unchecked.
    ///
    /// Sharing mode is set to `vk::SharingMode::EXCLUSIVE`, only 1 array layer, and clipping is enabled.
    pub fn new_default(
        device: &Device,
        surface: &Surface,
        preferred_image_count: u32,
        surface_format: vk::SurfaceFormatKHR,
        composite_alpha: vk::CompositeAlphaFlagsKHR,
        image_usage: vk::ImageUsageFlags,
        window_dimensions: [u32; 2],
    ) -> Result<Self, SwapchainError> {
        let surface_capabilities = surface
            .get_physical_device_surface_capabilities(device.physical_device())
            .map_err(|e| SwapchainError::GetPhysicalDeviceSurfaceCapabilities(e))?;

        let mut image_count = preferred_image_count;
        // max_image_count == 0 when there is no limits
        if surface_capabilities.max_image_count != 0 {
            // clamp between max and min
            image_count = max(
                min(image_count, surface_capabilities.max_image_count),
                surface_capabilities.min_image_count,
            );
        }

        let extent = match surface_capabilities.current_extent.width {
            std::u32::MAX => vk::Extent2D {
                width: window_dimensions[0],
                height: window_dimensions[1],
            },
            _ => surface_capabilities.current_extent,
        };

        let present_modes = surface
            .get_physical_device_surface_present_modes(device.physical_device())
            .map_err(|e| SwapchainError::GetPhysicalDeviceSurfacePresentModes(e))?;
        let present_mode = present_modes
            .into_iter()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        // should think more about this if targeting mobile in the future...
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        Ok(Self {
            image_count,
            surface_format,
            width_height: [extent.width, extent.height],
            image_usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform,
            composite_alpha,
            present_mode,
            clipping_enabled: true,
            ..Self::default()
        })
    }

    pub fn create_info_builder(
        &self,
        surface_handle: vk::SurfaceKHR,
        old_swapchain_handle: vk::SwapchainKHR,
    ) -> vk::SwapchainCreateInfoKHRBuilder {
        vk::SwapchainCreateInfoKHR::builder()
            .flags(self.flags)
            .surface(surface_handle)
            .min_image_count(self.image_count)
            .image_format(self.surface_format.format)
            .image_color_space(self.surface_format.color_space)
            .image_extent(extent_2d_from_width_height(self.width_height))
            .image_array_layers(self.array_layers)
            .image_usage(self.image_usage)
            .image_sharing_mode(self.sharing_mode)
            .pre_transform(self.pre_transform)
            .composite_alpha(self.composite_alpha)
            .present_mode(self.present_mode)
            .clipped(self.clipping_enabled)
            .old_swapchain(old_swapchain_handle)
            .queue_family_indices(&self.queue_family_indices)
    }

    pub fn dimensions(&self) -> ImageDimensions {
        ImageDimensions::Dim2d {
            width: self.width_height[0],
            height: self.width_height[1],
            array_layers: 1,
        }
    }
}

// Swapchain Image

pub struct SwapchainImage {
    handle: vk::Image,
    dimensions: ImageDimensions,

    // dependencies
    device: Arc<Device>,
}

impl SwapchainImage {
    /// Safety: make sure image 'handle' was retreived from 'swapchain'
    pub(crate) unsafe fn from_image_handle(
        device: Arc<Device>,
        handle: vk::Image,
        swapchain_properties: &SwapchainProperties,
    ) -> Self {
        Self {
            handle,
            dimensions: swapchain_properties.dimensions(),
            device,
        }
    }

    // Getters

    #[inline]
    pub fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }
}

impl ImageAccess for SwapchainImage {
    #[inline]
    fn handle(&self) -> vk::Image {
        self.handle
    }

    #[inline]
    fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }
}

impl DeviceOwned for SwapchainImage {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

// Helper

/// Checks surface support for the first compositie alpha flag in order of preference:
/// 1. `vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED`
/// 2. `vk::CompositeAlphaFlagsKHR::OPAQUE`
/// 3. `vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED` (because cbf implimenting the logic for this)
/// 4. `vk::CompositeAlphaFlagsKHR::INHERIT` (noooope)
pub fn choose_composite_alpha(
    surface_capabilities: vk::SurfaceCapabilitiesKHR,
) -> vk::CompositeAlphaFlagsKHR {
    let supported_composite_alpha = surface_capabilities.supported_composite_alpha;
    let composite_alpha_preference_order = [
        vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::OPAQUE,
        vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED,
        vk::CompositeAlphaFlagsKHR::INHERIT,
    ];
    composite_alpha_preference_order
        .into_iter()
        .find(|&ca| supported_composite_alpha.contains(ca))
        .expect("driver should support at least one type of composite alpha!")
}

#[derive(Debug, Clone)]
pub enum SwapchainError {
    GetPhysicalDeviceSurfaceCapabilities(vk::Result),
    GetPhysicalDeviceSurfacePresentModes(vk::Result),
    Creation(vk::Result),
    GetSwapchainImages(vk::Result),
}

impl fmt::Display for SwapchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetPhysicalDeviceSurfaceCapabilities(e) => write!(
                f,
                "call to vkGetPhysicalDeviceSurfaceCapabilitiesKHR failed: {}",
                e
            ),
            Self::GetPhysicalDeviceSurfacePresentModes(e) => write!(
                f,
                "call to vkGetPhysicalDeviceSurfacePresentModesKHR failed: {}",
                e
            ),
            Self::Creation(e) => write!(f, "failed to create swapchain: {}", e),

            Self::GetSwapchainImages(e) => {
                write!(f, "call to vkGetSwapchainImagesKHR failed: {}", e)
            }
        }
    }
}

impl error::Error for SwapchainError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::GetPhysicalDeviceSurfaceCapabilities(e) => Some(e),
            Self::GetPhysicalDeviceSurfacePresentModes(e) => Some(e),
            Self::Creation(e) => Some(e),
            Self::GetSwapchainImages(e) => Some(e),
        }
    }
}
