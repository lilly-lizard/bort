use crate::{Device, DeviceError, DeviceOwned, Fence};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct Queue {
    handle: vk::Queue,
    family_index: u32,
    queue_index: u32,

    // dependencies
    device: Arc<Device>,
}

impl Queue {
    /// Uses vkGetDeviceQueue
    pub fn new(
        device: Arc<Device>,
        family_index: u32,
        queue_index: u32,
    ) -> Result<Self, QueueError> {
        let handle = unsafe { device.inner().get_device_queue(family_index, queue_index) };
        if handle == vk::Queue::null() {
            return Err(QueueError::NoQueueHandle {
                device_handle: device.inner().handle(),
                family_index,
                queue_index,
            });
        }

        Ok(Self {
            handle,
            family_index,
            queue_index,
            device,
        })
    }

    /// Uses vkGetDeviceQueue2
    pub fn new_v2(device: Arc<Device>, queue_info: vk::DeviceQueueInfo2Builder) -> Self {
        let handle = unsafe { device.inner().get_device_queue2(&queue_info) };

        Self {
            handle,
            family_index: queue_info.queue_family_index,
            queue_index: queue_info.queue_index,
            device,
        }
    }

    pub fn submit<'a>(
        &self,
        submit_infos: impl IntoIterator<Item = vk::SubmitInfoBuilder<'a>>,
        fence: Option<&Fence>,
    ) -> VkResult<()> {
        let vk_submit_infos: Vec<vk::SubmitInfo> = submit_infos
            .into_iter()
            .map(|submit_info| submit_info.build())
            .collect();
        let fence_handle = fence.map(|f| f.handle());

        unsafe {
            self.device.inner().queue_submit(
                self.handle,
                &vk_submit_infos,
                fence_handle.unwrap_or_default(),
            )
        }
    }

    pub fn wait_idle(&self) -> Result<(), DeviceError> {
        self.device.queue_wait_idle(self)
    }

    // Getters

    pub fn handle(&self) -> vk::Queue {
        self.handle
    }

    pub fn family_index(&self) -> u32 {
        self.family_index
    }

    pub fn queue_index(&self) -> u32 {
        self.queue_index
    }
}

impl DeviceOwned for Queue {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

// ~~ Errors ~~

#[derive(Debug, Clone, Copy)]
pub enum QueueError {
    NoQueueHandle {
        device_handle: vk::Device,
        family_index: u32,
        queue_index: u32,
    },
}

impl std::fmt::Display for QueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::NoQueueHandle {
                device_handle,
                family_index,
                queue_index,
            } => write!(
                f,
                "queue family {} and index {} not found in device {}",
                family_index,
                queue_index,
                device_handle.as_raw()
            ),
        }
    }
}

impl std::error::Error for QueueError {}
