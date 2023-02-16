use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::vk;
use std::{error, fmt, fs, io, sync::Arc};

pub struct Shader {
    handle: vk::ShaderModule,

    // dependencies
    device: Arc<Device>,
}

impl Shader {
    pub fn new_from_file(device: Arc<Device>, file_path: &str) -> Result<Self, ShaderError> {
        let bytes = fs::read(file_path).map_err(|e| ShaderError::IOError {
            e,
            path: file_path.to_string(),
        })?;
    }

    // Getters

    pub fn handle(&self) -> vk::ShaderModule {
        self.handle
    }

    pub fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_shader_module(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Debug)]
pub enum ShaderError {
    IOError { e: io::Error, path: String },
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IOError { e, path } => {
                write!(f, "failed to read file {} due to: {}", path, e)
            }
        }
    }
}

impl error::Error for ShaderError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::IOError { e, .. } => Some(e),
        }
    }
}
