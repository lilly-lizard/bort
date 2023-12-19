use crate::{
    AllocationAccess, AllocatorAccess, Device, DeviceOwned, ImageAccess, ImageDimensions,
    MemoryAllocation, MemoryAllocator, PhysicalDevice,
};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use bort_vma::{Alloc, AllocationCreateInfo};
use std::sync::Arc;

// ~~ Image ~~

pub struct Image {
    handle: vk::Image,
    image_properties: ImageProperties,
    memory_allocation: MemoryAllocation,
}

impl Image {
    pub fn new(
        alloc_access: Arc<dyn AllocatorAccess>,
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

    pub unsafe fn new_from_create_info(
        alloc_access: Arc<dyn AllocatorAccess>,
        image_create_info_builder: vk::ImageCreateInfoBuilder,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let image_properties =
            ImageProperties::from_create_info_builder(&image_create_info_builder);

        let (handle, vma_allocation) = unsafe {
            alloc_access
                .vma_allocator()
                .create_image(&image_create_info_builder, &allocation_info)
        }?;

        let memory_allocation = MemoryAllocation::from_vma_allocation(vma_allocation, alloc_access);

        Ok(Self {
            handle,
            image_properties,
            memory_allocation,
        })
    }

    /// Create a (preferably) lazily-allocated transient attachment image.
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

    #[inline]
    pub fn properties(&self) -> &ImageProperties {
        &self.image_properties
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

impl AllocationAccess for Image {
    fn memory_allocation_mut(&mut self) -> &mut MemoryAllocation {
        &mut self.memory_allocation
    }
}

impl DeviceOwned for Image {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.memory_allocation.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.allocator_access()
                .clone()
                .vma_allocator()
                .destroy_image(self.handle, self.memory_allocation.inner_mut());
        }
    }
}

// ~~ Presets ~~

/// Properties for a device local and preferably lazily-allocated transient attachment image.
pub fn transient_image_info(
    dimensions: ImageDimensions,
    format: vk::Format,
    additional_usage: vk::ImageUsageFlags,
) -> (ImageProperties, AllocationCreateInfo) {
    let image_properties = ImageProperties::new_default(
        format,
        dimensions,
        vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | additional_usage,
    );

    let allocation_info = AllocationCreateInfo {
        //usage: bort_vma::MemoryUsage::GpuLazy,
        required_flags: vk::MemoryPropertyFlags::DEVICE_LOCAL,
        preferred_flags: vk::MemoryPropertyFlags::LAZILY_ALLOCATED,
        ..AllocationCreateInfo::default()
    };

    (image_properties, allocation_info)
}

// ~~ Image Properties ~~

/// Note: default values for `format`, `dimensions` and `usage` are nothing!
#[derive(Debug, Clone)]
pub struct ImageProperties {
    pub flags: vk::ImageCreateFlags,
    pub format: vk::Format,
    pub dimensions: ImageDimensions,
    pub mip_levels: u32,
    pub samples: vk::SampleCountFlags,
    pub tiling: vk::ImageTiling,
    pub usage: vk::ImageUsageFlags,
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
    pub initial_layout: vk::ImageLayout,
}

impl Default for ImageProperties {
    fn default() -> Self {
        Self {
            mip_levels: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling: vk::ImageTiling::OPTIMAL,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),
            initial_layout: vk::ImageLayout::UNDEFINED,
            flags: vk::ImageCreateFlags::empty(),

            // nonsense defaults. make sure you override these!
            format: vk::Format::default(),
            dimensions: ImageDimensions::default(),
            usage: vk::ImageUsageFlags::empty(),
        }
    }
}

impl ImageProperties {
    pub fn subresource_range(&self) -> vk::ImageSubresourceRange {
        let aspect_mask = aspect_mask_from_format(self.format);
        vk::ImageSubresourceRange {
            aspect_mask,
            base_mip_level: 0,
            level_count: self.mip_levels,
            base_array_layer: 0,
            layer_count: self.dimensions.array_layers(),
        }
    }

    #[inline]
    pub fn new_default(
        format: vk::Format,
        dimensions: ImageDimensions,
        usage: vk::ImageUsageFlags,
    ) -> Self {
        Self {
            format,
            dimensions,
            usage,
            ..Self::default()
        }
    }

