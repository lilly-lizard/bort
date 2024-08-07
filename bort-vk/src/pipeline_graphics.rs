use crate::{
    Device, DeviceOwned, PipelineAccess, PipelineCache, PipelineLayout, RenderPass, ShaderStage,
    ALLOCATION_CALLBACK_NONE,
};
use ash::{
    prelude::VkResult,
    vk::{self, Handle},
};
use std::sync::Arc;

pub struct GraphicsPipeline {
    handle: vk::Pipeline,
    properties: GraphicsPipelineProperties,

    // dependencies
    pipeline_layout: Arc<PipelineLayout>,
    // note: we don't need to store references to `ShaderModule`, `RenderPass` or `PipelineCache` as per https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#fundamentals-objectmodel-lifetime
}

impl GraphicsPipeline {
    pub fn new(
        pipeline_layout: Arc<PipelineLayout>,
        properties: GraphicsPipelineProperties,
        shader_stages: &[ShaderStage],
        render_pass: &RenderPass,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Self> {
        // populate vkPipelineShaderStageCreateInfo
        let shader_stages_vk = shader_stages
            .iter()
            .map(|stage| stage.create_info())
            .collect::<Vec<vk::PipelineShaderStageCreateInfo>>();

        // populate the sub-structs of vkGraphicsPipelineCreateInfo defined by GraphicsPipelineProperties
        let properties_vk = properties.vk_create_infos();

        // use these and other args to populate the fields of vkGraphicsPipelineCreateInfo
        let create_info =
            properties.write_create_info(vk::GraphicsPipelineCreateInfo::default(), &properties_vk);
        let create_info = create_info
            .stages(&shader_stages_vk)
            .render_pass(render_pass.handle())
            .layout(pipeline_layout.handle());

        let cache_handle = if let Some(pipeline_cache) = pipeline_cache {
            pipeline_cache.handle()
        } else {
            vk::PipelineCache::null()
        };

        let handle_res = unsafe {
            pipeline_layout.device().inner().create_graphics_pipelines(
                cache_handle,
                &[create_info],
                ALLOCATION_CALLBACK_NONE,
            )
        };
        // note: cbf taking VK_PIPELINE_COMPILE_REQUIRED into account rn...
        let handle = handle_res.map_err(|(_pipelines, err_code)| err_code)?[0];

        Ok(Self {
            handle,
            properties,
            pipeline_layout,
        })
    }

    /// # Safety
    /// Requirements found in docs for the following:
    ///   - [`VertexInputState::from_create_info`]
    ///   - [`ViewportState::from_create_info`]
    ///   - [`MultisampleState::from_create_info`]
    ///   - [`ColorBlendState::from_create_info`]
    ///   - [`DynamicState::from_create_info`]
    pub unsafe fn new_from_create_info(
        pipeline_layout: Arc<PipelineLayout>,
        create_info: vk::GraphicsPipelineCreateInfo,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Self> {
        let properties = unsafe { GraphicsPipelineProperties::from_create_info(&create_info) };

        let cache_handle = if let Some(pipeline_cache) = pipeline_cache {
            pipeline_cache.handle()
        } else {
            vk::PipelineCache::null()
        };

        let handle_res = unsafe {
            pipeline_layout.device().inner().create_graphics_pipelines(
                cache_handle,
                &[create_info],
                ALLOCATION_CALLBACK_NONE,
            )
        };
        // note: cbf taking VK_PIPELINE_COMPILE_REQUIRED into account rn...
        let handle = handle_res.map_err(|(_pipelines, err_code)| err_code)?[0];

        Ok(Self {
            handle,
            properties,
            pipeline_layout,
        })
    }

    #[allow(clippy::needless_range_loop)]
    pub fn new_batch_create(
        device: &Device,
        per_pipeline_params: Vec<PerPipelineCreationParams>,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Vec<Self>> {
        let pipeline_count = per_pipeline_params.len();

        // populate the sub-structs of vkGraphicsPipelineCreateInfo defined by GraphicsPipelineProperties
        let mut pipeline_properties_vk = Vec::<GraphicsPipelinePropertiesCreateInfosVk>::new();
        for pipeline_index in 0..pipeline_count {
            let properties_vk = per_pipeline_params[pipeline_index]
                .properties
                .vk_create_infos();
            pipeline_properties_vk.push(properties_vk);
        }

        // populate the vkPipelineShaderStageCreateInfo structs
        let mut shader_stage_handles = Vec::<Vec<vk::PipelineShaderStageCreateInfo>>::new();
        for pipeline_index in 0..pipeline_count {
            let shader_stages_vk: Vec<vk::PipelineShaderStageCreateInfo> = per_pipeline_params
                [pipeline_index]
                .shader_stages
                .iter()
                .map(|stage| stage.create_info())
                .collect();
            shader_stage_handles.push(shader_stages_vk);
        }

        // use these and other args to populate the fields of vkGraphicsPipelineCreateInfo
        let mut create_infos = Vec::<vk::GraphicsPipelineCreateInfo>::new();
        for pipeline_index in 0..pipeline_count {
            let create_info = per_pipeline_params[pipeline_index]
                .properties
                .write_create_info(
                    vk::GraphicsPipelineCreateInfo::default(),
                    &pipeline_properties_vk[pipeline_index],
                );

            let create_info = create_info
                .stages(&shader_stage_handles[pipeline_index])
                .render_pass(per_pipeline_params[pipeline_index].render_pass.handle())
                .layout(per_pipeline_params[pipeline_index].pipeline_layout.handle());

            create_infos.push(create_info);
        }

        let create_infos: Vec<vk::GraphicsPipelineCreateInfo> = create_infos
            .into_iter()
            .map(|create_info| create_info)
            .collect();

        let cache_handle = if let Some(pipeline_cache) = pipeline_cache {
            pipeline_cache.handle()
        } else {
            vk::PipelineCache::null()
        };

        let pipeline_handles = unsafe {
            device.inner().create_graphics_pipelines(
                cache_handle,
                &create_infos,
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .map_err(|(_pipelines, err_code)| err_code)?; // note: cbf taking VK_PIPELINE_COMPILE_REQUIRED into account...

        let pipelines: Vec<GraphicsPipeline> = per_pipeline_params
            .into_iter()
            .enumerate()
            .map(|(index, params)| Self {
                handle: pipeline_handles[index],
                properties: params.properties,
                pipeline_layout: params.pipeline_layout.clone(),
            })
            .collect();

        Ok(pipelines)
    }

    // Getters

    #[inline]
    pub fn properties(&self) -> &GraphicsPipelineProperties {
        &self.properties
    }
}

impl PipelineAccess for GraphicsPipeline {
    #[inline]
    fn handle(&self) -> vk::Pipeline {
        self.handle
    }

    #[inline]
    fn pipeline_layout(&self) -> &Arc<PipelineLayout> {
        &self.pipeline_layout
    }

    #[inline]
    fn bind_point(&self) -> vk::PipelineBindPoint {
        vk::PipelineBindPoint::GRAPHICS
    }
}

impl DeviceOwned for GraphicsPipeline {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        self.pipeline_layout.device()
    }

    #[inline]
    fn handle_raw(&self) -> u64 {
        self.handle.as_raw()
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device()
                .inner()
                .destroy_pipeline(self.handle, ALLOCATION_CALLBACK_NONE)
        }
    }
}

// Properties

/// Allows usage of `from_create_info_ptr`
trait FromCreateInfo<VkCreateInfo> {
    fn from_create_info(value: &VkCreateInfo) -> Self;
}

/// If `vk_create_info_ptr` isn't null, this function dereferences it then returns the result of
/// `from_create_info`. Otherwise returns `Default::default()`.
///
/// # Safety
/// - if `vk_create_info_ptr` is not null, it must point to something
unsafe fn from_create_info_ptr<Properties, VkCreateInfo>(
    vk_create_info_ptr: *const VkCreateInfo,
) -> Properties
where
    Properties: FromCreateInfo<VkCreateInfo> + Default,
    VkCreateInfo: Copy + Clone,
{
    if !vk_create_info_ptr.is_null() {
        let vk_create_info = unsafe { *vk_create_info_ptr };
        Properties::from_create_info(&vk_create_info)
    } else {
        Default::default()
    }
}

/// <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkGraphicsPipelineCreateInfo.html>
///
/// Note: doesn't include shader stages, render pass, pipeline layout or pipeline cache
#[derive(Debug, Clone, Default)]
pub struct GraphicsPipelineProperties {
    pub flags: vk::PipelineCreateFlags,
    pub subpass_index: u32,
    pub vertex_input_state: VertexInputState,
    pub input_assembly_state: InputAssemblyState,
    pub tessellation_state: TessellationState,
    pub viewport_state: ViewportState,
    pub rasterization_state: RasterizationState,
    pub multisample_state: MultisampleState,
    pub depth_stencil_state: DepthStencilState,
    pub color_blend_state: ColorBlendState,
    pub dynamic_state: DynamicState,
}
impl GraphicsPipelineProperties {
    /// Returns the `create_info` arg containing references to the structs in `properties_vk`.
    ///
    /// Note: this doesn't populate:
    /// - `layout`
    /// - `render_pass`
    /// - `subpass`
    /// - `stages`
    /// - `base_pipeline_handle`
    /// - `base_pipeline_index`
    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::GraphicsPipelineCreateInfo<'a>,
        properties_vk: &'a GraphicsPipelinePropertiesCreateInfosVk<'a>,
    ) -> vk::GraphicsPipelineCreateInfo<'a> {
        // pass references to `properties_vk` to `create_info` to populate relevant members
        create_info
            .flags(self.flags)
            .subpass(self.subpass_index)
            .vertex_input_state(&properties_vk.vertex_input_state_vk)
            .input_assembly_state(&properties_vk.input_assembly_state_vk)
            .tessellation_state(&properties_vk.tessellation_state_vk)
            .viewport_state(&properties_vk.viewport_state_vk)
            .rasterization_state(&properties_vk.rasterization_state_vk)
            .multisample_state(&properties_vk.multisample_state_vk)
            .depth_stencil_state(&properties_vk.depth_stencil_state_vk)
            .color_blend_state(&properties_vk.color_blend_state_vk)
            .dynamic_state(&properties_vk.dynamic_state_vk)
    }

    /// Returns a set of vk::*CreateInfo structs populated by the members of `self`.
    /// Use this with `Self::write_create_info` to populate a `GraphicsPipelineCreateInfo`.
    pub fn vk_create_infos(&self) -> GraphicsPipelinePropertiesCreateInfosVk {
        GraphicsPipelinePropertiesCreateInfosVk {
            // write vk create-info for each member to `properties_vk`
            vertex_input_state_vk: self.vertex_input_state.create_info(),
            input_assembly_state_vk: self.input_assembly_state.create_info(),
            tessellation_state_vk: self.tessellation_state.create_info(),
            viewport_state_vk: self.viewport_state.create_info(),
            rasterization_state_vk: self.rasterization_state.create_info(),
            multisample_state_vk: self.multisample_state.create_info(),
            depth_stencil_state_vk: self.depth_stencil_state.create_info(),
            color_blend_state_vk: self.color_blend_state.create_info(),
            dynamic_state_vk: self.dynamic_state.create_info(),
        }
    }

    /// # Safety
    /// - if any of the following members aren't null they must point to something!
    ///     - `p_vertex_input_state`
    ///     - `p_input_assembly_state`
    ///     - `p_tessellation_state`
    ///     - `p_viewport_state`
    ///     - `p_rasterization_state`
    ///     - `p_multisample_state`
    ///     - `p_depth_stencil_state`
    ///     - `p_color_blend_state`
    ///     - `p_dynamic_state`
    /// - see docs for the following functions for additional safety requirements:
    ///     - [`VertexInputState::from_create_info_ptr`]
    ///     - [`ViewportState::from_create_info_ptr`]
    ///     - [`MultisampleState::from_create_info_ptr`]
    ///     - [`ColorBlendState::from_create_info_ptr`]
    ///     - [`DynamicState::from_create_info_ptr`]
    pub unsafe fn from_create_info(value: &vk::GraphicsPipelineCreateInfo) -> Self {
        let vertex_input_state = if !value.p_vertex_input_state.is_null() {
            let vk_create_info = unsafe { *value.p_vertex_input_state };
            VertexInputState::from_create_info(&vk_create_info)
        } else {
            Default::default()
        };
        let input_assembly_state = from_create_info_ptr(value.p_input_assembly_state);
        let tessellation_state = from_create_info_ptr(value.p_tessellation_state);
        let viewport_state = if !value.p_viewport_state.is_null() {
            let vk_create_info = unsafe { *value.p_viewport_state };
            ViewportState::from_create_info(&vk_create_info)
        } else {
            Default::default()
        };
        let rasterization_state = from_create_info_ptr(value.p_rasterization_state);
        let multisample_state = if !value.p_multisample_state.is_null() {
            let vk_create_info = unsafe { *value.p_multisample_state };
            MultisampleState::from_create_info(&vk_create_info)
        } else {
            Default::default()
        };
        let depth_stencil_state = from_create_info_ptr(value.p_depth_stencil_state);
        let color_blend_state = if !value.p_color_blend_state.is_null() {
            let vk_create_info = unsafe { *value.p_color_blend_state };
            ColorBlendState::from_create_info(&vk_create_info)
        } else {
            Default::default()
        };
        let dynamic_state = if !value.p_dynamic_state.is_null() {
            let vk_create_info = unsafe { *value.p_dynamic_state };
            DynamicState::from_create_info(&vk_create_info)
        } else {
            Default::default()
        };

        Self {
            flags: value.flags,
            subpass_index: value.subpass,
            vertex_input_state,
            input_assembly_state,
            tessellation_state,
            viewport_state,
            rasterization_state,
            multisample_state,
            depth_stencil_state,
            color_blend_state,
            dynamic_state,
        }
    }
}

// Sub-Properties

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineColorBlendAttachmentState.html>"]
#[derive(Debug, Clone)]
pub struct ColorBlendState {
    pub flags: vk::PipelineColorBlendStateCreateFlags,
    pub logic_op: Option<vk::LogicOp>,
    pub attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    pub blend_constants: [f32; 4],
}
impl Default for ColorBlendState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineColorBlendStateCreateFlags::empty(),
            logic_op: None,
            attachments: Vec::new(),
            blend_constants: [0.; 4],
        }
    }
}
impl ColorBlendState {
    pub fn new_default(attachments: Vec<vk::PipelineColorBlendAttachmentState>) -> Self {
        Self {
            attachments,
            ..Default::default()
        }
    }

    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::PipelineColorBlendStateCreateInfo<'a>,
    ) -> vk::PipelineColorBlendStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .logic_op_enable(self.logic_op.is_some())
            .logic_op(self.logic_op.unwrap_or(vk::LogicOp::CLEAR))
            .attachments(&self.attachments)
            .blend_constants(self.blend_constants)
    }

    pub fn create_info(&self) -> vk::PipelineColorBlendStateCreateInfo {
        self.write_create_info(vk::PipelineColorBlendStateCreateInfo::default())
    }

    // Presets

    pub fn blend_state_disabled() -> vk::PipelineColorBlendAttachmentState {
        vk::PipelineColorBlendAttachmentState {
            color_write_mask: vk::ColorComponentFlags::RGBA,
            ..Default::default()
        }
    }

    /// Returns `vk::PipelineColorBlendAttachmentState` where the output of the fragment shader is ignored and the
    /// destination is untouched.
    pub fn blend_state_ignore_source() -> vk::PipelineColorBlendAttachmentState {
        vk::PipelineColorBlendAttachmentState {
            blend_enable: 1,
            color_blend_op: vk::BlendOp::ADD,
            src_color_blend_factor: vk::BlendFactor::ZERO,
            src_alpha_blend_factor: vk::BlendFactor::DST_COLOR,
            alpha_blend_op: vk::BlendOp::ADD,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            dst_alpha_blend_factor: vk::BlendFactor::DST_COLOR,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }
    }

    /// Returns `vk::PipelineColorBlendAttachmentState` where the colors are added, and alpha is set to the maximum of
    /// the two.
    pub fn blend_state_additive() -> vk::PipelineColorBlendAttachmentState {
        vk::PipelineColorBlendAttachmentState {
            blend_enable: 1,
            color_blend_op: vk::BlendOp::ADD,
            src_color_blend_factor: vk::BlendFactor::ONE,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            alpha_blend_op: vk::BlendOp::MAX,
            dst_color_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ONE,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        }
    }

    /// # Safety
    /// - if `p_attachments` is not null, it must point to an array of `attachment_count` values
    pub unsafe fn from_create_info(value: &vk::PipelineColorBlendStateCreateInfo) -> Self {
        let mut attachments = Vec::<vk::PipelineColorBlendAttachmentState>::new();
        for i in 0..value.attachment_count {
            let attachment_state = unsafe { *value.p_attachments.offset(i as isize) };
            attachments.push(attachment_state);
        }

        let logic_op = if value.logic_op_enable == 0 {
            None
        } else {
            Some(value.logic_op)
        };

        Self {
            flags: value.flags,
            logic_op,
            attachments,
            blend_constants: value.blend_constants,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineDepthStencilStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct DepthStencilState {
    pub flags: vk::PipelineDepthStencilStateCreateFlags,
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: vk::CompareOp,
    pub depth_bounds_test_enable: bool,
    pub stencil_test_enable: bool,
    pub front: vk::StencilOpState,
    pub back: vk::StencilOpState,
    pub min_depth_bounds: f32,
    pub max_depth_bounds: f32,
}
impl Default for DepthStencilState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineDepthStencilStateCreateFlags::empty(),
            depth_test_enable: false,
            depth_write_enable: false,
            depth_compare_op: vk::CompareOp::ALWAYS,
            depth_bounds_test_enable: false,
            stencil_test_enable: false,
            front: vk::StencilOpState::default(),
            back: vk::StencilOpState::default(),
            min_depth_bounds: 0.,
            max_depth_bounds: 0.,
        }
    }
}
impl DepthStencilState {
    pub fn write_create_info<'a>(
        &self,
        create_info: vk::PipelineDepthStencilStateCreateInfo<'a>,
    ) -> vk::PipelineDepthStencilStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .depth_test_enable(self.depth_test_enable)
            .depth_write_enable(self.depth_write_enable)
            .depth_compare_op(self.depth_compare_op)
            .depth_bounds_test_enable(self.depth_bounds_test_enable)
            .stencil_test_enable(self.depth_bounds_test_enable)
            .front(self.front)
            .back(self.back)
            .min_depth_bounds(self.min_depth_bounds)
            .max_depth_bounds(self.max_depth_bounds)
    }

    pub fn create_info(&self) -> vk::PipelineDepthStencilStateCreateInfo {
        self.write_create_info(vk::PipelineDepthStencilStateCreateInfo::default())
    }
}
impl<'a> FromCreateInfo<vk::PipelineDepthStencilStateCreateInfo<'a>> for DepthStencilState {
    fn from_create_info(value: &vk::PipelineDepthStencilStateCreateInfo) -> Self {
        Self {
            flags: value.flags,
            depth_test_enable: value.depth_test_enable != 0,
            depth_write_enable: value.depth_write_enable != 0,
            depth_compare_op: value.depth_compare_op,
            depth_bounds_test_enable: value.depth_bounds_test_enable != 0,
            stencil_test_enable: value.stencil_test_enable != 0,
            front: value.front,
            back: value.back,
            min_depth_bounds: value.min_depth_bounds,
            max_depth_bounds: value.max_depth_bounds,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineDynamicStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct DynamicState {
    pub flags: vk::PipelineDynamicStateCreateFlags,
    pub dynamic_states: Vec<vk::DynamicState>,
}
impl Default for DynamicState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_states: Vec::new(),
        }
    }
}
impl DynamicState {
    pub fn new_default(dynamic_states: Vec<vk::DynamicState>) -> Self {
        Self {
            dynamic_states,
            ..Default::default()
        }
    }

    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::PipelineDynamicStateCreateInfo<'a>,
    ) -> vk::PipelineDynamicStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .dynamic_states(&self.dynamic_states)
    }

    pub fn create_info(&self) -> vk::PipelineDynamicStateCreateInfo {
        self.write_create_info(vk::PipelineDynamicStateCreateInfo::default())
    }

    /// # Safety
    /// - if `p_dynamic_states` is not null, it must point to an array of `dynamic_state_count` values
    pub unsafe fn from_create_info(value: &vk::PipelineDynamicStateCreateInfo) -> Self {
        let mut dynamic_states = Vec::<vk::DynamicState>::new();
        for i in 0..value.dynamic_state_count {
            let dynamic_state = unsafe { *value.p_dynamic_states.offset(i as isize) };
            dynamic_states.push(dynamic_state);
        }

        Self {
            flags: value.flags,
            dynamic_states,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineInputAssemblyStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct InputAssemblyState {
    pub flags: vk::PipelineInputAssemblyStateCreateFlags,
    pub topology: vk::PrimitiveTopology,
    pub primitive_restart_enable: bool,
}
impl Default for InputAssemblyState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: false,
        }
    }
}
impl InputAssemblyState {
    pub fn write_create_info<'a>(
        &self,
        create_info: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    ) -> vk::PipelineInputAssemblyStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .topology(self.topology)
            .primitive_restart_enable(self.primitive_restart_enable)
    }

    pub fn create_info(&self) -> vk::PipelineInputAssemblyStateCreateInfo {
        self.write_create_info(vk::PipelineInputAssemblyStateCreateInfo::default())
    }
}
impl<'a> FromCreateInfo<vk::PipelineInputAssemblyStateCreateInfo<'a>> for InputAssemblyState {
    fn from_create_info(value: &vk::PipelineInputAssemblyStateCreateInfo) -> Self {
        Self {
            flags: value.flags,
            topology: value.topology,
            primitive_restart_enable: value.primitive_restart_enable != 0,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineMultisampleStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct MultisampleState {
    pub flags: vk::PipelineMultisampleStateCreateFlags,
    pub rasterization_samples: vk::SampleCountFlags,
    pub sample_shading_enable: bool,
    pub min_sample_shading: f32,
    pub sample_mask: Vec<vk::SampleMask>,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
}
impl Default for MultisampleState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineMultisampleStateCreateFlags::empty(),
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: false,
            min_sample_shading: 1.,
            sample_mask: Vec::new(),
            alpha_to_coverage_enable: false,
            alpha_to_one_enable: false,
        }
    }
}
impl MultisampleState {
    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::PipelineMultisampleStateCreateInfo<'a>,
    ) -> vk::PipelineMultisampleStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .rasterization_samples(self.rasterization_samples)
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading)
            .sample_mask(&self.sample_mask)
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable)
    }

    pub fn create_info(&self) -> vk::PipelineMultisampleStateCreateInfo {
        self.write_create_info(vk::PipelineMultisampleStateCreateInfo::default())
    }

    /// # Safety
    /// - If `p_sample_mask` is not null and `rasterization_samples` is equal to `VK_SAMPLE_COUNT_64_BIT`
    ///   the `p_sample_mask` must point to an array of 2 values as per
    ///   [VUID-VkPipelineMultisampleStateCreateInfo-pSampleMask-parameter](https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkPipelineMultisampleStateCreateInfo.html)
    pub unsafe fn from_create_info(value: &vk::PipelineMultisampleStateCreateInfo) -> Self {
        let mut sample_mask = Vec::<vk::SampleMask>::new();
        if !value.p_sample_mask.is_null() {
            let lower_32_bit_sample_mask = unsafe { *value.p_sample_mask };
            sample_mask.push(lower_32_bit_sample_mask);

            if value.rasterization_samples == vk::SampleCountFlags::TYPE_64 {
                let upper_32_bit_sample_mask = unsafe { *value.p_sample_mask.offset(1) };
                sample_mask.push(upper_32_bit_sample_mask);
            }
        }

        Self {
            flags: value.flags,
            rasterization_samples: value.rasterization_samples,
            sample_shading_enable: value.sample_shading_enable != 0,
            min_sample_shading: value.min_sample_shading,
            sample_mask,
            alpha_to_coverage_enable: value.alpha_to_coverage_enable != 0,
            alpha_to_one_enable: value.alpha_to_one_enable != 0,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineRasterizationStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct RasterizationState {
    pub flags: vk::PipelineRasterizationStateCreateFlags,
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: vk::CullModeFlags,
    pub front_face: vk::FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub line_width: f32,
}
impl Default for RasterizationState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: false,
            rasterizer_discard_enable: false,
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::NONE,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            depth_bias_enable: false,
            depth_bias_constant_factor: 1.,
            depth_bias_clamp: 0.,
            depth_bias_slope_factor: 1.,
            line_width: 1.,
        }
    }
}
impl RasterizationState {
    pub fn write_create_info<'a>(
        &self,
        create_info: vk::PipelineRasterizationStateCreateInfo<'a>,
    ) -> vk::PipelineRasterizationStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .depth_clamp_enable(self.depth_clamp_enable)
            .rasterizer_discard_enable(self.rasterizer_discard_enable)
            .polygon_mode(self.polygon_mode)
            .cull_mode(self.cull_mode)
            .front_face(self.front_face)
            .depth_bias_enable(self.depth_bias_enable)
            .depth_bias_constant_factor(self.depth_bias_constant_factor)
            .depth_bias_clamp(self.depth_bias_clamp)
            .depth_bias_slope_factor(self.depth_bias_slope_factor)
            .line_width(self.line_width)
    }

    pub fn create_info(&self) -> vk::PipelineRasterizationStateCreateInfo {
        self.write_create_info(vk::PipelineRasterizationStateCreateInfo::default())
    }
}
impl<'a> FromCreateInfo<vk::PipelineRasterizationStateCreateInfo<'a>> for RasterizationState {
    fn from_create_info(value: &vk::PipelineRasterizationStateCreateInfo) -> Self {
        Self {
            flags: value.flags,
            depth_clamp_enable: value.depth_clamp_enable != 0,
            rasterizer_discard_enable: value.rasterizer_discard_enable != 0,
            polygon_mode: value.polygon_mode,
            cull_mode: value.cull_mode,
            front_face: value.front_face,
            depth_bias_enable: value.depth_bias_enable != 0,
            depth_bias_constant_factor: value.depth_bias_constant_factor,
            depth_bias_clamp: value.depth_bias_clamp,
            depth_bias_slope_factor: value.depth_bias_slope_factor,
            line_width: value.line_width,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineTessellationStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct TessellationState {
    pub flags: vk::PipelineTessellationStateCreateFlags,
    pub patch_control_points: u32,
}
impl Default for TessellationState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineTessellationStateCreateFlags::empty(),
            patch_control_points: 0,
        }
    }
}
impl TessellationState {
    pub fn write_create_info<'a>(
        &self,
        create_info: vk::PipelineTessellationStateCreateInfo<'a>,
    ) -> vk::PipelineTessellationStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .patch_control_points(self.patch_control_points)
    }

    pub fn create_info(&self) -> vk::PipelineTessellationStateCreateInfo {
        self.write_create_info(vk::PipelineTessellationStateCreateInfo::default())
    }
}
impl<'a> FromCreateInfo<vk::PipelineTessellationStateCreateInfo<'a>> for TessellationState {
    fn from_create_info(value: &vk::PipelineTessellationStateCreateInfo) -> Self {
        Self {
            flags: value.flags,
            patch_control_points: value.patch_control_points,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineVertexInputStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct VertexInputState {
    pub flags: vk::PipelineVertexInputStateCreateFlags,
    pub vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
}
impl Default for VertexInputState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineVertexInputStateCreateFlags::empty(),
            vertex_binding_descriptions: Vec::new(),
            vertex_attribute_descriptions: Vec::new(),
        }
    }
}
impl VertexInputState {
    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::PipelineVertexInputStateCreateInfo<'a>,
    ) -> vk::PipelineVertexInputStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .vertex_binding_descriptions(&self.vertex_binding_descriptions)
            .vertex_attribute_descriptions(&self.vertex_attribute_descriptions)
    }

    pub fn create_info(&self) -> vk::PipelineVertexInputStateCreateInfo {
        self.write_create_info(vk::PipelineVertexInputStateCreateInfo::default())
    }

    /// # Safety
    /// - if `p_vertex_binding_descriptions` is not null, it must point to an array of `vertex_binding_description_count` values
    /// - if `p_vertex_attribute_descriptions` is not null, it must point to an array of `vertex_attribute_description_count` values
    pub unsafe fn from_create_info(value: &vk::PipelineVertexInputStateCreateInfo) -> Self {
        let mut vertex_binding_descriptions = Vec::<vk::VertexInputBindingDescription>::new();
        for i in 0..value.vertex_binding_description_count {
            let binding_description =
                unsafe { *value.p_vertex_binding_descriptions.offset(i as isize) };
            vertex_binding_descriptions.push(binding_description);
        }

        let mut vertex_attribute_descriptions = Vec::<vk::VertexInputAttributeDescription>::new();
        for i in 0..value.vertex_attribute_description_count {
            let attribute_description =
                unsafe { *value.p_vertex_attribute_descriptions.offset(i as isize) };
            vertex_attribute_descriptions.push(attribute_description);
        }

        Self {
            flags: value.flags,
            vertex_binding_descriptions,
            vertex_attribute_descriptions,
        }
    }
}

