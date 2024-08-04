use crate::{AllocatorAccess, Device, MemoryAllocator};
use ash::{prelude::VkResult, vk};
use bort_vma::ffi;
use std::{ffi::CStr, sync::Arc};

pub struct MemoryPool {
    handle: ffi::VmaPool,
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

        let handle = unsafe {
            let mut ffi_pool: ffi::VmaPool = std::mem::zeroed();
            ffi::vmaCreatePool(memory_allocator.handle(), &create_info, &mut ffi_pool).result()?;
            ffi_pool
        };

        Ok(Self {
            handle,
            properties,
            memory_allocator,
        })
    }

    pub fn set_name(&self, name: Option<&CStr>) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            ffi::vmaSetPoolName(
                self.memory_allocator.handle(),
                self.handle,
                name.map_or(std::ptr::null(), CStr::as_ptr),
            );
        }
    }

    pub fn name(&self) -> Option<&CStr> {
        if self.handle.is_null() {
            return None;
        }
        let mut ptr: *const ::std::os::raw::c_char = std::ptr::null();
        unsafe {
            ffi::vmaGetPoolName(self.memory_allocator.handle(), self.handle, &mut ptr);
            if ptr.is_null() {
                return None;
            }
            Some(CStr::from_ptr(ptr))
        }
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub fn get_statistics(&self) -> VkResult<ffi::VmaStatistics> {
        unsafe {
            let mut pool_stats: ffi::VmaStatistics = std::mem::zeroed();
            ffi::vmaGetPoolStatistics(self.memory_allocator.handle(), self.handle, &mut pool_stats);
            Ok(pool_stats)
        }
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub fn calculate_statistics(&self) -> VkResult<ffi::VmaDetailedStatistics> {
        unsafe {
            let mut pool_stats: ffi::VmaDetailedStatistics = std::mem::zeroed();
            ffi::vmaCalculatePoolStatistics(
                self.memory_allocator.handle(),
                self.handle,
                &mut pool_stats,
            );
            Ok(pool_stats)
        }
    }

    /// Checks magic number in margins around all allocations in given memory pool in search for corruptions.
    ///
    /// Corruption detection is enabled only when `VMA_DEBUG_DETECT_CORRUPTION` macro is defined to nonzero,
    /// `VMA_DEBUG_MARGIN` is defined to nonzero and the pool is created in memory type that is
    /// `ash::vk::MemoryPropertyFlags::HOST_VISIBLE` and `ash::vk::MemoryPropertyFlags::HOST_COHERENT`.
    ///
    /// Possible error values:
    ///
    /// - `ash::vk::Result::ERROR_FEATURE_NOT_PRESENT` - corruption detection is not enabled for specified pool.
    /// - `ash::vk::Result::ERROR_VALIDATION_FAILED_EXT` - corruption detection has been performed and found memory corruptions around one of the allocations.
    ///   `VMA_ASSERT` is also fired in that case.
    /// - Other value: Error returned by Vulkan, e.g. memory mapping failure.
    pub fn check_corruption(&self) -> VkResult<()> {
        unsafe { ffi::vmaCheckPoolCorruption(self.memory_allocator.handle(), self.handle).result() }
    }

    // Getters

    /// Access the `bort_vma::AllocatorPool` struct that `self` contains. Allows you to access pool-related
    /// vma related functions.
    #[inline]
    pub fn handle(&self) -> ffi::VmaPool {
        self.handle
    }

    #[inline]
    pub fn properties(&self) -> MemoryPoolPropeties {
        self.properties
    }
}

unsafe impl Send for MemoryPool {}
unsafe impl Sync for MemoryPool {}

impl AllocatorAccess for MemoryPool {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.memory_allocator.device()
    }

    #[inline]
    fn memory_allocator(&self) -> &MemoryAllocator {
        self.memory_allocator.as_ref()
    }

    #[inline]
    fn memory_pool_handle(&self) -> ffi::VmaPool {
        self.handle
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaDestroyPool(self.memory_allocator.handle(), self.handle);
        }
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