    pub fn create_info_builder(&self) -> vk::ImageCreateInfoBuilder {
        vk::ImageCreateInfo::builder()
            .flags(self.flags)
            .image_type(self.dimensions.image_type())
            .format(self.format)
            .extent(self.dimensions.extent_3d())
            .mip_levels(self.mip_levels)
            .array_layers(self.dimensions.array_layers())
            .samples(self.samples)
            .tiling(self.tiling)
            .usage(self.usage)
            .sharing_mode(self.sharing_mode)
            .initial_layout(self.initial_layout)
            .queue_family_indices(&self.queue_family_indices)
    }

    fn from_create_info_builder(value: &vk::ImageCreateInfoBuilder) -> Self {
        let dimensions =
            ImageDimensions::new_from_extent_and_layers(value.extent, value.array_layers);

        let mut queue_family_indices = Vec::<u32>::new();
        for i in 0..value.queue_family_index_count {
            let queue_family_index = unsafe { *value.p_queue_family_indices.offset(i as isize) };
            queue_family_indices.push(queue_family_index);
        }

        Self {
            mip_levels: value.mip_levels,
            samples: value.samples,
            tiling: value.tiling,
            sharing_mode: value.sharing_mode,
            queue_family_indices,
            initial_layout: value.initial_layout,
            flags: value.flags,
            format: value.format,
            dimensions,
            usage: value.usage,
        }
    }
}

// Helper Functions

/// Returns a depth stencil format guarenteed by the vulkan spec to be supported as a depth stencil
/// attachment. Prefers VK_FORMAT_D24_UNORM_S8_UINT.
///
/// According to the [vulkan spec](https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap47.html#formats-properties):
///
/// _VK_FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT feature must be supported for at least one of_ ...
/// _VK_FORMAT_D24_UNORM_S8_UINT and VK_FORMAT_D32_SFLOAT_S8_UINT._
pub fn guaranteed_depth_stencil_format(physical_device: &PhysicalDevice) -> vk::Format {
    let d24_s8_props = unsafe {
        physical_device
            .instance()
            .inner()
            .get_physical_device_format_properties(
                physical_device.handle(),
                vk::Format::D24_UNORM_S8_UINT,
            )
    };

    if d24_s8_props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
    {
        return vk::Format::D24_UNORM_S8_UINT;
    } else {
        return vk::Format::D32_SFLOAT_S8_UINT;
    }
}

/// Returns a pure depth format guarenteed by the vulkan spec to be supported as a depth stencil
/// attachment. Prefers VK_FORMAT_D32_SFLOAT.
///
/// According to the [vulkan spec](https://registry.khronos.org/vulkan/specs/1.3-extensions/html/chap47.html#formats-properties):
///
/// _VK_FORMAT_FEATURE_DEPTH_STENCIL_ATTACHMENT_BIT feature must be supported for at least one of
/// VK_FORMAT_X8_D24_UNORM_PACK32 and VK_FORMAT_D32_SFLOAT_
pub fn guaranteed_pure_depth_format(physical_device: &PhysicalDevice) -> vk::Format {
    let d32_props = unsafe {
        physical_device
            .instance()
            .inner()
            .get_physical_device_format_properties(physical_device.handle(), vk::Format::D32_SFLOAT)
    };

    if d32_props
        .optimal_tiling_features
        .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
    {
        return vk::Format::D32_SFLOAT;
    } else {
        return vk::Format::X8_D24_UNORM_PACK32;
    }
}

pub fn extent_2d_from_width_height(dimensions: [u32; 2]) -> vk::Extent2D {
    vk::Extent2D {
        width: dimensions[0],
        height: dimensions[1],
    }
}

/// Doesn't support planes/metadata.
pub fn aspect_mask_from_format(format: vk::Format) -> vk::ImageAspectFlags {
    let mut aspect = vk::ImageAspectFlags::empty();

    if !matches!(
        format,
        vk::Format::D16_UNORM
            | vk::Format::X8_D24_UNORM_PACK32
            | vk::Format::D32_SFLOAT
            | vk::Format::S8_UINT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    ) {
        aspect |= vk::ImageAspectFlags::COLOR;
    }

    if matches!(
        format,
        vk::Format::D16_UNORM
            | vk::Format::X8_D24_UNORM_PACK32
            | vk::Format::D32_SFLOAT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    ) {
        aspect |= vk::ImageAspectFlags::DEPTH;
    }

    if matches!(
        format,
        vk::Format::S8_UINT
            | vk::Format::D16_UNORM_S8_UINT
            | vk::Format::D24_UNORM_S8_UINT
            | vk::Format::D32_SFLOAT_S8_UINT
    ) {
        aspect |= vk::ImageAspectFlags::STENCIL;
    }

    aspect
}
