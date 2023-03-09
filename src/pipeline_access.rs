use crate::{Device, PipelineLayout};
use ash::vk;
use std::sync::Arc;

pub trait PipelineAccess {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout(&self) -> &Arc<PipelineLayout>;
    fn device(&self) -> &Arc<Device>;
}
