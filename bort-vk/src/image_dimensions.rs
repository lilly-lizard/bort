use ash::vk;

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
