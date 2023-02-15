use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct DescriptorSetLayout {
    handle: vk::DescriptorSetLayout,
    properties: DescriptorSetLayoutProperties,
    // note, I cbf supporting immutable samplers, but if I were to, I'd put a `Vec` of `Arc`s here.

    // dependencies
    device: Arc<Device>,
}

impl DescriptorSetLayout {
    pub fn new(
        device: Arc<Device>,
        mut properties: DescriptorSetLayoutProperties,
    ) -> anyhow::Result<Self> {
        let handle = unsafe {
            device.inner().create_descriptor_set_layout(
                &properties.create_info_builder(),
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .context("creating descriptor set layout")?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::DescriptorSetLayout {
        self.handle
    }

    pub fn properties(&self) -> &DescriptorSetLayoutProperties {
        &self.properties
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_descriptor_set_layout(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct DescriptorSetLayoutProperties {
    pub create_flags: vk::DescriptorSetLayoutCreateFlags,
    pub bindings: Vec<DescriptorSetLayoutBinding>,
    // because these need to be stored for the lifetime duration of self
    bindings_vk: Vec<vk::DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayoutProperties {
    pub fn new(
        create_flags: vk::DescriptorSetLayoutCreateFlags,
        bindings: Vec<DescriptorSetLayoutBinding>,
    ) -> Self {
        Self {
            create_flags,
            bindings,
            bindings_vk: Vec::new(),
        }
    }

    pub fn create_info_builder(&mut self) -> vk::DescriptorSetLayoutCreateInfoBuilder {
        self.bindings_vk = self
            .bindings
            .iter()
            .map(|binding| binding.builder().build())
            .collect();

        vk::DescriptorSetLayoutCreateInfo::builder()
            .flags(self.create_flags)
            .bindings(self.bindings_vk.as_slice())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
}

impl DescriptorSetLayoutBinding {
    pub fn builder(&self) -> vk::DescriptorSetLayoutBindingBuilder {
        vk::DescriptorSetLayoutBinding::builder()
            .binding(self.binding)
            .descriptor_type(self.descriptor_type)
            .descriptor_count(self.descriptor_count)
            .stage_flags(self.stage_flags)
    }
}
