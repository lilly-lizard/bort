use crate::{
    Device, DeviceOwned, PipelineAccess, PipelineCache, PipelineLayout, RenderPass, ShaderStage,
    ALLOCATION_CALLBACK_NONE,
};
use ash::{prelude::VkResult, vk};
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
        mut properties: GraphicsPipelineProperties,
        shader_stages: impl IntoIterator<Item = ShaderStage>,
        render_pass: &RenderPass,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Self> {
        let shader_stages_vk: Vec<vk::PipelineShaderStageCreateInfo> = shader_stages
            .into_iter()
            .map(|stage| stage.create_info_builder().build())
            .collect();

        let create_info_builder = properties
            .create_info_builder()
            .stages(shader_stages_vk.as_slice())
            .render_pass(render_pass.handle())
            .layout(pipeline_layout.handle());

        let cache_handle = if let Some(pipeline_cache) = pipeline_cache {
            pipeline_cache.handle()
        } else {
            vk::PipelineCache::null()
        };

        let handles = unsafe {
            pipeline_layout.device().inner().create_graphics_pipelines(
                cache_handle,
                &[create_info_builder.build()],
                ALLOCATION_CALLBACK_NONE,
            )
        }
        .map_err(|(_pipelines, err_code)| err_code)?; // note: cbf taking VK_PIPELINE_COMPILE_REQUIRED into account...
        let handle = handles[0];

        Ok(Self {
            handle,
            properties,
            pipeline_layout,
        })
    }

    pub fn batch_create<'a>(
        device: &Device,
        per_pipeline_params: impl Iterator<Item = NewPipelineCreateParams<'a>>,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Vec<Self>> {
        let mut per_pipeline_params = per_pipeline_params.collect::<Vec<_>>();
        let create_infos = per_pipeline_params
            .iter_mut()
            .map(
                |NewPipelineCreateParams {
                     pipeline_layout,
                     properties,
                     shader_stages,
                     render_pass,
                 }| {
                    let shader_stages_vk: Vec<vk::PipelineShaderStageCreateInfo> = shader_stages
                        .into_iter()
                        .map(|stage| stage.create_info_builder().build())
                        .collect();

                    properties
                        .create_info_builder()
                        .stages(shader_stages_vk.as_slice())
                        .render_pass(render_pass.handle())
                        .layout(pipeline_layout.handle())
                        .build()
                },
            )
            .collect::<Vec<_>>();

        let cache_handle = if let Some(pipeline_cache) = pipeline_cache {
            pipeline_cache.handle()
        } else {
            vk::PipelineCache::null()
        };

        let handles = unsafe {
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
                handle: handles[index],
                properties: params.properties,
                pipeline_layout: params.pipeline_layout.clone(),
            })
            .collect::<Vec<_>>();

        Ok(pipelines)
    }

    // Getters

    pub fn properties(&self) -> &GraphicsPipelineProperties {
        &self.properties
    }
}

impl PipelineAccess for GraphicsPipeline {
    fn handle(&self) -> vk::Pipeline {
        self.handle
    }

    fn pipeline_layout(&self) -> &Arc<PipelineLayout> {
        &self.pipeline_layout
    }
}

impl DeviceOwned for GraphicsPipeline {
    #[inline]
    fn device(&self) -> &Arc<Device> {
        &self.pipeline_layout.device()
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

pub struct NewPipelineCreateParams<'a> {
    pipeline_layout: Arc<PipelineLayout>,
    properties: GraphicsPipelineProperties,
    shader_stages: Vec<ShaderStage>,
    render_pass: &'a RenderPass,
}

// Properties

/// Note: doesn't include shader stages
#[derive(Default)]
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

