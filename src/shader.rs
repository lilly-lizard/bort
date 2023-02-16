use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use ash::{util::read_spv, vk};
use std::{
    error, fmt, fs,
    io::{self, Cursor},
    sync::Arc,
};

pub struct Shader {
    module_handle: vk::ShaderModule,

    // dependencies
    device: Arc<Device>,
}

impl Shader {
    pub fn new_from_file(device: Arc<Device>, file_path: &str) -> Result<Self, ShaderError> {
        let bytes = fs::read(file_path).map_err(|e| ShaderError::FileRead {
            e,
            path: file_path.to_string(),
        })?;
        let mut cursor = Cursor::new(bytes);

        Self::new_from_spirv(device, &mut cursor)
    }

    pub fn new_from_spirv<R: io::Read + io::Seek>(
        device: Arc<Device>,
        spirv: &mut R,
    ) -> Result<Self, ShaderError> {
        let code = read_spv(spirv).map_err(|e| ShaderError::SpirVDecode(e))?;
        let create_info = vk::ShaderModuleCreateInfo::builder().code(&code);

        Self::new_from_create_info(device, create_info)
    }

    pub fn new_from_create_info(
        device: Arc<Device>,
        create_info: vk::ShaderModuleCreateInfoBuilder,
    ) -> Result<Self, ShaderError> {
        let module_handle = unsafe {
            device
                .inner()
                .create_shader_module(&create_info, ALLOCATION_CALLBACK_NONE)
        }
        .map_err(|e| ShaderError::Creation(e))?;

        Ok(Self {
            module_handle,
            device,
        })
    }

    // Getters

    pub fn module_handle(&self) -> vk::ShaderModule {
        self.module_handle
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
                .destroy_shader_module(self.module_handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Debug)]
pub enum ShaderError {
    FileRead { e: io::Error, path: String },
    SpirVDecode(io::Error),
    Creation(vk::Result),
}

impl fmt::Display for ShaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FileRead { e, path } => {
                write!(f, "failed to read file {} due to: {}", path, e)
            }
            Self::SpirVDecode(e) => write!(f, "failed to decode spirv: {}", e),
            Self::Creation(e) => write!(f, "shader module creation failed: {}", e),
        }
    }
}

impl error::Error for ShaderError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::FileRead { e, .. } => Some(e),
            Self::SpirVDecode(e) => Some(e),
            Self::Creation(e) => Some(e),
        }
    }
}
