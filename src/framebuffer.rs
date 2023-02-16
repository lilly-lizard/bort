use crate::{
    device::Device, image_properties::ImageDimensions, image_view::ImageViewAccess,
    memory::ALLOCATION_CALLBACK_NONE, render_pass::RenderPass,
};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct Framebuffer {
    handle: vk::Framebuffer,

    // dependencies
    render_pass: Arc<RenderPass>,
}

impl Framebuffer {
    pub fn new(
        render_pass: Arc<RenderPass>,
        mut framebuffer_properties: FramebufferProperties,
    ) -> VkResult<Self> {
        let framebuffer_info_builder = framebuffer_properties.create_info_builder(&render_pass);

        let handle = unsafe {
            render_pass
                .device()
                .inner()
                .create_framebuffer(&framebuffer_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            render_pass,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::Framebuffer {
        self.handle
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.render_pass.device()
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device()
                .inner()
                .destroy_framebuffer(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

pub struct FramebufferProperties {
    pub create_flags: vk::FramebufferCreateFlags,
    pub attachments: Vec<Arc<dyn ImageViewAccess>>,
    pub dimensions: ImageDimensions,
    // because these need to be stored for the lifetime duration of self
    attachment_image_view_handles: Vec<vk::ImageView>,
}

impl FramebufferProperties {
    pub fn new(attachments: Vec<Arc<dyn ImageViewAccess>>, dimensions: ImageDimensions) -> Self {
        Self {
            create_flags: vk::FramebufferCreateFlags::empty(),
            attachments,
            attachment_image_view_handles: Vec::new(),
            dimensions,
        }
    }

    pub fn create_info_builder(
        &mut self,
        render_pass: &RenderPass,
    ) -> vk::FramebufferCreateInfoBuilder {
        self.attachment_image_view_handles = self
            .attachments
            .iter()
            .map(|image_view| image_view.handle())
            .collect::<Vec<_>>();

        vk::FramebufferCreateInfo::builder()
            .flags(self.create_flags)
            .render_pass(render_pass.handle())
            .attachments(self.attachment_image_view_handles.as_slice())
            .width(self.dimensions.width())
            .height(self.dimensions.height())
            .layers(self.dimensions.array_layers())
    }
}

impl Default for FramebufferProperties {
    fn default() -> Self {
        Self {
            create_flags: Default::default(),
            attachments: Vec::new(),
            dimensions: ImageDimensions::default(),
            attachment_image_view_handles: Vec::new(),
        }
    }
}