    // because these need to be stored for the lifetime duration of self todo can't partial initialize because of these!!
    vertex_input_state_vk: vk::PipelineVertexInputStateCreateInfo,
    input_assembly_state_vk: vk::PipelineInputAssemblyStateCreateInfo,
    tessellation_state_vk: vk::PipelineTessellationStateCreateInfo,
    viewport_state_vk: vk::PipelineViewportStateCreateInfo,
    rasterization_state_vk: vk::PipelineRasterizationStateCreateInfo,
    multisample_state_vk: vk::PipelineMultisampleStateCreateInfo,
    depth_stencil_state_vk: vk::PipelineDepthStencilStateCreateInfo,
    color_blend_state_vk: vk::PipelineColorBlendStateCreateInfo,
    dynamic_state_vk: vk::PipelineDynamicStateCreateInfo,
}

impl GraphicsPipelineProperties {
    pub fn new(
        flags: vk::PipelineCreateFlags,
        subpass_index: u32,
        vertex_input_state: VertexInputState,
        input_assembly_state: InputAssemblyState,
        tessellation_state: TessellationState,
        viewport_state: ViewportState,
        rasterization_state: RasterizationState,
        multisample_state: MultisampleState,
        depth_stencil_state: DepthStencilState,
        color_blend_state: ColorBlendState,
        dynamic_state: DynamicState,
    ) -> Self {
        Self {
            flags,
            subpass_index,
            vertex_input_state,
            input_assembly_state,
            tessellation_state,
            viewport_state,
            rasterization_state,
            multisample_state,
            depth_stencil_state,
            color_blend_state,
            dynamic_state,
            ..Self::default()
        }
    }

