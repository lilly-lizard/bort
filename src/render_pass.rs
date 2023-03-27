use crate::{Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct RenderPassProperties {
    pub attachment_descriptions: Vec<vk::AttachmentDescription>,
    pub subpasses: Vec<Subpass>,
    pub subpass_dependencies: Vec<vk::SubpassDependency>,
}

pub struct RenderPass {
    handle: vk::RenderPass,
    properties: RenderPassProperties,

    // dependencies
    device: Arc<Device>,
}

impl RenderPass {
    pub fn new(
        device: Arc<Device>,
        attachment_descriptions: impl IntoIterator<Item = vk::AttachmentDescription>,
        subpasses: impl IntoIterator<Item = Subpass>,
        subpass_dependencies: impl IntoIterator<Item = vk::SubpassDependency>,
    ) -> VkResult<Self> {
        let attachment_descriptions: Vec<vk::AttachmentDescription> =
            attachment_descriptions.into_iter().collect();

        let subpasses: Vec<Subpass> = subpasses.into_iter().collect();
        let subpass_descriptions: Vec<vk::SubpassDescription> = subpasses
            .iter()
            .map(|subpass| subpass.subpass_description_builder().build())
            .collect();

        let subpass_dependencies: Vec<vk::SubpassDependency> =
            subpass_dependencies.into_iter().collect();

        let render_pass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachment_descriptions)
            .subpasses(&subpass_descriptions)
            .dependencies(&subpass_dependencies);

        let handle = unsafe {
            device
                .inner()
                .create_render_pass(&render_pass_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties: RenderPassProperties {
                attachment_descriptions,
                subpasses,
                subpass_dependencies,
            },
            device,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::RenderPass {
        self.handle
    }

    pub fn properties(&self) -> &RenderPassProperties {
        &self.properties
    }
}

impl DeviceOwned for RenderPass {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for RenderPass {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_render_pass(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Debug, Default)]
pub struct Subpass {
    pub color_attachments: Vec<vk::AttachmentReference>,
    pub depth_attachment: Option<vk::AttachmentReference>,
    pub input_attachments: Vec<vk::AttachmentReference>,
}

impl Subpass {
    pub fn new(
        color_attachments: &[vk::AttachmentReference],
        depth_attachment: Option<vk::AttachmentReference>,
        input_attachments: &[vk::AttachmentReference],
    ) -> Self {
        Self {
            color_attachments: color_attachments.into(),
            depth_attachment,
            input_attachments: input_attachments.into(),
        }
    }

    pub fn subpass_description_builder(&self) -> vk::SubpassDescriptionBuilder {
        let mut subpass_description_builder =
            vk::SubpassDescription::builder().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        if self.color_attachments.len() > 0 {
            subpass_description_builder =
                subpass_description_builder.color_attachments(&self.color_attachments);
        }
        if self.input_attachments.len() > 0 {
            subpass_description_builder =
                subpass_description_builder.input_attachments(&self.input_attachments);
        }
        if let Some(depth_attachment) = &self.depth_attachment {
            subpass_description_builder =
                subpass_description_builder.depth_stencil_attachment(depth_attachment);
        }

        subpass_description_builder
    }
}
