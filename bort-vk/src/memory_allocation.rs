use crate::{device::Device, AllocatorAccess};
use ash::vk;
use bort_vma::AllocationCreateInfo;
#[cfg(feature = "bytemuck")]
use bytemuck::{NoUninit, Pod, PodCastError};
use std::{error, fmt, mem, ptr, sync::Arc};

// ~~ Memory Allocation ~~

/// Note this doesn't impl `Drop`. Destroy this yourself! e.g. with `Buffer` and `Image` `Drop` implementations
pub struct MemoryAllocation {
    inner: bort_vma::Allocation,
    memory_type: vk::MemoryType,
    size: vk::DeviceSize,

    // dependencies
    allocator_access: Arc<dyn AllocatorAccess>,
}

impl MemoryAllocation {
    pub(crate) fn from_vma_allocation(
        inner: bort_vma::Allocation,
        allocator_access: Arc<dyn AllocatorAccess>,
    ) -> Self {
        let memory_info = allocator_access.vma_allocator().get_allocation_info(&inner);

        let size = memory_info.size;

        let physical_device_mem_props = allocator_access
            .device()
            .physical_device()
            .memory_properties();

        debug_assert!(memory_info.memory_type < physical_device_mem_props.memory_type_count);
        let memory_type = physical_device_mem_props.memory_types[memory_info.memory_type as usize];

        Self {
            inner,
            memory_type,
            size,
            allocator_access,
        }
    }

    #[cfg(feature = "bytemuck")]
    pub fn write_into_bytes<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        let write_bytes = bytemuck::bytes_of(&write_data);
        self.write_bytes(write_bytes, allocation_offset)
    }

    #[cfg(feature = "bytemuck")]
    pub fn write_slice<T>(
        &mut self,
        write_data: &[T],
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        T: NoUninit,
    {
        let write_bytes: &[u8] = bytemuck::try_cast_slice(write_data)?;
        self.write_bytes(write_bytes, allocation_offset)
    }

    pub fn write_bytes(
        &mut self,
        write_bytes: &[u8],
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        let data_size = write_bytes.len();
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut u8 =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        let write_bytes_ptr = write_bytes.as_ptr();
        unsafe {
            ptr::copy_nonoverlapping(write_bytes_ptr, offset_mapped_memory, data_size);
        }

        let flush_res = self.flush_allocation(allocation_offset, data_size);
        unsafe { self.unmap_memory() };
        flush_res
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    /// Doesn't perform any GPU synchronization.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn write_struct<T>(
        &mut self,
        write_data: T,
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        let data_size = mem::size_of_val(&write_data);
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        unsafe { ptr::write::<T>(offset_mapped_memory, write_data) };

        let flush_res = self.flush_allocation(allocation_offset, data_size);
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
        allocation_offset: usize,
    ) -> Result<(), MemoryError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let write_data_iter = write_data.into_iter();
        let item_size = mem::size_of::<T>();
        let data_size = write_data_iter.len() * item_size;
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let mut offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        for element in write_data_iter {
            unsafe {
                ptr::write::<T>(offset_mapped_memory, element);
                offset_mapped_memory = offset_mapped_memory.offset(1);
            }
        }

        let flush_res = self.flush_allocation(allocation_offset, data_size);
        unsafe { self.unmap_memory() };
        flush_res
    }

    #[cfg(feature = "bytemuck")]
    pub fn read_vec<T>(
        &mut self,
        element_count: usize,
        allocation_offset: usize,
    ) -> Result<Vec<T>, MemoryError>
    where
        T: Pod,
    {
        let type_size = mem::size_of::<T>();
        let data_size = type_size * element_count;
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        let mut output_vec = Vec::<T>::new();
        output_vec.resize(element_count, T::zeroed());
        let output_vec_ptr = output_vec.as_mut_ptr();

        unsafe { ptr::copy_nonoverlapping(offset_mapped_memory, output_vec_ptr, element_count) };

        unsafe { self.unmap_memory() };
        Ok(output_vec)
    }

    /// Writes `data` to this memory allocation. Will flush if memory isn't host coherent.
    /// Doesn't perform any GPU synchronization.
    ///
    /// If memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` this will fail.
    pub fn read_struct<T>(&mut self, allocation_offset: usize) -> Result<T, MemoryError> {
        let data_size = mem::size_of::<T>();
        self.check_memory_access_parameters(data_size, allocation_offset)?;

        let offset_mapped_memory: *mut T =
            unsafe { self.map_memory_with_offset_unchecked(allocation_offset)? };

        let read_data = unsafe { ptr::read::<T>(offset_mapped_memory) };

        unsafe { self.unmap_memory() };
        Ok(read_data)
    }

    pub unsafe fn map_memory(&mut self) -> Result<*mut u8, MemoryError> {
        self.allocator_access
            .vma_allocator()
            .map_memory(&mut self.inner)
            .map_err(|e| MemoryError::Mapping(e))
    }

    pub unsafe fn unmap_memory(&mut self) {
        self.allocator_access
            .vma_allocator()
            .unmap_memory(&mut self.inner)
    }

    unsafe fn map_memory_with_offset_unchecked<T>(
        &mut self,
        allocation_offset: usize,
    ) -> Result<*mut T, MemoryError> {
        let mapped_memory = unsafe { self.map_memory() }?;
        let offset_mapped_memory =
            unsafe { mapped_memory.offset(allocation_offset as isize) } as *mut T;
        Ok(offset_mapped_memory)
    }

    /// Flushes allocated memory. Note that the VMA function only runs is the memory is host
    /// visible and isn't host coherent.
    #[inline]
    pub fn flush_allocation(
        &mut self,
        allocation_offset: usize,
        data_size: usize,
    ) -> Result<(), MemoryError> {
        self.allocator_access
            .vma_allocator()
            .flush_allocation(&self.inner, allocation_offset, data_size)
            .map_err(|e| MemoryError::Flushing(e))
    }

    fn check_memory_access_parameters(
        &self,
        data_size: usize,
        allocation_offset: usize,
    ) -> Result<(), MemoryError> {
        let allocation_size = self.size as usize;
        let allocation_write_size = allocation_size.checked_sub(allocation_offset).ok_or(
            MemoryError::AllocationOffsetTooBig {
                allocation_size,
                allocation_offset,
            },
        )?;

        if data_size > allocation_write_size {
            return Err(MemoryError::DataSizeTooBig {
                data_size,
                allocation_size,
                allocation_offset,
            });
        }

        Ok(())
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
    pub fn allocator_access(&self) -> &Arc<dyn AllocatorAccess> {
        &self.allocator_access
    }

    #[inline]
    pub fn memory_property_flags(&self) -> vk::MemoryPropertyFlags {
        self.memory_type.property_flags
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.allocator_access.device()
    }
}

