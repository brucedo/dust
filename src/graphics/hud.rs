use std::sync::OnceLock;

use ash::vk::{
    Buffer, BufferUsageFlags, CompareOp, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo, DescriptorType, Filter, GraphicsPipelineCreateInfo, Pipeline, PipelineCreateFlags, PipelineLayout, PipelineLayoutCreateInfo, PipelineShaderStageCreateFlags, PipelineShaderStageCreateInfo, RenderPass, Sampler, SamplerAddressMode, SamplerCreateFlags, SamplerCreateInfo, SamplerMipmapMode, Semaphore, ShaderStageFlags, EXT_PROVOKING_VERTEX_NAME
};
use log::debug;

use crate::setup::instance::VkContext;

use super::{
    image::DustImage,
    math_types::vertex::{
        sampled_vertex_3::{new as new_sampled, SampledVertex3},
        Vulkanic,
    },
    pools, shaders, transfer,
};

pub fn initialize(ctxt: &VkContext, hud_image: DustImage, render_pass: &RenderPass) {
    let hud_quad = vec![
        new_sampled([0f32, 0f32], [-0.5f32, 0.822222222, 0.1]),
        new_sampled([1f32, 0f32], [0.5f32, 0.822222222, 0.1]),
        new_sampled([0f32, 1f32], [-0.5f32, 1f32, 0.1]),
        new_sampled([1f32, 1f32], [0.5f32, 1f32, 0.1]),
    ];

    let (panel_buffer, copy_completed) = make_hud_panel(ctxt, &hud_quad);

    let texture_sampler = create_hud_samplers(ctxt);

    // Make a pipeline for this pass
    let hud_pipeline = GraphicsPipelineCreateInfo::default()
        .render_pass(*render_pass)
        .flags(PipelineCreateFlags::empty())
        .stages(&create_hud_pipeline_stages())
        .subpass(0)
    .layout(layout)
    // .viewport_state(viewport_state)
    // .multisample_state()
    // .color_blend_state(color_blend_state)
    // .vertex_input_state(vertex_input_state)
    // .rasterization_state(rasterization_state)
    // .input_assembly_state(input_assembly_state)

    // draw command?  Do I construct that here?
}


fn create_hud_samplers(ctxt: &VkContext) -> Sampler {
    let sampler_create = SamplerCreateInfo::default()
        .flags(SamplerCreateFlags::empty())
        .mag_filter(Filter::LINEAR)
        .min_filter(Filter::LINEAR)
        .compare_enable(false)
        .mipmap_mode(SamplerMipmapMode::LINEAR)
        .mip_lod_bias(0.0)
        .min_lod(0.0)
        .max_lod(0.0)
        .anisotropy_enable(false)
        .address_mode_u(SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(SamplerAddressMode::CLAMP_TO_EDGE);

        match unsafe {ctxt.logical_device.create_sampler(&sampler_create, None) } {
        Ok(sampler) => sampler, 
        Err(msg) => {panic!("Sampler creation failed: {:?}", msg); }
    }
}

fn create_descriptor_set_layout() {
    DescriptorSetLayoutBinding::default()
        .binding(0)
        .stage_flags(ShaderStageFlags::FRAGMENT)
        .descriptor_type(DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
    DescriptorSetLayoutCreateInfo::default()
        
}

fn create_hud_pipeline_layout() -> PipelineLayout {
    PipelineLayoutCreateInfo::default()
        .set_layouts(set_layouts);
}

fn create_hud_pipeline_stages<'a>() -> Vec<PipelineShaderStageCreateInfo<'a>> {
    let vertex_shader = match shaders::shader_by_name("passthrough") {
        Some(shader) => shader,
        None => {
            panic!("You need to actually make the vertex passthrough shader.");
        }
    };

    let fragment_shader = match shaders::shader_by_name("simple_texture") {
        Some(shader) => shader,
        None => {
            panic!("You need to make the texturing shader.");
        }
    };

    vec![
        PipelineShaderStageCreateInfo::default()
            .module(vertex_shader.shader_module)
            .flags(PipelineShaderStageCreateFlags::empty())
            .name(&vertex_shader.name),
        PipelineShaderStageCreateInfo::default()
            .module(fragment_shader.shader_module)
            .name(&vertex_shader.name),
    ]
}

fn make_hud_panel(ctxt: &VkContext, hud_mesh: &[SampledVertex3]) -> (Buffer, Semaphore) {
    debug!("Copying hud panel into device memory.");
    let mut buffer = Vec::<u8>::new();
    for sample in hud_mesh {
        sample.copy_into_vk_vec(&mut buffer);
    }

    transfer::copy_to_buffer(
        &buffer,
        ctxt,
        BufferUsageFlags::VERTEX_BUFFER,
        pools::get_graphics_queue_family(),
    )
}

// pub fn render_hud(ctxt: &VkContext) -> (Pipeline) {}
