use std::collections::HashMap;

use crate::{
    DescriptorPool, DescriptorPoolProperties, DescriptorSetLayout, DescriptorSetLayoutProperties,
    Device, DeviceOwned, Refc,
};
use ash::{
    prelude::VkResult,
    vk::{self, DescriptorPoolSize, Handle},
};

// Note: no destructor needed. Just drop pool.
pub struct DescriptorSet {
    handle: vk::DescriptorSet,
    layout: Refc<DescriptorSetLayout>,

    // dependencies
    descriptor_pool: Refc<DescriptorPool>,
}

impl DescriptorSet {
    /// Convenience function that creates a new `DescriptorPool` and `DescriptorSetLayout` based
    /// on the requirements defined in `layout_properties`
    pub fn new_from_set_layout(
        device: Refc<Device>,
        layout_property: DescriptorSetLayoutProperties,
    ) -> VkResult<DescriptorSet> {
        let mut max_sets: u32 = 0;
        let mut pool_size_map: HashMap<vk::DescriptorType, u32> = HashMap::new();

        for binding in &layout_property.bindings {
            max_sets += binding.descriptor_count;
            let count = pool_size_map.entry(binding.descriptor_type).or_insert(0);
            *count += 1;
        }
        let descriptor_pool = create_descriptor_pool(device.clone(), max_sets, pool_size_map)?;

        let layout = Refc::new(DescriptorSetLayout::new(device.clone(), layout_property)?);
        descriptor_pool.allocate_descriptor_set(layout)
    }

    /// Convenience function that creates a new `DescriptorPool` and `DescriptorSetLayout`s based
    /// on the requirements defined in `layout_properties`
    pub fn new_from_set_layouts(
        device: Refc<Device>,
        layout_properties: Vec<DescriptorSetLayoutProperties>,
    ) -> VkResult<Vec<DescriptorSet>> {
        let mut max_sets: u32 = 0;
        let mut pool_size_map: HashMap<vk::DescriptorType, u32> = HashMap::new();

        for layout_property in &layout_properties {
            for binding in &layout_property.bindings {
                max_sets += binding.descriptor_count;
                let count = pool_size_map.entry(binding.descriptor_type).or_insert(0);
                *count += 1;
            }
        }
        let descriptor_pool = create_descriptor_pool(device.clone(), max_sets, pool_size_map)?;

        let mut layouts: Vec<Refc<DescriptorSetLayout>> = Vec::new();
        for layout_property in layout_properties {
            layouts.push(Refc::new(DescriptorSetLayout::new(
                device.clone(),
                layout_property,
            )?));
        }

        descriptor_pool.allocate_descriptor_sets(layouts)
    }

    pub fn new(
        descriptor_pool: Refc<DescriptorPool>,
        layout: Refc<DescriptorSetLayout>,
    ) -> VkResult<Self> {
        descriptor_pool.allocate_descriptor_set(layout)
    }

    /// Safetey: make sure `handle` was allocated from `descriptor_pool` using `layout`.
    pub(crate) unsafe fn from_handle(
        handle: vk::DescriptorSet,
        layout: Refc<DescriptorSetLayout>,
        descriptor_pool: Refc<DescriptorPool>,
    ) -> Self {
        Self {
            handle,
            layout,
            descriptor_pool,
        }
    }

    // Getters

    pub fn handle(&self) -> vk::DescriptorSet {
        self.handle
    }

    pub fn layout(&self) -> &Refc<DescriptorSetLayout> {
        &self.layout
    }

    #[inline]
    pub fn descriptor_pool(&self) -> &Refc<DescriptorPool> {
        &self.descriptor_pool
    }
}

fn create_descriptor_pool(
    device: Refc<Device>,
    max_sets: u32,
    pool_size_map: HashMap<vk::DescriptorType, u32>,
) -> VkResult<Refc<DescriptorPool>> {
    let pool_sizes: Vec<DescriptorPoolSize> = pool_size_map
        .iter()
        .map(|(&ty, &descriptor_count)| DescriptorPoolSize {
            ty,
            descriptor_count,
        })
        .collect();
    let descriptor_pool_properties = DescriptorPoolProperties {
        max_sets,
        pool_sizes,
        ..Default::default()
    };
    let descriptor_pool = Refc::new(DescriptorPool::new(device, descriptor_pool_properties)?);
    Ok(descriptor_pool)
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        let reset_flag_set = self
            .descriptor_pool
            .properties()
            .flags
            .contains(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET);
        if !reset_flag_set {
            return; // this set can only be freed by the pool
        }

        unsafe {
            let _res = self
                .device()
                .inner()
                .free_descriptor_sets(self.descriptor_pool.handle(), &[self.handle]);
        }
    }
}

impl DeviceOwned for DescriptorSet {
    #[inline]
    fn device(&self) -> &Refc<Device> {
        self.descriptor_pool.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}
