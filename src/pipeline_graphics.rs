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
        shader_stages: impl IntoIterator<Item = ShaderStage>,
        render_pass: &RenderPass,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Self> {
        // populate vkPipelineShaderStageCreateInfo
        let shader_stages_vk: Vec<vk::PipelineShaderStageCreateInfo> = shader_stages
            .into_iter()
            .map(|stage| stage.create_info_builder().build())
            .collect();

        // populate the sub-structs of vkGraphicsPipelineCreateInfo defined by GraphicsPipelineProperties
        let properties_vk = properties.vk_create_infos();

        // use these and other args to populate the fields of vkGraphicsPipelineCreateInfo
        let create_info_builder = properties
            .write_create_info_builder(vk::GraphicsPipelineCreateInfo::builder(), &properties_vk);
        let create_info_builder = create_info_builder
            .stages(shader_stages_vk.as_slice())
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
                &[create_info_builder.build()],
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

    pub fn new_batch_create<'a>(
        device: &Device,
        per_pipeline_params: Vec<PerPipelineCreationParams<'a>>,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Vec<Self>> {
        let pipeline_count = per_pipeline_params.len();

        // populate the sub-structs of vkGraphicsPipelineCreateInfo defined by GraphicsPipelineProperties
        let mut pipeline_properties_vk: Vec<GraphicsPipelinePropertiesCreateInfosVk> = Vec::new();
        for pipeline_index in 0..pipeline_count {
            let properties_vk = per_pipeline_params[pipeline_index]
                .properties
                .vk_create_infos();
            pipeline_properties_vk.push(properties_vk);
        }

        // populate the vkPipelineShaderStageCreateInfo structs
        let mut shader_stage_handles: Vec<Vec<vk::PipelineShaderStageCreateInfo>> = Vec::new();
        for pipeline_index in 0..pipeline_count {
            let shader_stages_vk = per_pipeline_params[pipeline_index]
                .shader_stages
                .iter()
                .map(|stage| stage.create_info_builder().build())
                .collect::<Vec<_>>();
            shader_stage_handles.push(shader_stages_vk);
        }

        // use these and other args to populate the fields of vkGraphicsPipelineCreateInfo
        let mut create_info_builders: Vec<vk::GraphicsPipelineCreateInfoBuilder> = Vec::new();
        for pipeline_index in 0..pipeline_count {
            let create_info_builder = per_pipeline_params[pipeline_index]
                .properties
                .write_create_info_builder(
                    vk::GraphicsPipelineCreateInfo::builder(),
                    &pipeline_properties_vk[pipeline_index],
                );

            let create_info_builder = create_info_builder
                .stages(shader_stage_handles[pipeline_index].as_slice())
                .render_pass(per_pipeline_params[pipeline_index].render_pass.handle())
                .layout(per_pipeline_params[pipeline_index].pipeline_layout.handle());

            create_info_builders.push(create_info_builder);
        }

        let create_infos = create_info_builders
            .into_iter()
            .map(|builder| builder.build())
            .collect::<Vec<_>>();

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

        let pipelines = per_pipeline_params
            .into_iter()
            .enumerate()
            .map(|(index, params)| Self {
                handle: pipeline_handles[index],
                properties: params.properties,
                pipeline_layout: params.pipeline_layout.clone(),
            })
            .collect::<Vec<_>>();

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
}

