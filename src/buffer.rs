use crate::{device::Device, memory::MemoryAllocator};
use ash::{prelude::VkResult, vk};
use std::{error, fmt, mem, ptr, sync::Arc};
use vk_mem::{Alloc, AllocationCreateInfo};

pub struct Buffer {
    handle: vk::Buffer,
    buffer_properties: BufferProperties,

    memory_allocation: vk_mem::Allocation,
    memory_type: vk::MemoryType,

    // dependencies
    memory_allocator: Arc<MemoryAllocator>,
}

impl Buffer {
    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        buffer_properties: BufferProperties,
        allocation_info: AllocationCreateInfo,
    ) -> VkResult<Self> {
        let (handle, memory_allocation) = unsafe {
            memory_allocator
                .inner()
                .create_buffer(&buffer_properties.create_info_builder(), &allocation_info)
        }?;

        Ok(Self::with_handle_and_allocation(
            memory_allocator,
            buffer_properties,
            handle,
            memory_allocation,
        ))
    }

    pub fn with_handle_and_allocation(
        memory_allocator: Arc<MemoryAllocator>,
        mut buffer_properties: BufferProperties,
        handle: vk::Buffer,
        memory_allocation: vk_mem::Allocation,
    ) -> Self {
        let memory_info = memory_allocator
            .inner()
            .get_allocation_info(&memory_allocation);

        buffer_properties.size = memory_info.size; // should be the same, but just in case

        let physical_device_mem_props = memory_allocator
            .device()
            .physical_device()
            .memory_properties();

        debug_assert!(memory_info.memory_type < physical_device_mem_props.memory_type_count);
        let memory_type = physical_device_mem_props.memory_types[memory_info.memory_type as usize];

        Self {
            handle,
            buffer_properties,
            memory_allocation,
            memory_type,
            memory_allocator,
        }
    }

    /// Note that if memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` writing will fial
    pub fn write_struct<T>(&mut self, data: T, buffer_offset: usize) -> Result<(), BufferError> {
        let data_size = mem::size_of_val(&data);

        let buffer_size = self.buffer_properties.size as usize;
        if data_size > buffer_size - buffer_offset {
            return Err(BufferError::WriteDataSize {
                data_size,
                buffer_size,
                buffer_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let offset_mapped_memory =
            unsafe { mapped_memory.offset(buffer_offset as isize) } as *mut T;

        unsafe { ptr::write::<T>(offset_mapped_memory, data) };

        let flush_res = self.flush_allocation(buffer_offset, data_size);
        if let Err(e) = flush_res {
            unsafe { self.unmap_memory() };
            return Err(e);
        }

        unsafe { self.unmap_memory() };

        Ok(())
    }

    /// Note that if memory wasn't created with `vk::MemoryPropertyFlags::HOST_VISIBLE` writing will fial
    pub fn write_iter<I, T>(&mut self, data: I, buffer_offset: usize) -> Result<(), BufferError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
    {
        let data = data.into_iter();
        let item_size = mem::size_of::<T>();
        let data_size = data.len() * item_size;

        let buffer_size = self.buffer_properties.size as usize;
        if data_size > buffer_size - buffer_offset {
            return Err(BufferError::WriteDataSize {
                data_size,
                buffer_size,
                buffer_offset,
            });
        }

        let mapped_memory = unsafe { self.map_memory() }?;
        let mut offset_mapped_memory =
            unsafe { mapped_memory.offset(buffer_offset as isize) } as *mut T;

        unsafe {
            for element in data {
                ptr::write::<T>(offset_mapped_memory, element);
                offset_mapped_memory = offset_mapped_memory.offset(1);
            }
        }

        let flush_res = self.flush_allocation(buffer_offset, data_size);
        if let Err(e) = flush_res {
            unsafe { self.unmap_memory() };
            return Err(e);
        }

        unsafe { self.unmap_memory() };

        Ok(())
    }

    pub unsafe fn map_memory(&mut self) -> Result<*mut u8, BufferError> {
        self.memory_allocator
            .inner()
            .map_memory(&mut self.memory_allocation)
            .map_err(|e| BufferError::Mapping(e))
    }

    pub unsafe fn unmap_memory(&mut self) {
        self.memory_allocator
            .inner()
            .unmap_memory(&mut self.memory_allocation)
    }

    pub fn flush_allocation(
        &mut self,
        buffer_offset: usize,
        data_size: usize,
    ) -> Result<(), BufferError> {
        // don't need to flush if memory is host coherent
        if !self
            .memory_property_flags()
            .contains(vk::MemoryPropertyFlags::HOST_COHERENT)
        {
            self.memory_allocator
                .inner()
                .flush_allocation(&self.memory_allocation, buffer_offset, data_size)
                .map_err(|e| BufferError::Flushing(e))?;
        }

        Ok(())
    }

    // Getters

    pub fn handle(&self) -> vk::Buffer {
        self.handle
    }

    pub fn buffer_properties(&self) -> &BufferProperties {
        &self.buffer_properties
    }

    pub fn memory_allocator(&self) -> &Arc<MemoryAllocator> {
        &self.memory_allocator
    }

    pub fn memory_allocation(&self) -> &vk_mem::Allocation {
        &self.memory_allocation
    }

    pub fn memory_allocation_mut(&mut self) -> &mut vk_mem::Allocation {
        &mut self.memory_allocation
    }

    pub fn memory_type(&self) -> vk::MemoryType {
        self.memory_type
    }

    #[inline]
    pub fn memory_property_flags(&self) -> vk::MemoryPropertyFlags {
        self.memory_type.property_flags
    }

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.memory_allocator.device()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.memory_allocator
                .inner()
                .destroy_buffer(self.handle, &mut self.memory_allocation);
        }
    }
}

#[derive(Clone)]
pub struct BufferProperties {
    pub create_flags: vk::BufferCreateFlags,
    pub size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
    pub sharing_mode: vk::SharingMode,
    pub queue_family_indices: Vec<u32>,
}

impl Default for BufferProperties {
    fn default() -> Self {
        Self {
            create_flags: vk::BufferCreateFlags::empty(),
            size: 0,
            usage: vk::BufferUsageFlags::empty(),
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            queue_family_indices: Vec::new(),
        }
    }
}

impl BufferProperties {
    pub fn create_info_builder(&self) -> vk::BufferCreateInfoBuilder {
        vk::BufferCreateInfo::builder()
            .flags(self.create_flags)
            .size(self.size)
            .usage(self.usage)
            .sharing_mode(self.sharing_mode)
            .queue_family_indices(self.queue_family_indices.as_slice())
    }
}

#[derive(Debug, Clone)]
pub enum BufferError {
    Mapping(vk::Result),
    WriteDataSize {
        data_size: usize,
        buffer_size: usize,
        buffer_offset: usize,
    },
    Flushing(vk::Result),
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mapping(e) => {
                write!(f, "failed map buffer memory: {}", e)
            }
            Self::WriteDataSize {
                data_size,
                buffer_size,
                buffer_offset,
            } => {
                write!(
                    f,
                    "invalid data size to be written: data size = {}, buffer size = {}, buffer offset = {}",
                    data_size, buffer_size, buffer_offset
                )
            }
            Self::Flushing(e) => write!(f, "failed to flush memory: {}", e),
        }
    }
}

impl error::Error for BufferError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Mapping(e) => Some(e),
            Self::WriteDataSize { .. } => None,
            Self::Flushing(e) => Some(e),
        }
    }
}
