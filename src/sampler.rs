use crate::{Device, DeviceOwned, ALLOCATION_CALLBACK_NONE};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct Sampler {
    handle: vk::Sampler,
    properties: SamplerProperties,

    // dependencies
    device: Arc<Device>,
}

impl Sampler {
    pub fn new(device: Arc<Device>, properties: SamplerProperties) -> VkResult<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_sampler(&properties.create_info_builder(), ALLOCATION_CALLBACK_NONE)
        }?;

        Ok(Self {
            handle,
            properties,
            device,
        })
    }

    // Getters

    pub fn handle(&self) -> vk::Sampler {
        self.handle
    }

    pub fn properties(&self) -> &SamplerProperties {
        &self.properties
    }
}

impl DeviceOwned for Sampler {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.device
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            self.device
                .inner()
                .destroy_sampler(self.handle, ALLOCATION_CALLBACK_NONE);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SamplerProperties {
    pub flags: vk::SamplerCreateFlags,
    pub mag_filter: vk::Filter,
    pub min_filter: vk::Filter,
    pub mipmap_mode: vk::SamplerMipmapMode,
    pub address_mode: [vk::SamplerAddressMode; 3],
    pub mip_lod_bias: f32,
    pub max_anisotropy: Option<f32>,
    pub compare_op: Option<vk::CompareOp>,
    pub min_lod: f32,
    pub max_lod: f32,
    pub border_color: vk::BorderColor,
    pub unnormalized_coordinates: bool,
}

impl Default for SamplerProperties {
    fn default() -> Self {
        Self {
            flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            address_mode: [vk::SamplerAddressMode::CLAMP_TO_EDGE; 3],
            mip_lod_bias: 0.,
            max_anisotropy: None,
            compare_op: None,
            min_lod: 0.,
            max_lod: vk::LOD_CLAMP_NONE,
            border_color: vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
            unnormalized_coordinates: false,
        }
    }
}

impl SamplerProperties {
    pub fn create_info_builder(&self) -> vk::SamplerCreateInfoBuilder {
        vk::SamplerCreateInfo::builder()
            .flags(self.flags)
            .mag_filter(self.mag_filter)
            .min_filter(self.min_filter)
            .mipmap_mode(self.mipmap_mode)
            .address_mode_u(self.address_mode[0])
            .address_mode_v(self.address_mode[1])
            .address_mode_w(self.address_mode[2])
            .mip_lod_bias(self.mip_lod_bias)
            .anisotropy_enable(self.max_anisotropy.is_some())
            .max_anisotropy(self.max_anisotropy.unwrap_or(0.))
            .compare_enable(self.compare_op.is_some())
            .compare_op(self.compare_op.unwrap_or(vk::CompareOp::NEVER))
            .min_lod(self.min_lod)
            .max_lod(self.max_lod)
            .border_color(self.border_color)
            .unnormalized_coordinates(self.unnormalized_coordinates)
    }
}
