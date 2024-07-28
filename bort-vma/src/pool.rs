use std::ffi::CStr;
use std::sync::Arc;

use crate::ffi;
use crate::Allocator;
use crate::PoolCreateInfo;
use ash::prelude::VkResult;
#[derive(Clone, Copy)]
pub struct PoolHandle(pub ffi::VmaPool);

/// Represents custom memory pool handle.
pub struct AllocatorPool {
    pub(crate) allocator: Arc<Allocator>,
    pub(crate) pool: PoolHandle,
}
unsafe impl Send for AllocatorPool {}
unsafe impl Sync for AllocatorPool {}

impl AllocatorPool {
    pub fn new(allocator: Arc<Allocator>, create_info: &PoolCreateInfo) -> VkResult<Self> {
        unsafe {
            let mut ffi_pool: ffi::VmaPool = std::mem::zeroed();
            ffi::vmaCreatePool(allocator.internal, &create_info.inner, &mut ffi_pool).result()?;

            Ok(AllocatorPool {
                pool: PoolHandle(ffi_pool),
                allocator,
            })
        }
    }

    pub fn set_name(&self, name: Option<&CStr>) {
        if self.pool.0.is_null() {
            return;
        }
        unsafe {
            ffi::vmaSetPoolName(
                self.allocator.internal,
                self.pool.0,
                name.map_or(std::ptr::null(), CStr::as_ptr),
            );
        }
    }

    pub fn name(&self) -> Option<&CStr> {
        if self.pool.0.is_null() {
            return None;
        }
        let mut ptr: *const ::std::os::raw::c_char = std::ptr::null();
        unsafe {
            ffi::vmaGetPoolName(self.allocator.internal, self.pool.0, &mut ptr);
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
            ffi::vmaGetPoolStatistics(self.allocator.internal, self.pool.0, &mut pool_stats);
            Ok(pool_stats)
        }
    }

    /// Retrieves statistics of existing `AllocatorPool` object.
    pub fn calculate_statistics(&self) -> VkResult<ffi::VmaDetailedStatistics> {
        unsafe {
            let mut pool_stats: ffi::VmaDetailedStatistics = std::mem::zeroed();
            ffi::vmaCalculatePoolStatistics(self.allocator.internal, self.pool.0, &mut pool_stats);
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
        unsafe { ffi::vmaCheckPoolCorruption(self.allocator.internal, self.pool.0).result() }
    }
}

impl Drop for AllocatorPool {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaDestroyPool(self.allocator.internal, self.pool.0);
        }
    }
}