impl DeviceOwned for GraphicsPipeline {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.pipeline_layout.device()
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

/// Note: doesn't include shader stages, render pass, pipeline layout or pipeline cache
#[derive(Clone, Default)]
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
    /// Returns the `builder` arg containing references to the structs in `properties_vk`.
    ///
    /// Note: this doesn't populate:
    /// - `layout`
    /// - `render_pass`
    /// - `subpass`
    /// - `stages`
    /// - `base_pipeline_handle`
    /// - `base_pipeline_index`
    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::GraphicsPipelineCreateInfoBuilder<'a>,
        properties_vk: &'a GraphicsPipelinePropertiesCreateInfosVk<'a>,
    ) -> vk::GraphicsPipelineCreateInfoBuilder<'a> {
        // pass references to `properties_vk` to `builder` to populate relevant members
        builder
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

    /// Returns a set of vk::*CreateInfoBuilder structs populated by the members of `self`.
    /// Use this with `Self::write_create_info_builder` to populate a `GraphicsPipelineCreateInfoBuilder`.
    pub fn vk_create_infos<'a>(&'a self) -> GraphicsPipelinePropertiesCreateInfosVk<'a> {
        let mut properties_vk = GraphicsPipelinePropertiesCreateInfosVk::default();
        // write vk create-info builders for each member to `properties_vk`
        properties_vk.vertex_input_state_vk = self.vertex_input_state.create_info_builder();
        properties_vk.input_assembly_state_vk = self.input_assembly_state.create_info_builder();
        properties_vk.tessellation_state_vk = self.tessellation_state.create_info_builder();
        properties_vk.viewport_state_vk = self.viewport_state.create_info_builder();
        properties_vk.rasterization_state_vk = self.rasterization_state.create_info_builder();
        properties_vk.multisample_state_vk = self.multisample_state.create_info_builder();
        properties_vk.depth_stencil_state_vk = self.depth_stencil_state.create_info_builder();
        properties_vk.color_blend_state_vk = self.color_blend_state.create_info_builder();
        properties_vk.dynamic_state_vk = self.dynamic_state.create_info_builder();
        properties_vk
    }
}

// Sub-Properties

#[derive(Clone)]
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
    pub fn new(attachments: Vec<vk::PipelineColorBlendAttachmentState>) -> Self {
        Self {
            attachments,
            ..Default::default()
        }
    }

    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::PipelineColorBlendStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineColorBlendStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .logic_op_enable(self.logic_op.is_some())
            .logic_op(self.logic_op.unwrap_or(vk::LogicOp::CLEAR))
            .attachments(self.attachments.as_slice())
            .blend_constants(self.blend_constants)
    }

    pub fn create_info_builder(&self) -> vk::PipelineColorBlendStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineColorBlendStateCreateInfo::builder())
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
            ..Default::default()
        }
    }

    /// Returns `vk::PipelineColorBlendAttachmentState` where the output will be merged with the existing value
    /// based on the alpha of the source.
    pub fn blend_state_alpha() -> vk::PipelineColorBlendAttachmentState {
        vk::PipelineColorBlendAttachmentState {
            blend_enable: 1,
            color_blend_op: vk::BlendOp::ADD,
            src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            src_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            alpha_blend_op: vk::BlendOp::ADD,
            dst_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
            dst_alpha_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
            color_write_mask: vk::ColorComponentFlags::RGBA,
            ..Default::default()
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
            ..Default::default()
        }
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &self,
        builder: vk::PipelineDepthStencilStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineDepthStencilStateCreateInfoBuilder<'a> {
        builder
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

    pub fn create_info_builder(&self) -> vk::PipelineDepthStencilStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineDepthStencilStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::PipelineDynamicStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineDynamicStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .dynamic_states(self.dynamic_states.as_slice())
    }

    pub fn create_info_builder(&self) -> vk::PipelineDynamicStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineDynamicStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &self,
        builder: vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineInputAssemblyStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .topology(self.topology)
            .primitive_restart_enable(self.primitive_restart_enable)
    }

    pub fn create_info_builder(&self) -> vk::PipelineInputAssemblyStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineInputAssemblyStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::PipelineMultisampleStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineMultisampleStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .rasterization_samples(self.rasterization_samples)
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading)
            .sample_mask(self.sample_mask.as_slice())
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable)
    }

    pub fn create_info_builder(&self) -> vk::PipelineMultisampleStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineMultisampleStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &self,
        builder: vk::PipelineRasterizationStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineRasterizationStateCreateInfoBuilder<'a> {
        builder
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

    pub fn create_info_builder(&self) -> vk::PipelineRasterizationStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineRasterizationStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &self,
        builder: vk::PipelineTessellationStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineTessellationStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .patch_control_points(self.patch_control_points)
    }

    pub fn create_info_builder(&self) -> vk::PipelineTessellationStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineTessellationStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::PipelineVertexInputStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineVertexInputStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .vertex_binding_descriptions(self.vertex_binding_descriptions.as_slice())
            .vertex_attribute_descriptions(self.vertex_attribute_descriptions.as_slice())
    }

    pub fn create_info_builder(&self) -> vk::PipelineVertexInputStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineVertexInputStateCreateInfo::builder())
    }
}

