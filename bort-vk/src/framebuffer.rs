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
    pub fn new(render_pass: Arc<RenderPass>, properties: FramebufferProperties) -> VkResult<Self> {
        let vk_attachment_image_view_handles = properties.vk_attachment_image_view_handles();
        let create_info = properties.write_create_info(
            vk::FramebufferCreateInfo::default(),
            &vk_attachment_image_view_handles,
            &render_pass,
        );

        let handle = unsafe {
            render_pass
                .device()
                .inner()
                .create_framebuffer(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            render_pass,
        })
    }

    /// _Note: this fn doesn't check that the render pass handle in `create_info` is equal to
    /// that of `render_pass`._
    ///
    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        render_pass: Arc<RenderPass>,
        create_info: vk::FramebufferCreateInfo,
    ) -> VkResult<Self> {
        let properties = FramebufferProperties::from_create_info(&create_info);

        let handle = unsafe {
            render_pass
                .device()
                .inner()
                .create_framebuffer(&create_info, ALLOCATION_CALLBACK_NONE)
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
        self.render_pass.device()
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

// Note: default is empty!
#[derive(Clone, Default)]
pub struct FramebufferProperties {
    pub flags: vk::FramebufferCreateFlags,
    pub attachments: Vec<Arc<dyn ImageViewAccess>>,
    pub dimensions: ImageDimensions,
}

impl FramebufferProperties {
    pub fn new_default(
        attachments: Vec<Arc<dyn ImageViewAccess>>,
        dimensions: ImageDimensions,
    ) -> Self {
        Self {
            flags: vk::FramebufferCreateFlags::empty(),
            attachments,
            dimensions,
        }
    }

    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::FramebufferCreateInfo<'a>,
        vk_attchment_image_view_handles: &'a [vk::ImageView],
        render_pass: &RenderPass,
    ) -> vk::FramebufferCreateInfo {
        create_info
            .flags(self.flags)
            .attachments(vk_attchment_image_view_handles)
            .height(self.dimensions.height())
            .width(self.dimensions.width())
            .layers(self.dimensions.array_layers())
            .render_pass(render_pass.handle())
    }

    pub fn vk_attachment_image_view_handles(&self) -> Vec<vk::ImageView> {
        self.attachments
            .iter()
            .map(|image_view| image_view.handle())
            .collect()
    }

    /// Note: leaves `attachments` empty because the create info only provides handles
    pub fn from_create_info(value: &vk::FramebufferCreateInfo) -> Self {
        let dimensions = ImageDimensions::new_2d_array(value.width, value.height, value.layers);
        Self {
            flags: value.flags,
            attachments: Vec::new(), // because the create info only provides handles
            dimensions,
        }
    }
}
