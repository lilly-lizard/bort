use crate::{
    render_pass::RenderPass, Device, DeviceOwned, ImageDimensions, ImageViewAccess,
    ALLOCATION_CALLBACK_NONE,
};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct Framebuffer {
    handle: vk::Framebuffer,
    properties: FramebufferProperties,

    // dependencies
    render_pass: Arc<RenderPass>,
}

impl Framebuffer {
    pub fn new(
        render_pass: Arc<RenderPass>,
        mut properties: FramebufferProperties,
    ) -> VkResult<Self> {
        let framebuffer_info_builder = properties.create_info_builder(&render_pass);

        let handle = unsafe {
            render_pass
                .device()
                .inner()
                .create_framebuffer(&framebuffer_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            render_pass,
        })
    }

    pub fn whole_rect(&self) -> vk::Rect2D {
        vk::Rect2D {
            extent: vk::Extent2D {
                width: self.properties.dimensions.width(),
                height: self.properties.dimensions.height(),
            },
            offset: vk::Offset2D { x: 0, y: 0 },
        }
    }

    pub fn whole_viewport(&self) -> vk::Viewport {
        self.properties.dimensions.whole_viewport()
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::Framebuffer {
        self.handle
    }

    #[inline]
    pub fn properties(&self) -> &FramebufferProperties {
        &self.properties
    }
}

impl DeviceOwned for Framebuffer {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.render_pass.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
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
    pub flags: vk::FramebufferCreateFlags,
    pub attachments: Vec<Arc<dyn ImageViewAccess>>,
    pub dimensions: ImageDimensions,
    // because these need to be stored for the lifetime duration of self
    attachment_image_view_handles: Vec<vk::ImageView>,
}

impl FramebufferProperties {
    pub fn new(attachments: Vec<Arc<dyn ImageViewAccess>>, dimensions: ImageDimensions) -> Self {
        Self {
            flags: vk::FramebufferCreateFlags::empty(),
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
            .flags(self.flags)
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
            flags: Default::default(),
            attachments: Vec::new(),
            dimensions: ImageDimensions::default(),
            attachment_image_view_handles: Vec::new(),
        }
    }
}
