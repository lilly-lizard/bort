use crate::{device::Device, memory::ALLOCATION_CALLBACK_NONE};
use anyhow::Context;
use ash::vk;
use std::sync::Arc;

pub struct Sampler {
    handle: vk::Sampler,
    properties: SamplerProperties,

    // dependencies
    device: Arc<Device>,
}

impl Sampler {
    pub fn new(device: Arc<Device>, properties: SamplerProperties) -> anyhow::Result<Self> {
        let handle = unsafe {
            device
                .inner()
                .create_sampler(&properties.create_info_builder(), ALLOCATION_CALLBACK_NONE)
        }
        .context("creating sampler")?;

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

    #[inline]
    pub fn device(&self) -> &Arc<Device> {
        &self.device
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
    pub create_flags: vk::SamplerCreateFlags,
    pub mag_filter: vk::Filter,
    pub min_filter: vk::Filter,
    pub mipmap_mode: vk::SamplerMipmapMode,
    pub address_mode_u: vk::SamplerAddressMode,
    pub address_mode_v: vk::SamplerAddressMode,
    pub address_mode_w: vk::SamplerAddressMode,
    pub mip_lod_bias: f32,
    pub anisotropy_enable: bool,
    pub max_anisotropy: f32,
    pub compare_enable: bool,
    pub compare_op: vk::CompareOp,
    pub min_lod: f32,
    pub max_lod: f32,
    pub border_color: vk::BorderColor,
    pub unnormalized_coordinates: bool,
}

impl Default for SamplerProperties {
    fn default() -> Self {
        Self {
            create_flags: vk::SamplerCreateFlags::empty(),
            mag_filter: vk::Filter::NEAREST,
            min_filter: vk::Filter::NEAREST,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.,
            anisotropy_enable: false,
            max_anisotropy: 0.,
            compare_enable: false,
            compare_op: vk::CompareOp::NEVER,
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
            .flags(self.create_flags)
            .mag_filter(self.mag_filter)
            .min_filter(self.min_filter)
            .mipmap_mode(self.mipmap_mode)
            .address_mode_u(self.address_mode_u)
            .address_mode_v(self.address_mode_v)
            .address_mode_w(self.address_mode_w)
            .mip_lod_bias(self.mip_lod_bias)
            .anisotropy_enable(self.anisotropy_enable)
            .max_anisotropy(self.max_anisotropy)
            .compare_enable(self.compare_enable)
            .compare_op(self.compare_op)
            .min_lod(self.min_lod)
            .max_lod(self.max_lod)
            .border_color(self.border_color)
            .unnormalized_coordinates(self.unnormalized_coordinates)
    }
}