// ~~ Presets ~~

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

// ~~ Memory Error ~~

#[derive(Debug, Clone)]
pub enum MemoryError {
    Mapping(vk::Result),
    DataSizeTooBig {
        data_size: usize,
        allocation_size: usize,
        allocation_offset: usize,
    },
    AllocationOffsetTooBig {
        allocation_size: usize,
        allocation_offset: usize,
    },
    Flushing(vk::Result),
    #[cfg(feature = "bytemuck")]
    PodCastError(PodCastError),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(e) => write!(f, "failed map allocation memory: {}", e),
            Self::DataSizeTooBig {
                data_size,
                allocation_size,
                allocation_offset,
            } => write!(
                f,
                "invalid data size access parameters: data size = {}, allocation size = {}, write offset = {}",
                data_size, allocation_size, allocation_offset
            ),
            Self::AllocationOffsetTooBig {
                allocation_size,
                allocation_offset,
            } => write!(f,
                "allocation offset {} is larger than the allocated memory {}",
                allocation_offset, allocation_size
            ),
            Self::Flushing(e) => write!(f, "failed to flush memory: {}", e),
            #[cfg(feature = "bytemuck")]
            Self::PodCastError(e) => write!(f, "slice cast failed: {}", e),
        }
    }
}

impl error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Mapping(e) => Some(e),
            Self::DataSizeTooBig { .. } => None,
            Self::AllocationOffsetTooBig { .. } => None,
            Self::Flushing(e) => Some(e),
            #[cfg(feature = "bytemuck")]
            Self::PodCastError(e) => Some(e),
        }
    }
}

impl From<PodCastError> for MemoryError {
    fn from(e: PodCastError) -> Self {
        Self::PodCastError(e)
    }
}
