use crate::{device::Device, AllocAccess};
use ash::prelude::VkResult;
use bort_vma::AllocatorCreateInfo;
use std::sync::Arc;

/// so it's easy to find all allocation callback args, just in case I want to use them in the future.
pub const ALLOCATION_CALLBACK_NONE: Option<&ash::vk::AllocationCallbacks> = None;

// Memory Allocator

pub struct MemoryAllocator {
    inner: Arc<bort_vma::Allocator>,

    // dependencies
    device: Arc<Device>,
}

impl MemoryAllocator {
    pub fn new(device: Arc<Device>) -> VkResult<Self> {
        let allocator_info = AllocatorCreateInfo::new(
            device.instance().inner(),
            device.inner(),
            device.physical_device().handle(),
        );
        let inner = Arc::new(bort_vma::Allocator::new(allocator_info)?);

        Ok(Self { inner, device })
    }

    // Getters

    /// Access the `bort_vma::Allocator` struct that `self` contains. Allows you to access vma allocator
    /// functions.
    #[inline]
    pub fn inner(&self) -> &bort_vma::Allocator {
        &self.inner
    }

    /// Needed because of the way `bort_vma::AllocatorPool` is implemented.
    #[inline]
    pub(crate) fn inner_arc(&self) -> &Arc<bort_vma::Allocator> {
        &self.inner
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl AllocAccess for MemoryAllocator {
    #[inline]
    fn vma_alloc_ref(&self) -> &dyn bort_vma::Alloc {
        self.inner.as_ref()
    }

    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }
}
