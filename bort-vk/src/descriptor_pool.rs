use crate::{DescriptorSet, DescriptorSetLayout, Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct DescriptorPool {
    handle: vk::DescriptorPool,
    properties: DescriptorPoolProperties,

    // dependencies
    device: Arc<Device>,
}

impl DescriptorPool {
    pub fn new(device: Arc<Device>, properties: DescriptorPoolProperties) -> VkResult<Self> {
        let create_info = properties.create_info();

        let handle = unsafe {
            device
                .inner()
                .create_descriptor_pool(&create_info, ALLOCATION_CALLBACK_NONE)
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
        create_info: vk::DescriptorPoolCreateInfo,
    ) -> VkResult<Self> {
        let properties = DescriptorPoolProperties::from_create_info(&create_info);

        let handle = unsafe {
            device
                .inner()
                .create_descriptor_pool(&create_info, ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    pub fn allocate_descriptor_set(
        self: &Arc<Self>,
        layout: Arc<DescriptorSetLayout>,
    ) -> VkResult<DescriptorSet> {
        let layout_handles = [layout.handle()];
        let create_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.handle)
            .set_layouts(&layout_handles);

        let descriptor_set_handle =
            unsafe { self.device().inner().allocate_descriptor_sets(&create_info) }?[0];

        Ok(unsafe { DescriptorSet::from_handle(descriptor_set_handle, layout, self.clone()) })
    }

    pub fn allocate_descriptor_sets(
        self: &Arc<Self>,
        layouts: Vec<Arc<DescriptorSetLayout>>,
    ) -> VkResult<Vec<DescriptorSet>> {
        let layout_handles: Vec<vk::DescriptorSetLayout> =
            layouts.iter().map(|l| l.handle()).collect();
        let create_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.handle)
            .set_layouts(&layout_handles);

        let descriptor_set_handles =
            unsafe { self.device().inner().allocate_descriptor_sets(&create_info) }?;

        let mut descriptor_sets = Vec::<DescriptorSet>::new();
        for i in 0..layouts.len() {
            let descriptor_set = unsafe {
                DescriptorSet::from_handle(
                    descriptor_set_handles[i],
                    layouts[i].clone(),
                    self.clone(),
                )
            };
            descriptor_sets.push(descriptor_set);
        }

        Ok(descriptor_sets)
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::DescriptorPool {
        self.handle
    }

    #[inline]
    pub fn properties(&self) -> &DescriptorPoolProperties {
        &self.properties
    }
}

impl DeviceOwned for DescriptorPool {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_descriptor_pool(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

/// Note: default values are nothing!
#[derive(Default, Clone)]
pub struct DescriptorPoolProperties {
    pub flags: vk::DescriptorPoolCreateFlags,
    pub max_sets: u32,
    pub pool_sizes: Vec<vk::DescriptorPoolSize>,
}

impl DescriptorPoolProperties {
    pub fn new_default(max_sets: u32, pool_sizes: Vec<vk::DescriptorPoolSize>) -> Self {
        Self {
            max_sets,
            pool_sizes,
            ..Default::default()
        }
    }

    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::DescriptorPoolCreateInfo<'a>,
    ) -> vk::DescriptorPoolCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .max_sets(self.max_sets)
            .pool_sizes(&self.pool_sizes)
    }

    pub fn create_info(&self) -> vk::DescriptorPoolCreateInfo {
        self.write_create_info(vk::DescriptorPoolCreateInfo::default())
    }

    pub fn from_create_info(value: &vk::DescriptorPoolCreateInfo) -> Self {
        let mut pool_sizes = Vec::<vk::DescriptorPoolSize>::new();
        for i in 0..value.pool_size_count {
            let pool_size = unsafe { *value.p_pool_sizes.offset(i as isize) };
            pool_sizes.push(pool_size);
        }

        Self {
            flags: value.flags,
            max_sets: value.max_sets,
            pool_sizes,
        }
    }
}