    /// Note: this doesn't populate:
    /// - `layout`
    /// - `render_pass`
    /// - `subpass`
    /// - `stages`
    /// - `base_pipeline_handle`
    /// - `base_pipeline_index`
    pub fn create_info_builder(&mut self) -> vk::GraphicsPipelineCreateInfoBuilder {
        self.vertex_input_state_vk = self
            .vertex_input_state
            .build(vk::PipelineVertexInputStateCreateInfo::builder());
        self.input_assembly_state_vk = self
            .input_assembly_state
            .build(vk::PipelineInputAssemblyStateCreateInfo::builder());
        self.tessellation_state_vk = self
            .tessellation_state
            .build(vk::PipelineTessellationStateCreateInfo::builder());
        self.viewport_state_vk = self
            .viewport_state
            .build(vk::PipelineViewportStateCreateInfo::builder());
        self.rasterization_state_vk = self
            .rasterization_state
            .build(vk::PipelineRasterizationStateCreateInfo::builder());
        self.multisample_state_vk = self
            .multisample_state
            .build(vk::PipelineMultisampleStateCreateInfo::builder());
        self.depth_stencil_state_vk = self
            .depth_stencil_state
            .build(vk::PipelineDepthStencilStateCreateInfo::builder());
        self.color_blend_state_vk = self
            .color_blend_state
            .build(vk::PipelineColorBlendStateCreateInfo::builder());
        self.dynamic_state_vk = self
            .dynamic_state
            .build(vk::PipelineDynamicStateCreateInfo::builder());

        vk::GraphicsPipelineCreateInfo::builder()
            .flags(self.flags)
            .subpass(self.subpass_index)
            .vertex_input_state(&self.vertex_input_state_vk)
            .input_assembly_state(&self.input_assembly_state_vk)
            .tessellation_state(&self.tessellation_state_vk)
            .viewport_state(&self.viewport_state_vk)
            .rasterization_state(&self.rasterization_state_vk)
            .multisample_state(&self.multisample_state_vk)
            .depth_stencil_state(&self.depth_stencil_state_vk)
            .color_blend_state(&self.color_blend_state_vk)
            .dynamic_state(&self.dynamic_state_vk)
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
impl ColorBlendState {
    pub fn new_default(attachments: Vec<vk::PipelineColorBlendAttachmentState>) -> Self {
        Self {
            attachments,
            ..Default::default()
        }
    }

    pub fn build(
        &self,
        builder: vk::PipelineColorBlendStateCreateInfoBuilder,
    ) -> vk::PipelineColorBlendStateCreateInfo {
        builder
            .flags(self.flags)
            .logic_op_enable(self.logic_op.is_some())
            .logic_op(self.logic_op.unwrap_or(vk::LogicOp::CLEAR))
            .attachments(self.attachments.as_slice())
            .blend_constants(self.blend_constants)
            .build()
    }

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
impl DepthStencilState {
    pub fn build(
        &self,
        builder: vk::PipelineDepthStencilStateCreateInfoBuilder,
    ) -> vk::PipelineDepthStencilStateCreateInfo {
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
            .build()
    }
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

#[derive(Clone)]
pub struct DynamicState {
    pub flags: vk::PipelineDynamicStateCreateFlags,
    pub dynamic_states: Vec<vk::DynamicState>,
}
impl DynamicState {
    pub fn new_default(dynamic_states: Vec<vk::DynamicState>) -> Self {
        Self {
            dynamic_states,
            ..Default::default()
        }
    }

    pub fn build(
        &self,
        builder: vk::PipelineDynamicStateCreateInfoBuilder,
    ) -> vk::PipelineDynamicStateCreateInfo {
        builder
            .flags(self.flags)
            .dynamic_states(self.dynamic_states.as_slice())
            .build()
    }
}
impl Default for DynamicState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineDynamicStateCreateFlags::empty(),
            dynamic_states: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct InputAssemblyState {
    pub flags: vk::PipelineInputAssemblyStateCreateFlags,
    pub topology: vk::PrimitiveTopology,
    pub primitive_restart_enable: bool,
}
impl InputAssemblyState {
    pub fn build(
        &self,
        builder: vk::PipelineInputAssemblyStateCreateInfoBuilder,
    ) -> vk::PipelineInputAssemblyStateCreateInfo {
        builder
            .flags(self.flags)
            .topology(self.topology)
            .primitive_restart_enable(self.primitive_restart_enable)
            .build()
    }
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
impl MultisampleState {
    pub fn build(
        &self,
        builder: vk::PipelineMultisampleStateCreateInfoBuilder,
    ) -> vk::PipelineMultisampleStateCreateInfo {
        builder
            .flags(self.flags)
            .rasterization_samples(self.rasterization_samples)
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading)
            .sample_mask(self.sample_mask.as_slice())
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable)
            .build()
    }
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
impl RasterizationState {
    pub fn build(
        &self,
        builder: vk::PipelineRasterizationStateCreateInfoBuilder,
    ) -> vk::PipelineRasterizationStateCreateInfo {
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
            .build()
    }
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

#[derive(Clone)]
pub struct TessellationState {
    pub flags: vk::PipelineTessellationStateCreateFlags,
    pub patch_control_points: u32,
}
impl TessellationState {
    pub fn build(
        &self,
        builder: vk::PipelineTessellationStateCreateInfoBuilder,
    ) -> vk::PipelineTessellationStateCreateInfo {
        builder
            .flags(self.flags)
            .patch_control_points(self.patch_control_points)
            .build()
    }
}
impl Default for TessellationState {
    fn default() -> Self {
        Self {
            flags: vk::PipelineTessellationStateCreateFlags::empty(),
            patch_control_points: 0,
        }
    }
}

#[derive(Clone)]
pub struct VertexInputState {
    pub flags: vk::PipelineVertexInputStateCreateFlags,
    pub vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
}
impl VertexInputState {
    pub fn build(
        &self,
        builder: vk::PipelineVertexInputStateCreateInfoBuilder,
    ) -> vk::PipelineVertexInputStateCreateInfo {
        builder
            .flags(self.flags)
            .vertex_binding_descriptions(self.vertex_binding_descriptions.as_slice())
            .vertex_attribute_descriptions(self.vertex_attribute_descriptions.as_slice())
            .build()
    }
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

#[derive(Clone)]
pub struct ViewportState {
    pub flags: vk::PipelineViewportStateCreateFlags,
    pub viewports: Vec<vk::Viewport>,
    pub scissors: Vec<vk::Rect2D>,
}
impl ViewportState {
    pub fn build(
        &self,
        builder: vk::PipelineViewportStateCreateInfoBuilder,
    ) -> vk::PipelineViewportStateCreateInfo {
        builder
            .flags(self.flags)
            .viewports(self.viewports.as_slice())
            .scissors(self.scissors.as_slice())
            .build()
    }
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
