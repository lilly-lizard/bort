use crate::device::Device;
use ash::vk;
use bort_vma::AllocationCreateInfo;
use std::{error, fmt, mem, ptr, sync::Arc};

pub trait AllocAccess: Send + Sync {
    fn vma_alloc_ref(&self) -> &dyn bort_vma::Alloc;
    fn device(&self) -> &Arc<Device>;

    #[inline]
    fn vma_allocator(&self) -> &bort_vma::Allocator {
        self.vma_alloc_ref().allocator()
    }
}

/// Note this doesn't impl `Drop`. Destroy this yourself! e.g. with `Buffer` and `Image` `Drop` implementations
pub struct MemoryAllocation {
    inner: bort_vma::Allocation,
    memory_type: vk::MemoryType,
    size: vk::DeviceSize,

    // dependencies
    alloc_access: Arc<dyn AllocAccess>,
}

impl MemoryAllocation {
    pub(crate) fn from_vma_allocation(
        inner: bort_vma::Allocation,
        alloc_access: Arc<dyn AllocAccess>,
    ) -> Self {
        let memory_info = alloc_access.vma_allocator().get_allocation_info(&inner);

        let size = memory_info.size;

        let physical_device_mem_props = alloc_access.device().physical_device().memory_properties();

        debug_assert!(memory_info.memory_type < physical_device_mem_props.memory_type_count);
        let memory_type = physical_device_mem_props.memory_types[memory_info.memory_type as usize];

        Self {
            inner,
            memory_type,
            size,
            alloc_access,
        }
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    /// Doesn't perform any GPU synchronization.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn write_struct<T>(
        &mut self,
        write_data: T,
        write_offset: usize,
    ) -> Result<(), MemoryError> {
        let data_size = mem::size_of_val(&write_data);

        let allocation_size = self.size as usize;
        if data_size > allocation_size - write_offset {
            return Err(MemoryError::AccessDataSize {
                data_size,
                allocation_size,
                offset: write_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let offset_mapped_memory = unsafe { mapped_memory.offset(write_offset as isize) } as *mut T;

        unsafe { ptr::write::<T>(offset_mapped_memory, write_data) };

        let flush_res = self.flush_allocation(write_offset, data_size);

        unsafe { self.unmap_memory() };

        flush_res
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    /// Doesn't perform any GPU synchronization.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn write_iter<I, T>(
        &mut self,
        write_data: I,
        write_offset: usize,
    ) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let write_data_iter = write_data.into_iter();
        let item_size = mem::size_of::<T>();
        let data_size = write_data_iter.len() * item_size;

        let allocation_size = self.size as usize;
        if data_size > allocation_size - write_offset {
            return Err(MemoryError::AccessDataSize {
                data_size,
                allocation_size,
                offset: write_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let mut offset_mapped_memory =
            unsafe { mapped_memory.offset(write_offset as isize) } as *mut T;

        unsafe {
            for element in write_data_iter {
                ptr::write::<T>(offset_mapped_memory, element);
                offset_mapped_memory = offset_mapped_memory.offset(1);
            }
        }

        let flush_res = self.flush_allocation(write_offset, data_size);

        unsafe { self.unmap_memory() };

        flush_res
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    /// Doesn't perform any GPU synchronization.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn read_struct<T>(&mut self, read_offset: usize) -> Result<T, MemoryError> {
        let data_size = mem::size_of::<T>();

        let allocation_size = self.size as usize;
        if data_size > allocation_size - read_offset {
            return Err(MemoryError::AccessDataSize {
                data_size,
                allocation_size,
                offset: read_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let offset_mapped_memory = unsafe { mapped_memory.offset(read_offset as isize) } as *mut T;

        let read_data = unsafe { ptr::read::<T>(offset_mapped_memory) };

        unsafe { self.unmap_memory() };

        Ok(read_data)
    }

    pub unsafe fn map_memory(&mut self) -> Result<*mut u8, MemoryError> {
        self.alloc_access
            .vma_allocator()
            .map_memory(&mut self.inner)
            .map_err(|e| MemoryError::Mapping(e))
    }

    pub unsafe fn unmap_memory(&mut self) {
        self.alloc_access
            .vma_allocator()
            .unmap_memory(&mut self.inner)
    }

    /// Flushes allocated memory. Note that the VMA function only runs is the memory is host
    /// visible and isn't host coherent.
    #[inline]
    pub fn flush_allocation(
        &mut self,
        write_offset: usize,
        data_size: usize,
    ) -> Result<(), MemoryError> {
        self.alloc_access
            .vma_allocator()
            .flush_allocation(&self.inner, write_offset, data_size)
            .map_err(|e| MemoryError::Flushing(e))
    }

    // Getters

    /// Access the `bort_vma::Allocation` handle that `self` contains.
    #[inline]
    pub fn inner(&self) -> &bort_vma::Allocation {
        &self.inner
    }

    /// Access the `bort_vma::Allocation` handle that `self` contains.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut bort_vma::Allocation {
        &mut self.inner
    }

    #[inline]
    pub fn memory_type(&self) -> vk::MemoryType {
        self.memory_type
    }

    /// Returns self as a dynamic allocation type.
    #[inline]
    pub fn alloc_access(&self) -> &Arc<dyn AllocAccess> {
        &self.alloc_access
    }

    #[inline]
    pub fn memory_property_flags(&self) -> vk::MemoryPropertyFlags {
        self.memory_type.property_flags
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.alloc_access.device()
    }
}

// Presets

/// Default `AllocationCreateInfo` with specified required and preferred flags.
pub fn allocation_info_from_flags(
    required_flags: vk::MemoryPropertyFlags,
    preferred_flags: vk::MemoryPropertyFlags,
) -> AllocationCreateInfo {
    AllocationCreateInfo {
        required_flags,
        preferred_flags,
        ..Default::default()
    }
}

/// For allocating memory that can be accessed and mapped from the cpu. Prefered flags include
/// HOST_COHERENT (doesn't require flushing). Good for staging buffers.
pub fn allocation_info_cpu_accessible() -> AllocationCreateInfo {
    // more info here: https://asawicki.info/news_1740_vulkan_memory_types_on_pc_and_how_to_use_them
    allocation_info_from_flags(
        vk::MemoryPropertyFlags::HOST_VISIBLE,
        vk::MemoryPropertyFlags::HOST_COHERENT,
    )
}

// Memory Error

#[derive(Debug, Clone)]
pub enum MemoryError {
    Mapping(vk::Result),
    AccessDataSize {
        data_size: usize,
        allocation_size: usize,
        offset: usize,
    },
    Flushing(vk::Result),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(e) => {
                write!(f, "failed map allocation memory: {}", e)
            }
            Self::AccessDataSize {
                data_size,
                allocation_size,
                offset,
            } => {
                write!(
                    f,
                    "invalid data size access parameters: data size = {}, allocation size = {}, write offset = {}",
                    data_size, allocation_size, offset
                )
            }
            Self::Flushing(e) => write!(f, "failed to flush memory: {}", e),
        }
    }
}

impl error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Mapping(e) => Some(e),
            Self::AccessDataSize { .. } => None,
            Self::Flushing(e) => Some(e),
        }
    }
}
