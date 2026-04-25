use crate::{DeviceOwned, PipelineLayout, Refc};
use ash::vk;

/// Unifies different types of pipeline
pub trait PipelineAccess: DeviceOwned + Send + Sync {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout(&self) -> &Refc<PipelineLayout>;
    fn bind_point(&self) -> vk::PipelineBindPoint;
}
