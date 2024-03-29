use crate::{Device, DeviceOwned, Sampler, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct DescriptorSetLayout {
    handle: vk::DescriptorSetLayout,
    properties: DescriptorSetLayoutProperties,

    // dependencies
    device: Arc<Device>,
}

impl DescriptorSetLayout {
    pub fn new(device: Arc<Device>, properties: DescriptorSetLayoutProperties) -> VkResult<Self> {
        let mut vk_immutable_samplers = Vec::<Vec<vk::Sampler>>::new();
        let vk_layout_bindings: Vec<vk::DescriptorSetLayoutBinding> = properties
            .vk_layout_bindings(&mut vk_immutable_samplers)
            .into_iter()
            .map(|builder| builder.build())
            .collect();

        let create_info_builder = properties.write_create_info_builder(
            vk::DescriptorSetLayoutCreateInfo::builder(),
            &vk_layout_bindings,
        );

        let handle = unsafe {
            device
                .inner()
                .create_descriptor_set_layout(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        device: Arc<Device>,
        create_info_builder: vk::DescriptorSetLayoutCreateInfoBuilder,
    ) -> VkResult<Self> {
        let properties =
            DescriptorSetLayoutProperties::from_create_info_builder(&create_info_builder);

        let handle = unsafe {
            device
                .inner()
                .create_descriptor_set_layout(&create_info_builder, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::DescriptorSetLayout {
        self.handle
    }

    #[inline]
    pub fn properties(&self) -> &DescriptorSetLayoutProperties {
        &self.properties
    }
}

impl DeviceOwned for DescriptorSetLayout {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
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

// Properties

/// Note: default has no bindings!
#[derive(Default, Clone)]
pub struct DescriptorSetLayoutProperties {
    pub flags: vk::DescriptorSetLayoutCreateFlags,
    pub bindings: Vec<DescriptorSetLayoutBinding>,
}

impl DescriptorSetLayoutProperties {
    pub fn new_default(bindings: Vec<DescriptorSetLayoutBinding>) -> Self {
        Self {
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            bindings,
        }
    }

    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::DescriptorSetLayoutCreateInfoBuilder<'a>,
        vk_layout_bindings: &'a [vk::DescriptorSetLayoutBinding],
    ) -> vk::DescriptorSetLayoutCreateInfoBuilder {
        builder.flags(self.flags).bindings(vk_layout_bindings)
    }

    /// Clears `vk_immutable_samplers` and stores in it a vector of sampler handles for each
    /// binding. The returned builder struct contains references to these vectors with a
    /// lifetime of `'a`.
    #[allow(clippy::needless_range_loop)]
    pub fn vk_layout_bindings<'a>(
        &'a self,
        vk_immutable_samplers: &'a mut Vec<Vec<vk::Sampler>>,
    ) -> Vec<vk::DescriptorSetLayoutBindingBuilder<'a>> {
        vk_immutable_samplers.clear();
        let mut vk_layout_bindings = Vec::<vk::DescriptorSetLayoutBindingBuilder>::new();

        for i in 0..self.bindings.len() {
            let new_immutable_samplers = self.bindings[i].vk_immutable_samplers();
            vk_immutable_samplers.push(new_immutable_samplers);
        }

        for i in 0..self.bindings.len() {
            let vk_layout_binding = self.bindings[i].write_vk_binding_builder(
                vk::DescriptorSetLayoutBinding::builder(),
                &vk_immutable_samplers[i],
            );
            vk_layout_bindings.push(vk_layout_binding);
        }
        vk_layout_bindings
    }

    pub fn from_create_info_builder(value: &vk::DescriptorSetLayoutCreateInfoBuilder) -> Self {
        let mut bindings = Vec::<DescriptorSetLayoutBinding>::new();
        for i in 0..value.binding_count {
            let vk_binding = unsafe { *value.p_bindings.offset(i as isize) };
            let binding = DescriptorSetLayoutBinding::from_vk_binding(&vk_binding);
            bindings.push(binding);
        }

        Self {
            flags: value.flags,
            bindings,
        }
    }
}

// Descriptor set layout binding

/// Note: default values are nothing!
#[derive(Default, Clone)]
pub struct DescriptorSetLayoutBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
    pub immutable_samplers: Vec<Arc<Sampler>>,
}

impl DescriptorSetLayoutBinding {
    pub fn vk_immutable_samplers(&self) -> Vec<vk::Sampler> {
        self.immutable_samplers
            .iter()
            .map(|sampler| sampler.handle())
            .collect()
    }

    /// Note: leaves `immutable_samplers` empty because the create info only provides the handles.
    pub fn from_vk_binding(value: &vk::DescriptorSetLayoutBinding) -> Self {
        Self {
            binding: value.binding,
            descriptor_type: value.descriptor_type,
            descriptor_count: value.descriptor_count,
            stage_flags: value.stage_flags,
            immutable_samplers: Vec::new(), // because the create info only provides handles
        }
    }

    pub fn write_vk_binding_builder<'a>(
        &self,
        mut builder: vk::DescriptorSetLayoutBindingBuilder<'a>,
        vk_immutable_samplers: &'a [vk::Sampler],
    ) -> vk::DescriptorSetLayoutBindingBuilder<'a> {
        if vk_immutable_samplers.is_empty() {
            builder = builder.immutable_samplers(vk_immutable_samplers); // put before descriptor_count because calling this overrides it
        }
        builder
            .binding(self.binding)
            .descriptor_type(self.descriptor_type)
            .descriptor_count(self.descriptor_count)
            .stage_flags(self.stage_flags)
    }

    pub fn from_vk_binding_builder(value: &vk::DescriptorSetLayoutBindingBuilder) -> Self {
        Self::from_vk_binding(value)
    }
}
