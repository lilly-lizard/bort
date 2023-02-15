use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::{prelude::VkResult, vk};
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
            .attachments(attachment_descriptions.as_slice())
            .subpasses(subpass_descriptions.as_slice())
            .dependencies(subpass_dependencies.as_slice());

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

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
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
    pub depth_attachment: vk::AttachmentReference,
    pub input_attachments: Vec<vk::AttachmentReference>,
}

impl Subpass {
    /// If you don't have a depth attachment, just pass in `vk::AttachmentReference::default()`
    pub fn new(
        color_attachments: impl IntoIterator<Item = vk::AttachmentReference>,
        depth_attachment: vk::AttachmentReference,
        input_attachments: impl IntoIterator<Item = vk::AttachmentReference>,
    ) -> Self {
        let color_attachments: Vec<vk::AttachmentReference> =
            color_attachments.into_iter().collect();
        let input_attachments: Vec<vk::AttachmentReference> =
            input_attachments.into_iter().collect();

        Self {
            color_attachments,
            depth_attachment,
            input_attachments,
        }
    }

    pub fn subpass_description_builder(&self) -> vk::SubpassDescriptionBuilder {
        let mut subpass_description_builder =
            vk::SubpassDescription::builder().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        if self.color_attachments.len() > 0 {
            subpass_description_builder =
                subpass_description_builder.color_attachments(self.color_attachments.as_slice());
        }
        if self.input_attachments.len() > 0 {
            subpass_description_builder =
                subpass_description_builder.color_attachments(self.input_attachments.as_slice());
        }
        subpass_description_builder =
            subpass_description_builder.depth_stencil_attachment(&self.depth_attachment);

        subpass_description_builder
    }
}
