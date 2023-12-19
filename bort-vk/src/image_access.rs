use crate::{DeviceOwned, ImageDimensions, MemoryError};
use ash::vk;
use std::{error, fmt};

// ~~ Image Access ~~

/// Unifies different types of images
pub trait ImageAccess: DeviceOwned + Send + Sync {
    fn handle(&self) -> vk::Image;
    fn dimensions(&self) -> ImageDimensions;
}

// ~~ Image Access Error ~~

#[derive(Debug, Clone)]
pub enum ImageAccessError {
    /// `requested_coordinates` and `image_dimensions` are different enum variants
    IncompatibleDimensions {
        requested_coordinates: ImageDimensions,
        image_dimensions: ImageDimensions,
    },
    InvalidDimensions {
        requested_coordinates: ImageDimensions,
        image_dimensions: ImageDimensions,
    },
    MemoryError(MemoryError),
}

impl fmt::Display for ImageAccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemoryError(e) => e.fmt(f),
            Self::IncompatibleDimensions {
                requested_coordinates,
                image_dimensions,
            } => {
                write!(
                    f,
                    "incompatible coordinate/dimension types: requested coordinates = {:?}, image dimensions = {:?}",
                    requested_coordinates, image_dimensions
                )
            }
            Self::InvalidDimensions {
                requested_coordinates,
                image_dimensions,
            } => {
                write!(
                    f,
                    "invalid coordinates/dimensions: requested coordinates = {:?}, image dimensions = {:?}",
                    requested_coordinates, image_dimensions
                )
            }
        }
    }
}

impl error::Error for ImageAccessError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::MemoryError(e) => Some(e),
            Self::IncompatibleDimensions { .. } => None,
            Self::InvalidDimensions { .. } => None,
        }
    }
}
