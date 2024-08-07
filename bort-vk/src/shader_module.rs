use crate::{Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    util::read_spv,
    vk::{self, Handle},
};
use std::{
    error,
    ffi::CString,
    fmt, fs,
    io::{self, Cursor},
    sync::Arc,
};

pub struct ShaderModule {
    handle: vk::ShaderModule,

    // dependencies
    device: Arc<Device>,
}

impl ShaderModule {
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
        let code = read_spv(spirv).map_err(ShaderError::SpirVDecode)?;
        let create_info = vk::ShaderModuleCreateInfo::default().code(&code);

        unsafe { Self::new_from_create_info(device, create_info) }
    }

    /// # Safety
    /// Make sure your `p_next` chain contains valid pointers.
    pub unsafe fn new_from_create_info(
        device: Arc<Device>,
        create_info: vk::ShaderModuleCreateInfo,
    ) -> Result<Self, ShaderError> {
        let handle = unsafe {
            device
                .inner()
                .create_shader_module(&create_info, ALLOCATION_CALLBACK_NONE)
        }
        .map_err(ShaderError::Creation)?;

        Ok(Self { handle, device })
    }

    // Getters

    #[inline]
    pub fn handle(&self) -> vk::ShaderModule {
        self.handle
    }
}

impl DeviceOwned for ShaderModule {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for ShaderModule {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_shader_module(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

// Shader Stage

// Note: this isn't a member of `GraphicsPipelineProperties` because we only need to ensure
// the `ShaderModule` lifetime lasts during pipeline creation. Not needed after that.
#[derive(Clone)]
pub struct ShaderStage<'a> {
    pub flags: vk::PipelineShaderStageCreateFlags,
    pub stage: vk::ShaderStageFlags,
    pub module: Arc<ShaderModule>,
    pub entry_point: CString,
    pub write_specialization_info: bool,
    pub specialization_info: vk::SpecializationInfo<'a>,
}

impl<'a> ShaderStage<'a> {
    pub fn new(
        stage: vk::ShaderStageFlags,
        module: Arc<ShaderModule>,
        entry_point: CString,
        specialization_info: Option<vk::SpecializationInfo<'a>>,
    ) -> Self {
        Self {
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage,
            module,
            entry_point,
            write_specialization_info: specialization_info.is_some(),
            specialization_info: specialization_info.unwrap_or_default(),
        }
    }

    pub fn write_create_info<'b>(
        &'b self,
        create_info: vk::PipelineShaderStageCreateInfo<'b>,
    ) -> vk::PipelineShaderStageCreateInfo {
        let create_info = create_info
            .flags(self.flags)
            .module(self.module.handle())
            .stage(self.stage)
            .name(self.entry_point.as_c_str());
        if self.write_specialization_info {
            create_info.specialization_info(&self.specialization_info)
        } else {
            create_info
        }
    }

    pub fn create_info(&self) -> vk::PipelineShaderStageCreateInfo {
        self.write_create_info(vk::PipelineShaderStageCreateInfo::default())
    }
}

// Errors

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
