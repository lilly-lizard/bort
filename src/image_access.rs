use crate::{DeviceOwned, ImageDimensions};
use ash::vk;
use std::sync::Arc;

pub trait ImageAccess: DeviceOwned {
    fn handle(&self) -> vk::Image;
    fn dimensions(&self) -> ImageDimensions;
}

pub trait ImageViewAccess: DeviceOwned {
    fn handle(&self) -> vk::ImageView;
    fn image_access(&self) -> Arc<dyn ImageAccess>;
}
