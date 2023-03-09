use crate::{Device, MemoryAllocator};
use ash::{prelude::VkResult, vk};
use std::sync::Arc;

pub struct MemoryPool {
    inner: bort_vma::AllocatorPool,
    properties: MemoryPoolPropeties,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl MemoryPool {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        create_info: &bort_vma::PoolCreateInfo,
    ) -> VkResult<Self> {
        let inner =
            bort_vma::AllocatorPool::new(memory_allocator.inner_arc().clone(), create_info)?;

        let properties = MemoryPoolPropeties::from(create_info);

        Ok(Self {
            inner,
            properties,
            memory_allocator,
        })
    }

    // Getters

    /// Access the `bort_vma::AllocatorPool` struct that `self` contains. Allows you to access pool-related
    /// vma related functions.
    #[inline]
    pub fn inner(&self) -> &bort_vma::AllocatorPool {
        &self.inner
    }

    #[inline]
    pub fn properties(&self) -> MemoryPoolPropeties {
        self.properties
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        self.memory_allocator.device()
    }
}

#[derive(Clone, Copy)]
pub struct MemoryPoolPropeties {
    pub flags: bort_vma::AllocatorPoolCreateFlags,
    pub memory_type_index: u32,
    pub block_size: vk::DeviceSize,
    pub min_block_count: usize,
    pub max_block_count: usize,
    pub priority: f32,
    pub min_allocation_alignment: vk::DeviceSize,
}

impl Default for MemoryPoolPropeties {
    fn default() -> Self {
        Self {
            flags: bort_vma::AllocatorPoolCreateFlags::empty(),
            memory_type_index: 0,
            block_size: 0,
            min_block_count: 0,
            max_block_count: 0,
            priority: 0.,
            min_allocation_alignment: 0,
        }
    }
}

impl MemoryPoolPropeties {
    pub fn create_info(&self) -> bort_vma::PoolCreateInfo {
        bort_vma::PoolCreateInfo::new()
            .flags(self.flags)
            .memory_type_index(self.memory_type_index)
            .block_size(self.block_size)
            .min_block_count(self.min_block_count)
            .max_block_count(self.max_block_count)
            .priority(self.priority)
            .min_allocation_alignment(self.min_allocation_alignment)
    }
}

impl From<&bort_vma::PoolCreateInfo<'_>> for MemoryPoolPropeties {
    fn from(value: &bort_vma::PoolCreateInfo) -> Self {
        Self {
            flags: value.get_flags(),
            memory_type_index: value.get_memory_type_index(),
            block_size: value.get_block_size(),
            min_block_count: value.get_min_block_count(),
            max_block_count: value.get_max_block_count(),
            priority: value.get_priority(),
            min_allocation_alignment: value.get_min_allocation_alignment(),
        }
    }
}