#[doc = "<https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkPipelineViewportStateCreateInfo.html>"]
#[derive(Debug, Clone)]
pub struct ViewportState {
    pub flags: vk::PipelineViewportStateCreateFlags,
    pub viewports: Vec<vk::Viewport>,
    pub scissors: Vec<vk::Rect2D>,
}
impl Default for ViewportState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewports: Vec::new(),
            scissors: Vec::new(),
        }
    }
}
impl ViewportState {
    pub fn new_dynamic(viewport_count: usize, scissor_count: usize) -> Self {
        Self {
            viewports: vec![Default::default(); viewport_count],
            scissors: vec![Default::default(); scissor_count],
            ..Default::default()
        }
    }

    pub fn write_create_info<'a>(
        &'a self,
        create_info: vk::PipelineViewportStateCreateInfo<'a>,
    ) -> vk::PipelineViewportStateCreateInfo<'a> {
        create_info
            .flags(self.flags)
            .viewports(&self.viewports)
            .scissors(&self.scissors)
    }

    pub fn create_info(&self) -> vk::PipelineViewportStateCreateInfo {
        self.write_create_info(vk::PipelineViewportStateCreateInfo::default())
    }

    /// # Safety
    /// - if `p_viewports` is not null, it must point to an array of `viewport_count` values
    /// - if `p_scissors` is not null, it must point to an array of `scissor_count` values
    pub unsafe fn from_create_info(value: &vk::PipelineViewportStateCreateInfo) -> Self {
        let mut viewports = Vec::<vk::Viewport>::new();
        for i in 0..value.viewport_count {
            let viewport = unsafe { *value.p_viewports.offset(i as isize) };
            viewports.push(viewport);
        }

        let mut scissors = Vec::<vk::Rect2D>::new();
        for i in 0..value.scissor_count {
            let scissor = unsafe { *value.p_scissors.offset(i as isize) };
            scissors.push(scissor);
        }

        Self {
            flags: value.flags,
            viewports,
            scissors,
        }
    }
}

