use crate::{Device, ImageDimensions};
use ash::vk;
use std::sync::Arc;

pub trait ImageAccess {
    fn handle(&self) -> vk::Image;
    fn dimensions(&self) -> ImageDimensions;
    fn device(&self) -> &Arc<Device>;
}

pub trait ImageViewAccess {
    fn handle(&self) -> vk::ImageView;
    fn image_access(&self) -> Arc<dyn ImageAccess>;
    fn device(&self) -> &Arc<Device>;
}
