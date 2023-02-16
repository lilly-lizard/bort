use crate::{
    device::Device, memory::ALLOCATION_CALLBACK_NONE, pipeline::PipelineAccess,
    pipeline_layout::PipelineLayout,
};
use ash::vk;
use core::slice::SlicePattern;
use std::sync::Arc;

pub struct GraphicsPipeline {
    handle: vk::Pipeline,
    properties: GraphicsPipelineProperties,

    // dependencies
    pipeline_layout: Arc<PipelineLayout>,
    // note: we don't need to store references to `ShaderModule` or `PipelineCache` as per https://registry.khronos.org/vulkan/specs/1.0/html/vkspec.html#fundamentals-objectmodel-lifetime
}

impl GraphicsPipeline {
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

#[derive(Clone, Default)]
pub struct GraphicsPipelineProperties {
    pub flags: vk::PipelineCreateFlags,
    pub vertex_input_state: VertexInputState,
    pub input_assembly_state: InputAssemblyState,
    // lifetime members
    vertex_input_state_vk: vk::PipelineVertexInputStateCreateInfo,
    input_assembly_state_vk: vk::PipelineInputAssemblyStateCreateInfo,
}

impl GraphicsPipelineProperties {
    pub fn create_info_builder(&mut self) -> vk::GraphicsPipelineCreateInfoBuilder {
        self.vertex_input_state_vk = self.vertex_input_state.create_info_builder().build();
        self.input_assembly_state_vk = self.input_assembly_state.create_info_builder().build();

        vk::GraphicsPipelineCreateInfo::builder()
            .flags(self.flags)
            .vertex_input_state(&self.vertex_input_state_vk)
            .input_assembly_state(&self.input_assembly_state_vk)
    }
}

#[derive(Clone, Default)]
pub struct VertexInputState {
    pub flags: vk::PipelineVertexInputStateCreateFlags,
    pub vertex_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
}
impl VertexInputState {
    pub fn create_info_builder(&self) -> vk::PipelineVertexInputStateCreateInfoBuilder {
        vk::PipelineVertexInputStateCreateInfo::builder()
            .flags(self.flags)
            .vertex_binding_descriptions(self.vertex_binding_descriptions.as_slice())
            .vertex_attribute_descriptions(self.vertex_attribute_descriptions.as_slice())
    }
}

#[derive(Clone, Default)]
pub struct InputAssemblyState {
    pub flags: vk::PipelineInputAssemblyStateCreateFlags,
    pub topology: vk::PrimitiveTopology,
    pub primitive_restart_enable: bool,
}
impl InputAssemblyState {
    pub fn create_info_builder(&self) -> vk::PipelineInputAssemblyStateCreateInfoBuilder {
        vk::PipelineInputAssemblyStateCreateInfo::builder()
            .flags(self.flags)
            .topology(self.topology)
            .primitive_restart_enable(self.primitive_restart_enable)
    }
}

#[derive(Clone, Default)]
pub struct TessellationState {
    pub flags: vk::PipelineTessellationStateCreateFlags,
    pub patch_control_points: u32,
}
impl TessellationState {
    pub fn create_info_builder(&self) -> vk::PipelineTessellationStateCreateInfoBuilder {
        vk::PipelineTessellationStateCreateInfo::builder()
            .flags(self.flags)
            .patch_control_points(self.patch_control_points)
    }
}

#[derive(Clone, Default)]
pub struct ViewportState {
    pub flags: vk::PipelineViewportStateCreateFlags,
    pub viewports: Vec<vk::Viewport>,
    pub scissors: Vec<vk::Rect2D>,
}
impl ViewportState {
    pub fn create_info_builder(&self) -> vk::PipelineViewportStateCreateInfoBuilder {
        vk::PipelineViewportStateCreateInfo::builder()
            .flags(self.flags)
            .viewports(self.viewports.as_slice())
            .scissors(self.scissors.as_slice())
    }
}

#[derive(Clone, Default)]
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
    pub fn create_info_builder(&self) -> vk::PipelineRasterizationStateCreateInfoBuilder {
        vk::PipelineRasterizationStateCreateInfo::builder()
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
    pub fn create_info_builder(&self) -> vk::PipelineMultisampleStateCreateInfoBuilder {
        vk::PipelineMultisampleStateCreateInfo::builder()
            .flags(self.flags)
            .rasterization_samples(self.rasterization_samples)
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading)
            .sample_mask(self.sample_mask.as_slice())
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable)
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
    pub fn create_info_builder(&self) -> vk::PipelineDepthStencilStateCreateInfoBuilder {
        vk::PipelineDepthStencilStateCreateInfo::builder()
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
}

pub struct ColorBlendState {
    pub flags: vk::PipelineColorBlendStateCreateFlags,
    pub logic_op_enable: bool,
    pub logic_op: vk::LogicOp,
    pub attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    pub blend_constants: [f32; 4],
}
impl ColorBlendState {
    pub fn create_info_builder(&self) -> vk::PipelineColorBlendStateCreateInfoBuilder {
        vk::PipelineColorBlendStateCreateInfo::builder()
            .flags(self.flags)
            .logic_op_enable(self.logic_op_enable)
            .logic_op(self.logic_op)
            .attachments(self.attachments.as_slice())
            .blend_constants(self.blend_constants)
    }
}
