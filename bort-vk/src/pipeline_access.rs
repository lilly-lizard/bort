use crate::{DeviceOwned, PipelineLayout};
use ash::vk;
use std::sync::Arc;

/// Unifies different types of pipeline
pub trait PipelineAccess: DeviceOwned + Send + Sync {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout(&self) -> &Arc<PipelineLayout>;
    fn bind_point(&self) -> vk::PipelineBindPoint;
}
