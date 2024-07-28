use crate::{AllocatorAccess, Device, MemoryAllocator};
use ash::{prelude::VkResult, vk};
use bort_vma::ffi;
use std::sync::Arc;

pub struct MemoryPool {
    allocator: Arc<bort_vma::Allocator>,
    pool: bort_vma::PoolHandle,
    properties: MemoryPoolPropeties,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl MemoryPool {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        properties: MemoryPoolPropeties,
    ) -> VkResult<Self> {
        unsafe { Self::new_with_pnext_chain(memory_allocator, properties, None) }
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_with_pnext_chain(
        memory_allocator: Arc<MemoryAllocator>,
        properties: MemoryPoolPropeties,
        memory_allocate_next: Option<&mut ash::vk::MemoryAllocateInfo>,
    ) -> VkResult<Self> {
        let mut create_info = properties.create_info();
        if let Some(some_memory_allocate_next) = memory_allocate_next {
            create_info.pMemoryAllocateNext =
                some_memory_allocate_next as *mut ash::vk::MemoryAllocateInfo as *mut _;
        }

        unsafe {
            let vma_allocator = memory_allocator.inner();

            let mut ffi_pool: ffi::VmaPool = std::mem::zeroed();
            ffi::vmaCreatePool(vma_allocator.internal, &create_info, &mut ffi_pool).result()?;

            let pool = bort_vma::PoolHandle(ffi_pool);
        }

        Ok(Self {
            allocator,
            pool,
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

impl AllocatorAccess for MemoryPool {
    fn vma_alloc_ref(&self) -> &dyn bort_vma::Alloc {
        &self.inner
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.memory_allocator.device()
    }
}

// `bort_vma::PoolCreateInfo` would have been pretty good here (unlike the ash create_infos and
// their dangling pnext pointers) but that would require explicit lifetime specifiers for
// `MemoryPool` which propogates up every stuct containing it which is kinda a pain in the ass
// for goshenite...
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
    pub fn create_info(&self) -> ffi::VmaPoolCreateInfo {
        ffi::VmaPoolCreateInfo {
            flags: self.flags.bits(),
            memoryTypeIndex: self.memory_type_index,
            blockSize: self.block_size,
            minBlockCount: self.min_block_count,
            maxBlockCount: self.max_block_count,
            priority: self.priority,
            minAllocationAlignment: self.min_allocation_alignment,
            pMemoryAllocateNext: core::ptr::null_mut(),
        }
    }

    pub fn from_create_info(create_info: &ffi::VmaPoolCreateInfo) -> Self {
        Self {
            flags: bort_vma::AllocatorPoolCreateFlags::from_bits_retain(create_info.flags),
            memory_type_index: create_info.memoryTypeIndex,
            block_size: create_info.blockSize,
            min_block_count: create_info.minBlockCount,
            max_block_count: create_info.maxBlockCount,
            priority: create_info.priority,
            min_allocation_alignment: create_info.minAllocationAlignment,
        }
    }
}
