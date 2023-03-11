use ash::vk;
use bort_vma::AllocationCreateInfo;

// Presets

/// Properties for a transient, lazily allocated image.
pub fn transient_image_info(
    dimensions: ImageDimensions,
    format: vk::Format,
    additional_usage: vk::ImageUsageFlags,
) -> (ImageProperties, AllocationCreateInfo) {
    let image_properties = ImageProperties {
        dimensions,
        format,
        usage: vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | additional_usage,
        ..ImageProperties::default()
    };

    let allocation_info = AllocationCreateInfo {
        //usage: MemoryUsage::GpuLazy,
        required_flags: vk::MemoryPropertyFlags::LAZILY_ALLOCATED
            | vk::MemoryPropertyFlags::DEVICE_LOCAL,
        ..AllocationCreateInfo::default()
    };

    (image_properties, allocation_info)
}

// Image Properties

/// WARNING `default()` values for `format`, `dimensions` and `usage` are nothing!
#[derive(Debug, Clone)]
pub struct ImageProperties {
    pub create_flags: vk::ImageCreateFlags,
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
            initial_layout: vk::ImageLayout::GENERAL,
            create_flags: vk::ImageCreateFlags::empty(),

            // nonsense defaults. make sure you override these!
            format: vk::Format::default(),
            dimensions: ImageDimensions::default(),
            usage: vk::ImageUsageFlags::empty(),
        }
    }
}

impl ImageProperties {
    pub fn new_default(
        format: vk::Format,
        dimensions: ImageDimensions,
        usage: vk::ImageUsageFlags,
        initial_layout: vk::ImageLayout,
    ) -> Self {
        Self {
            format,
            dimensions,
            usage,
            initial_layout,
            ..Self::default()
        }
    }

    pub fn create_info_builder(&self) -> vk::ImageCreateInfoBuilder {
        vk::ImageCreateInfo::builder()
            .flags(self.create_flags)
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
            .queue_family_indices(self.queue_family_indices.as_slice())
    }

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
}

impl From<&vk::ImageCreateInfo> for ImageProperties {
    fn from(value: &vk::ImageCreateInfo) -> Self {
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
            create_flags: value.flags,
            format: value.format,
            dimensions,
            usage: value.usage,
        }
    }
}

// Image Dimensions

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ImageDimensions {
    Dim1d {
        width: u32,
        array_layers: u32,
    },
    Dim2d {
        width: u32,
        height: u32,
        array_layers: u32,
    },
    Dim3d {
        width: u32,
        height: u32,
        depth: u32,
    },
}

impl ImageDimensions {
    pub fn new_from_extent_and_layers(extent_3d: vk::Extent3D, array_layers: u32) -> Self {
        if array_layers > 1 {
            if extent_3d.height > 1 {
                Self::new_2d_array(extent_3d.width, extent_3d.height, array_layers)
            } else {
                Self::new_1d_array(extent_3d.width, array_layers)
            }
        } else {
            if extent_3d.depth > 1 {
                Self::new_3d(extent_3d.width, extent_3d.height, extent_3d.depth)
            } else if extent_3d.height > 1 {
                Self::new_2d(extent_3d.width, extent_3d.height)
            } else {
                Self::new_1d(extent_3d.width)
            }
        }
    }

    pub fn new_1d(width: u32) -> Self {
        Self::Dim1d {
            width,
            array_layers: 1,
        }
    }

    pub fn new_1d_array(width: u32, array_layers: u32) -> Self {
        Self::Dim1d {
            width,
            array_layers,
        }
    }

    pub fn new_2d(width: u32, height: u32) -> Self {
        Self::Dim2d {
            width,
            height,
            array_layers: 1,
        }
    }

    pub fn new_2d_array(width: u32, height: u32, array_layers: u32) -> Self {
        Self::Dim2d {
            width,
            height,
            array_layers,
        }
    }

    pub fn new_3d(width: u32, height: u32, depth: u32) -> Self {
        Self::Dim3d {
            width,
            height,
            depth,
        }
    }

    pub fn width(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { width, .. } => width,
            ImageDimensions::Dim2d { width, .. } => width,
            ImageDimensions::Dim3d { width, .. } => width,
        }
    }

    pub fn height(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { .. } => 1,
            ImageDimensions::Dim2d { height, .. } => height,
            ImageDimensions::Dim3d { height, .. } => height,
        }
    }

    pub fn width_height(&self) -> [u32; 2] {
        [self.width(), self.height()]
    }

    pub fn depth(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { .. } => 1,
            ImageDimensions::Dim2d { .. } => 1,
            ImageDimensions::Dim3d { depth, .. } => depth,
        }
    }

    pub fn extent_3d(&self) -> vk::Extent3D {
        vk::Extent3D {
            width: self.width(),
            height: self.height(),
            depth: self.depth(),
        }
    }

    pub fn array_layers(&self) -> u32 {
        match *self {
            ImageDimensions::Dim1d { array_layers, .. } => array_layers,
            ImageDimensions::Dim2d { array_layers, .. } => array_layers,
            ImageDimensions::Dim3d { .. } => 1,
        }
    }

    pub fn num_texels(&self) -> u32 {
        self.width() * self.height() * self.depth() * self.array_layers()
    }

    pub fn image_type(&self) -> vk::ImageType {
        match *self {
            ImageDimensions::Dim1d { .. } => vk::ImageType::TYPE_1D,
            ImageDimensions::Dim2d { .. } => vk::ImageType::TYPE_2D,
            ImageDimensions::Dim3d { .. } => vk::ImageType::TYPE_3D,
        }
    }

    pub fn default_image_view_type(&self) -> vk::ImageViewType {
        match self {
            Self::Dim1d {
                array_layers: 1, ..
            } => vk::ImageViewType::TYPE_1D,
            Self::Dim1d { .. } => vk::ImageViewType::TYPE_1D_ARRAY,
            Self::Dim2d {
                array_layers: 1, ..
            } => vk::ImageViewType::TYPE_2D,
            Self::Dim2d { .. } => vk::ImageViewType::TYPE_2D_ARRAY,
            Self::Dim3d { .. } => vk::ImageViewType::TYPE_3D,
        }
    }

    pub fn whole_viewport(&self) -> vk::Viewport {
        vk::Viewport {
            x: 0.,
            y: 0.,
            width: self.width() as f32,
            height: self.height() as f32,
            min_depth: 0.,
            max_depth: 1., // not to be confused with `self.depth()`
        }
    }
}

impl Default for ImageDimensions {
    fn default() -> Self {
        Self::Dim1d {
            width: 1,
            array_layers: 1,
        }
    }
}

pub fn extent_2d_from_width_height(dimensions: [u32; 2]) -> vk::Extent2D {
    vk::Extent2D {
        width: dimensions[0],
        height: dimensions[1],
    }
}

// Helper Functions

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
