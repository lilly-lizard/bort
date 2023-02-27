use crate::device::Device;
use ash::vk;

pub struct CommandBuffer {
    handle: vk::CommandBuffer,
}
