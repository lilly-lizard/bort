use crate::{Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

#[derive(Clone)]
pub struct RenderPass {
    handle: vk::RenderPass,
    properties: RenderPassProperties,

    // dependencies
    device: Arc<Device>,
}

impl RenderPass {
    pub fn new(
        device: Arc<Device>,
        attachment_descriptions: Vec<vk::AttachmentDescription>,
        subpasses: Vec<Subpass>,
        subpass_dependencies: Vec<vk::SubpassDependency>,
    ) -> VkResult<Self> {
        let subpass_descriptions: Vec<vk::SubpassDescription> = subpasses
            .iter()
            .map(|subpass| subpass.subpass_description())
            .collect();
        let render_pass_info = vk::RenderPassCreateInfo::default()
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

    /// # Safety
    ///
    /// For each subpass:
    /// - if `subpass_description.p_color_attachments` is not null it must point to an array with
    ///   `subpass_description.color_attachment_count` many elements.
    /// - if `subpass_description.p_input_attachments` is not null it must point to an array with
    ///   `subpass_description.input_attachment_count` many elements.
    pub unsafe fn new_from_create_info(
        device: Arc<Device>,
        create_info: vk::RenderPassCreateInfo,
    ) -> VkResult<Self> {
        let properties = unsafe { RenderPassProperties::from_create_info(&create_info) };

        let handle = unsafe {
            device
                .inner()
                .create_render_pass(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::RenderPass {
        self.handle
    }

    #[inline]
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

#[derive(Debug, Default, Clone)]
pub struct RenderPassProperties {
    pub attachment_descriptions: Vec<vk::AttachmentDescription>,
    pub subpasses: Vec<Subpass>,
    pub subpass_dependencies: Vec<vk::SubpassDependency>,
}

impl RenderPassProperties {
    /// # Safety
    ///
    /// For each subpass:
    /// - if `subpass_description.p_color_attachments` is not null it must point to an array with
    ///   `subpass_description.color_attachment_count` many elements.
    /// - if `subpass_description.p_input_attachments` is not null it must point to an array with
    ///   `subpass_description.input_attachment_count` many elements.
    pub unsafe fn from_create_info(create_info: &vk::RenderPassCreateInfo) -> Self {
        let mut attachment_descriptions = Vec::<vk::AttachmentDescription>::new();
        if !create_info.p_attachments.is_null() {
            for i in 0..create_info.attachment_count {
                let vk_attachment = unsafe { *create_info.p_attachments.offset(i as isize) };
                attachment_descriptions.push(vk_attachment);
            }
        }

        let mut subpasses = Vec::<Subpass>::new();
        if !create_info.p_subpasses.is_null() {
            for i in 0..create_info.subpass_count {
                let vk_subpass = unsafe { *create_info.p_subpasses.offset(i as isize) };
                let subpass = unsafe { Subpass::from_subpass_description(&vk_subpass) };
                subpasses.push(subpass);
            }
        }

        let mut subpass_dependencies = Vec::<vk::SubpassDependency>::new();
        if !create_info.p_dependencies.is_null() {
            for i in 0..create_info.dependency_count {
                let vk_dependency = unsafe { *create_info.p_dependencies.offset(i as isize) };
                subpass_dependencies.push(vk_dependency);
            }
        }

        Self {
            attachment_descriptions,
            subpasses,
            subpass_dependencies,
        }
    }
}

#[derive(Debug, Default, Clone)]
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

    /// # Safety
    ///
    /// - if `subpass_description.p_color_attachments` is not null it must point to an array with
    ///   `subpass_description.color_attachment_count` many elements.
    /// - if `subpass_description.p_input_attachments` is not null it must point to an array with
    ///   `subpass_description.input_attachment_count` many elements.
    pub unsafe fn from_subpass_description(subpass_description: &vk::SubpassDescription) -> Self {
        let mut color_attachments = Vec::<vk::AttachmentReference>::new();
        if !subpass_description.p_color_attachments.is_null() {
            for i in 0..subpass_description.color_attachment_count {
                let vk_attachment =
                    unsafe { *subpass_description.p_color_attachments.offset(i as isize) };
                color_attachments.push(vk_attachment);
            }
        }

        let depth_attachment: Option<vk::AttachmentReference> =
            if !subpass_description.p_depth_stencil_attachment.is_null() {
                let vk_attachment = *subpass_description.p_depth_stencil_attachment;
                Some(vk_attachment)
            } else {
                None
            };

        let mut input_attachments = Vec::<vk::AttachmentReference>::new();
        if !subpass_description.p_input_attachments.is_null() {
            for i in 0..subpass_description.input_attachment_count {
                let vk_attachment =
                    unsafe { *subpass_description.p_input_attachments.offset(i as isize) };
                input_attachments.push(vk_attachment);
            }
        }

        Self {
            color_attachments,
            depth_attachment,
            input_attachments,
        }
    }

    pub fn subpass_description(&self) -> vk::SubpassDescription {
        let mut subpass_description =
            vk::SubpassDescription::default().pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

        if self.color_attachments.len() > 0 {
            subpass_description = subpass_description.color_attachments(&self.color_attachments);
        }
        if self.input_attachments.len() > 0 {
            subpass_description = subpass_description.input_attachments(&self.input_attachments);
        }
        if let Some(depth_attachment) = &self.depth_attachment {
            subpass_description = subpass_description.depth_stencil_attachment(depth_attachment);
        }

        subpass_description
    }
}
