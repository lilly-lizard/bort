use crate::{is_format_linear, is_format_srgb, Instance, PhysicalDevice, ALLOCATION_CALLBACK_NONE};
use ash::{extensions::khr, prelude::VkResult, vk, Entry};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::sync::Arc;

pub struct Surface {
    handle: vk::SurfaceKHR,
    surface_loader: khr::Surface,

    // dependencies
    instance: Arc<Instance>,
}

impl Surface {
    pub fn new(
        entry: &Entry,
        instance: Arc<Instance>,
        raw_display_handle: RawDisplayHandle,
        raw_window_handle: RawWindowHandle,
    ) -> VkResult<Self> {
        let handle = unsafe {
            ash_window::create_surface(
                entry,
                instance.inner(),
                raw_display_handle,
                raw_window_handle,
                ALLOCATION_CALLBACK_NONE,
            )
        }?;

        let surface_loader = khr::Surface::new(&entry, instance.inner());

        Ok(Self {
            handle,
            surface_loader,

            instance,
        })
    }

    pub fn get_physical_device_surface_support(
        &self,
        physical_device: &PhysicalDevice,
        queue_family_index: u32,
    ) -> VkResult<bool> {
        unsafe {
            self.surface_loader.get_physical_device_surface_support(
                physical_device.handle(),
                queue_family_index,
                self.handle,
            )
        }
    }

    pub fn get_physical_device_surface_capabilities(
        &self,
        physical_device: &PhysicalDevice,
    ) -> VkResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device.handle(), self.handle)
        }
    }

    pub fn get_physical_device_surface_formats(
        &self,
        physical_device: &PhysicalDevice,
    ) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device.handle(), self.handle)
        }
    }

    pub fn get_physical_device_surface_present_modes(
        &self,
        physical_device: &PhysicalDevice,
    ) -> VkResult<Vec<vk::PresentModeKHR>> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device.handle(), self.handle)
        }
    }

    // Getters

    pub fn handle(&self) -> vk::SurfaceKHR {
        self.handle
    }

    pub fn surface_loader(&self) -> &khr::Surface {
        &self.surface_loader
    }

    pub fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader
                .destroy_surface(self.handle, ALLOCATION_CALLBACK_NONE)
        };
    }
}

/// Returns the first surface format with a linear image format in the vec. Returns `None` is there's none.
pub fn get_first_srgb_surface_format(
    surface_formats: &Vec<vk::SurfaceFormatKHR>,
) -> Option<vk::SurfaceFormatKHR> {
    surface_formats
        .iter()
        .cloned()
        // use the first SRGB format we find
        .find(|vk::SurfaceFormatKHR { format, .. }| is_format_srgb(*format))
}

/// Returns the first surface format with a linear image format in the vec. Returns `None` is there's none.
pub fn get_first_linear_surface_format(
    surface_formats: &Vec<vk::SurfaceFormatKHR>,
) -> Option<vk::SurfaceFormatKHR> {
    surface_formats
        .iter()
        .cloned()
        // use the first linear format we find
        .find(|vk::SurfaceFormatKHR { format, .. }| is_format_linear(*format))
}
