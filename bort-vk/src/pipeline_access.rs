use crate::{DeviceOwned, PipelineLayout, Refc};
use ash::vk;

/// Unifies different types of pipeline
#[cfg(not(feature = "rc"))]
pub trait PipelineAccess: DeviceOwned + Send + Sync {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout(&self) -> &Refc<PipelineLayout>;
    fn bind_point(&self) -> vk::PipelineBindPoint;
}
#[cfg(feature = "rc")]
pub trait PipelineAccess: DeviceOwned {
    fn handle(&self) -> vk::Pipeline;
    fn pipeline_layout(&self) -> &Refc<PipelineLayout>;
    fn bind_point(&self) -> vk::PipelineBindPoint;
}
