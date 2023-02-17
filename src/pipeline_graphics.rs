use crate::{
    device::Device, memory::ALLOCATION_CALLBACK_NONE, pipeline::PipelineAccess,
    pipeline_cache::PipelineCache, pipeline_layout::PipelineLayout, render_pass::RenderPass,
};
use ash::{prelude::VkResult, vk};
use std::{ffi::CString, sync::Arc};

pub struct GraphicsPipeline {
    handle: vk::Pipeline,
    properties: GraphicsPipelineProperties,

    // dependencies
    pipeline_layout: Arc<PipelineLayout>,
    // note: we don't need to store references to `ShaderModule` or `PipelineCache` as per https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#fundamentals-objectmodel-lifetime
}

impl GraphicsPipeline {
    pub fn new(
        pipeline_layout: Arc<PipelineLayout>,
        mut properties: GraphicsPipelineProperties,
        render_pass: &RenderPass,
        subpass_index: u32,
        pipeline_cache: Option<&PipelineCache>,
    ) -> VkResult<Self> {
        let create_info_builder = properties
            .create_info_builder()
            .render_pass(render_pass.handle())
            .subpass(subpass_index)
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
        .map_err(|(_pipelines, err_code)| err_code)?;
        let handle = handles[0];

        Ok(Self {
            handle,
            properties,
            pipeline_layout,
        })
    }

    pub unsafe fn from_handle(
        handle: vk::Pipeline,
        properties: GraphicsPipelineProperties,
        pipeline_layout: Arc<PipelineLayout>,
    ) -> Self {
        Self {
            handle,
            properties,
            pipeline_layout,
        }
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

#[derive(Default)]
pub struct GraphicsPipelineProperties {
    pub flags: vk::PipelineCreateFlags,
    pub shader_stages: Vec<ShaderStage>,
    pub vertex_input_state: VertexInputState,
    pub input_assembly_state: InputAssemblyState,
    pub tessellation_state: TessellationState,
    pub viewport_state: ViewportState,
    pub rasterization_state: RasterizationState,
    pub multisample_state: MultisampleState,
    pub depth_stencil_state: DepthStencilState,
    pub color_blend_state: ColorBlendState,
    pub dynamic_state: DynamicState,

    // lifetime members
    shader_stages_vk: Vec<vk::PipelineShaderStageCreateInfo>,
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
        shader_stages: Vec<ShaderStage>,
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
            shader_stages,
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
    /// - `base_pipeline_handle`
    /// - `base_pipeline_index`
    pub fn create_info_builder(&mut self) -> vk::GraphicsPipelineCreateInfoBuilder {
        self.shader_stages_vk = self
            .shader_stages
            .iter()
            .map(|stage| stage.build(vk::PipelineShaderStageCreateInfo::builder()))
            .collect();
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
            .stages(self.shader_stages_vk.as_slice())
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

#[derive(Clone)]
pub struct ShaderStage {
    pub flags: vk::PipelineShaderStageCreateFlags,
    pub stage: vk::ShaderStageFlags,
    pub module_handle: vk::ShaderModule,
    pub entry_point: CString,
    // todo spec constants...
}
impl ShaderStage {
    pub fn build(
        &self,
        builder: vk::PipelineShaderStageCreateInfoBuilder,
    ) -> vk::PipelineShaderStageCreateInfo {
        builder
            .flags(self.flags)
            .module(self.module_handle)
            .name(self.entry_point.as_c_str())
            .build()
    }
}
impl Default for ShaderStage {
    fn default() -> Self {
        Self {
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            stage: vk::ShaderStageFlags::empty(),
            module_handle: vk::ShaderModule::default(),
            entry_point: CString::default(),
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
        Self{
            flags: vk::PipelineInputAssemblyStateCreateFlags::empty(),
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: false,
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
        Self{
            flags: vk::PipelineTessellationStateCreateFlags::empty(),
            patch_control_points: ,
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
        Self{
            flags: vk::PipelineViewportStateCreateFlags::empty(),
            viewports: Vec::new(),
            scissors: Vec::new(),
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
        Self{
            flags: vk::PipelineRasterizationStateCreateFlags::empty(),
            depth_clamp_enable: false,
            rasterizer_discard_enable: bool,
            polygon_mode: vk::PolygonMode,
            cull_mode: vk::CullModeFlags,
            front_face: vk::FrontFace,
            depth_bias_enable: bool,
            depth_bias_constant_factor: f32,
            depth_bias_clamp: f32,
            depth_bias_slope_factor: f32,
            line_width: f32,
        }
    }
}

#[derive(Clone, Default)]
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

#[derive(Clone, Default)]
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
#[derive(Clone, Default)]

pub struct ColorBlendState {
    pub flags: vk::PipelineColorBlendStateCreateFlags,
    pub logic_op_enable: bool,
    pub logic_op: vk::LogicOp,
    pub attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    pub blend_constants: [f32; 4],
}
impl ColorBlendState {
    pub fn build(
        &self,
        builder: vk::PipelineColorBlendStateCreateInfoBuilder,
    ) -> vk::PipelineColorBlendStateCreateInfo {
        builder
            .flags(self.flags)
            .logic_op_enable(self.logic_op_enable)
            .logic_op(self.logic_op)
            .attachments(self.attachments.as_slice())
            .blend_constants(self.blend_constants)
            .build()
    }
}

#[derive(Clone, Default)]
pub struct DynamicState {
    pub flags: vk::PipelineDynamicStateCreateFlags,
    pub dynamic_states: Vec<vk::DynamicState>,
}
impl DynamicState {
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