#[derive(Clone)]
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
    pub fn write_create_info_builder<'a>(
        &'a self,
        builder: vk::PipelineViewportStateCreateInfoBuilder<'a>,
    ) -> vk::PipelineViewportStateCreateInfoBuilder<'a> {
        builder
            .flags(self.flags)
            .viewports(self.viewports.as_slice())
            .scissors(self.scissors.as_slice())
    }

    pub fn create_info_builder(&self) -> vk::PipelineViewportStateCreateInfoBuilder {
        self.write_create_info_builder(vk::PipelineViewportStateCreateInfo::builder())
    }
}

// Helper

/// Per-pipeline creation arguments
#[derive(Clone)]
pub struct PerPipelineCreationParams<'a> {
    pipeline_layout: Arc<PipelineLayout>,
    properties: GraphicsPipelineProperties,
    shader_stages: Vec<ShaderStage>,
    render_pass: &'a RenderPass,
}

/// The equivilent vk*CreateInfo structs for the members of `GraphicsPipelineProperties`.
///
/// These are populated in `GraphicsPipelineProperties::vk_create_infos` in order for the
/// `GraphicsPipelineCreateInfo` to have references to create info structs whose lifetimes
/// can be ensured to live for the duration of the builder.
pub struct GraphicsPipelinePropertiesCreateInfosVk<'a> {
    pub vertex_input_state_vk: vk::PipelineVertexInputStateCreateInfoBuilder<'a>,
    pub input_assembly_state_vk: vk::PipelineInputAssemblyStateCreateInfoBuilder<'a>,
    pub tessellation_state_vk: vk::PipelineTessellationStateCreateInfoBuilder<'a>,
    pub viewport_state_vk: vk::PipelineViewportStateCreateInfoBuilder<'a>,
    pub rasterization_state_vk: vk::PipelineRasterizationStateCreateInfoBuilder<'a>,
    pub multisample_state_vk: vk::PipelineMultisampleStateCreateInfoBuilder<'a>,
    pub depth_stencil_state_vk: vk::PipelineDepthStencilStateCreateInfoBuilder<'a>,
    pub color_blend_state_vk: vk::PipelineColorBlendStateCreateInfoBuilder<'a>,
    pub dynamic_state_vk: vk::PipelineDynamicStateCreateInfoBuilder<'a>,
}
impl<'a> Default for GraphicsPipelinePropertiesCreateInfosVk<'a> {
    fn default() -> Self {
        Self {
            vertex_input_state_vk: vk::PipelineVertexInputStateCreateInfo::builder(),
            input_assembly_state_vk: vk::PipelineInputAssemblyStateCreateInfo::builder(),
            tessellation_state_vk: vk::PipelineTessellationStateCreateInfo::builder(),
            viewport_state_vk: vk::PipelineViewportStateCreateInfo::builder(),
            rasterization_state_vk: vk::PipelineRasterizationStateCreateInfo::builder(),
            multisample_state_vk: vk::PipelineMultisampleStateCreateInfo::builder(),
            depth_stencil_state_vk: vk::PipelineDepthStencilStateCreateInfo::builder(),
            color_blend_state_vk: vk::PipelineColorBlendStateCreateInfo::builder(),
            dynamic_state_vk: vk::PipelineDynamicStateCreateInfo::builder(),
        }
    }
}