// Helper stuff (ft. borrow checker wrestling <3 rust)

/// Per-pipeline creation arguments
#[derive(Clone)]
pub struct PerPipelineCreationParams<'a> {
    pipeline_layout: Arc<PipelineLayout>,
    properties: GraphicsPipelineProperties,
    shader_stages: Vec<ShaderStage<'a>>,
    render_pass: &'a RenderPass,
}

/// The equivilent vk*CreateInfo structs for the members of `GraphicsPipelineProperties`.
///
/// These are populated in `GraphicsPipelineProperties::vk_create_infos` in order for the
/// `GraphicsPipelineCreateInfo` to have references to create info structs whose lifetimes
/// can be ensured to live for the duration of the create_info.
pub struct GraphicsPipelinePropertiesCreateInfosVk<'a> {
    pub vertex_input_state_vk: vk::PipelineVertexInputStateCreateInfo<'a>,
    pub input_assembly_state_vk: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    pub tessellation_state_vk: vk::PipelineTessellationStateCreateInfo<'a>,
    pub viewport_state_vk: vk::PipelineViewportStateCreateInfo<'a>,
    pub rasterization_state_vk: vk::PipelineRasterizationStateCreateInfo<'a>,
    pub multisample_state_vk: vk::PipelineMultisampleStateCreateInfo<'a>,
    pub depth_stencil_state_vk: vk::PipelineDepthStencilStateCreateInfo<'a>,
    pub color_blend_state_vk: vk::PipelineColorBlendStateCreateInfo<'a>,
    pub dynamic_state_vk: vk::PipelineDynamicStateCreateInfo<'a>,
}
impl<'a> Default for GraphicsPipelinePropertiesCreateInfosVk<'a> {
    fn default() -> Self {
        Self {
            vertex_input_state_vk: vk::PipelineVertexInputStateCreateInfo::default(),
            input_assembly_state_vk: vk::PipelineInputAssemblyStateCreateInfo::default(),
            tessellation_state_vk: vk::PipelineTessellationStateCreateInfo::default(),
            viewport_state_vk: vk::PipelineViewportStateCreateInfo::default(),
            rasterization_state_vk: vk::PipelineRasterizationStateCreateInfo::default(),
            multisample_state_vk: vk::PipelineMultisampleStateCreateInfo::default(),
            depth_stencil_state_vk: vk::PipelineDepthStencilStateCreateInfo::default(),
            color_blend_state_vk: vk::PipelineColorBlendStateCreateInfo::default(),
            dynamic_state_vk: vk::PipelineDynamicStateCreateInfo::default(),
        }
    }
}
