use ash::vk::Pipeline;

use crate::setup::instance::VkContext;

use super::{
    image::DustImage, math_types::vertex::sampled_vertex_3::new as new_sampled,
    math_types::vertex::sampled_vertex_3::SampledVertex3,
};

pub fn initialize(hud_image: DustImage) {
    let hud_quad = vec![
        new_sampled([0f32, 0f32], [-0.5f32, .822222222, .1] ),
        new_sampled([1f32, 0f32], [0.5f32, .822222222, .1]),
        new_sampled([0f32, 1f32], [-0.5f32, 1, .1]),
        new_sampled([1f32, 1f32], [0.5f32, 1, .1])
    ];
}

pub fn render_hud(ctxt: &VkContext) -> (Pipeline) {}
