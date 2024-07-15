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
        let mut vk_layout_bindings_storage: Vec<vk::DescriptorSetLayoutBinding> = Vec::new();
        let mut vk_immutable_samplers_storage: Vec<Vec<vk::Sampler>> = Vec::new();
        let create_info = properties.create_info(
            &mut vk_layout_bindings_storage,
            &mut vk_immutable_samplers_storage,
        );

        let handle = unsafe {
            device
                .inner()
                .create_descriptor_set_layout(&create_info, ALLOCATION_CALLBACK_NONE)
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
        create_info: vk::DescriptorSetLayoutCreateInfo,
    ) -> VkResult<Self> {
        let properties = DescriptorSetLayoutProperties::from_create_info(&create_info);
        let handle = unsafe {
            device
                .inner()
                .create_descriptor_set_layout(&create_info, ALLOCATION_CALLBACK_NONE)
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

    /// Clears and populates `vk_layout_bindings_storage` and `vk_immutable_samplers_storage`
    /// with data pointed to by the returned create info. `vk_layout_bindings_storage` and
    /// `vk_immutable_samplers_storage` must outlive the returned create info.
    pub fn create_info<'a>(
        &'a self,
        vk_layout_bindings_storage: &'a mut Vec<vk::DescriptorSetLayoutBinding<'a>>,
        vk_immutable_samplers_storage: &'a mut Vec<Vec<vk::Sampler>>,
    ) -> vk::DescriptorSetLayoutCreateInfo {
        *vk_layout_bindings_storage = self.vk_layout_bindings(vk_immutable_samplers_storage);
        vk::DescriptorSetLayoutCreateInfo::default()
            .flags(self.flags)
            .bindings(vk_layout_bindings_storage)
    }

    /// Clears `vk_immutable_samplers` and stores in it a vector of sampler handles for each
    /// binding. The returned create_info struct contains references to these vectors with a
    /// lifetime of `'a`.
    #[allow(clippy::needless_range_loop)]
    pub fn vk_layout_bindings<'a>(
        &'a self,
        vk_immutable_samplers_storage: &'a mut Vec<Vec<vk::Sampler>>,
    ) -> Vec<vk::DescriptorSetLayoutBinding<'a>> {
        vk_immutable_samplers_storage.clear();
        vk_immutable_samplers_storage.resize_with(self.bindings.len(), Default::default);

        let mut vk_layout_bindings: Vec<vk::DescriptorSetLayoutBinding> =
            Vec::with_capacity(self.bindings.len());
        vk_immutable_samplers_storage
            .iter_mut()
            .enumerate()
            .for_each(|(i, vk_immutable_sample_storage_ptr)| {
                let vk_layout_binding =
                    self.bindings[i].vk_binding(vk_immutable_sample_storage_ptr);
                vk_layout_bindings.push(vk_layout_binding);
            });
        vk_layout_bindings
    }

    pub fn from_create_info(value: &vk::DescriptorSetLayoutCreateInfo) -> Self {
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

    /// Clears `vk_immutable_samplers` and stores in it a vector of sampler handles for each
    /// binding. The returned create_info struct contains references to these vectors with a
    /// lifetime of `'a`.
    pub fn vk_binding<'a>(
        &'a self,
        vk_immutable_samplers_storage: &'a mut Vec<vk::Sampler>,
    ) -> vk::DescriptorSetLayoutBinding<'a> {
        let mut vk_binding = vk::DescriptorSetLayoutBinding::default();
        if !self.immutable_samplers.is_empty() {
            *vk_immutable_samplers_storage = self.vk_immutable_samplers();
            vk_binding = vk_binding.immutable_samplers(vk_immutable_samplers_storage);
        }
        vk_binding
            .binding(self.binding)
            .descriptor_type(self.descriptor_type)
            .descriptor_count(self.descriptor_count)
            .stage_flags(self.stage_flags)
    }

    pub fn vk_immutable_samplers(&self) -> Vec<vk::Sampler> {
        self.immutable_samplers
            .iter()
            .map(|sampler| sampler.handle())
            .collect()
    }
}
