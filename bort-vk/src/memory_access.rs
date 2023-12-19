use crate::{Device, MemoryAllocation, MemoryError};
#[cfg(feature = "bytemuck")]
use bytemuck::{NoUninit, Pod};
use std::sync::Arc;

// ~~ Allocator Access ~~

/// Unifies different types of vma allocators
pub trait AllocatorAccess: Send + Sync {
    fn vma_alloc_ref(&self) -> &dyn bort_vma::Alloc;
    fn device(&self) -> &Arc<Device>;

    #[inline]
    fn vma_allocator(&self) -> &bort_vma::Allocator {
        self.vma_alloc_ref().allocator()
    }
}

// ~~ Allocation Access ~~

/// Allows any struct containing a memory allocation to "inherit" the read/write functions
pub trait AllocationAccess {
    fn memory_allocation_mut(&mut self) -> &mut MemoryAllocation;

    #[cfg(feature = "bytemuck")]
    fn write_into_bytes<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        self.memory_allocation_mut()
            .write_into_bytes(write_data, allocation_offset)
    }

    #[cfg(feature = "bytemuck")]
    fn write_slice<T>(
        &mut self,
        write_data: &[T],
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        self.memory_allocation_mut()
            .write_slice(write_data, allocation_offset)
    }

    fn write_bytes(
        &mut self,
        write_bytes: &[u8],
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        self.memory_allocation_mut()
            .write_bytes(write_bytes, allocation_offset)
    }

    fn write_struct<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        self.memory_allocation_mut()
            .write_struct(write_data, allocation_offset)
    }

    fn write_iter<I, T>(
        &mut self,
        write_data: I,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        self.memory_allocation_mut()
            .write_iter(write_data, allocation_offset)
    }

    #[cfg(feature = "bytemuck")]
    fn read_vec<T>(
        &mut self,
        element_count: usize,
        allocation_offset: usize,
    ) -> Result<Vec<T>, MemoryError>
    where
        T: Pod,
    {
        self.memory_allocation_mut()
            .read_vec(element_count, allocation_offset)
    }

    fn read_struct<T>(&mut self, allocation_offset: usize) -> Result<T, MemoryError> {
        self.memory_allocation_mut().read_struct(allocation_offset)
    }
}
