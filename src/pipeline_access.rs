use crate::{DeviceOwned, PipelineLayout};
use ash::vk;
use std::sync::Arc;

pub trait PipelineAccess: DeviceOwned {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout(&self) -> &Arc<PipelineLayout>;
}
