use crate::{
    device::Device, image::Image, image_base::ImageBase, image_properties::ImageDimensions,
    memory::ALLOCATION_CALLBACK_NONE, render_pass::RenderPass,
};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct Framebuffer {
    handle: vk::Framebuffer,

    // dependencies
    render_pass: Arc<RenderPass>,
}

impl Framebuffer {
    pub fn new(
        render_pass: Arc<RenderPass>,
        framebuffer_properties: FramebufferProperties,
    ) -> anyhow::Result<Self> {
        let framebuffer_info_builder = framebuffer_properties.create_info_builder(&render_pass);

        let handle = unsafe {
            render_pass
                .device()
                .inner()
                .create_framebuffer(&framebuffer_info_builder, ALLOCATION_CALLBACK_NONE)
        }
        .context("creating framebuffer")?;

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
    pub attachments: Vec<Arc<Image>>,
    pub dimensions: ImageDimensions,
    attachment_image_view_handles: Vec<vk::ImageView>,
}

impl FramebufferProperties {
    pub fn new(
        create_flags: vk::FramebufferCreateFlags,
        attachments: Vec<Arc<Image>>,
        dimensions: ImageDimensions,
    ) -> Self {
        let attachment_image_view_handles = attachments
            .iter()
            .map(|image| image.image_view_handle())
            .collect::<Vec<_>>();

        Self {
            create_flags,
            attachments,
            attachment_image_view_handles,
            dimensions,
        }
    }

    pub fn create_info_builder(
        &self,
        render_pass: &RenderPass,
    ) -> vk::FramebufferCreateInfoBuilder {
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
