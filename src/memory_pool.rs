use crate::{AllocAccess, Device, MemoryAllocator};
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
        properties: MemoryPoolPropeties,
    ) -> VkResult<Self> {
        let create_info = properties.create_info();

        let inner =
            bort_vma::AllocatorPool::new(memory_allocator.inner_arc().clone(), &create_info)?;

        Ok(Self {
            inner,
            properties,
            memory_allocator,
        })
    }

    pub fn new_from_create_info(
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
}

impl AllocAccess for MemoryPool {
    fn vma_alloc_ref(&self) -> &dyn bort_vma::Alloc {
        &self.inner
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.memory_allocator.device()
    }
}

#[derive(Clone, Copy)]
pub struct MemoryPoolPropeties {
    /// Use combination of `VmaPoolCreateFlagBits`.
    pub flags: bort_vma::AllocatorPoolCreateFlags,

    /// Vulkan memory type index to allocate this pool from.
    pub memory_type_index: u32,

    /// Size of a single `VkDeviceMemory` block to be allocated as part of this pool, in bytes. Optional.
    ///
    /// Specify nonzero to set explicit, constant size of memory blocks used by this
    /// pool.
    ///
    /// Leave 0 to use default and let the library manage block sizes automatically.
    /// Sizes of particular blocks may vary.
    /// In this case, the pool will also support dedicated allocations.
    pub block_size: vk::DeviceSize,

    /// Minimum number of blocks to be always allocated in this pool, even if they stay empty.
    ///
    /// Set to 0 to have no preallocated blocks and allow the pool be completely empty.
    pub min_block_count: usize,

    /// Maximum number of blocks that can be allocated in this pool. Optional.
    ///
    /// Set to 0 to use default, which is `SIZE_MAX`, which means no limit.
    ///
    /// Set to same value as VmaPoolCreateInfo::minBlockCount to have fixed amount of memory allocated
    /// throughout whole lifetime of this pool.
    pub max_block_count: usize,

    /// A floating-point value between 0 and 1, indicating the priority of the allocations in this pool relative to other memory allocations.
    ///
    /// It is used only when #VMA_ALLOCATOR_CREATE_EXT_MEMORY_PRIORITY_BIT flag was used during creation of the #VmaAllocator object.
    /// Otherwise, this variable is ignored.
    pub priority: f32,

    /// Additional minimum alignment to be used for all allocations created from this pool. Can be 0.
    ///
    /// Leave 0 (default) not to impose any additional alignment. If not 0, it must be a power of two.
    /// It can be useful in cases where alignment returned by Vulkan by functions like `vkGetBufferMemoryRequirements` is not enough,
    /// e.g. when doing interop with OpenGL.
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
