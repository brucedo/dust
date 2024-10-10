use std::sync::OnceLock;

use ash::vk::{Buffer, BufferUsageFlags, Pipeline, Semaphore};
use log::debug;

use crate::setup::instance::VkContext;

use super::{
    image::DustImage,
    math_types::vertex::{
        sampled_vertex_3::{new as new_sampled, SampledVertex3},
        Vulkanic,
    },
    pools, transfer,
};

static HUD_VERTEX_BUFFER: OnceLock<Buffer> = OnceLock::new();

pub fn initialize(ctxt: &VkContext, hud_image: DustImage) {
    let hud_quad = vec![
        new_sampled([0f32, 0f32], [-0.5f32, 0.822222222, 0.1]),
        new_sampled([1f32, 0f32], [0.5f32, 0.822222222, 0.1]),
        new_sampled([0f32, 1f32], [-0.5f32, 1f32, 0.1]),
        new_sampled([1f32, 1f32], [0.5f32, 1f32, 0.1]),
    ];

    let (panel_buffer, copy_completed) = make_hud_panel(ctxt, &hud_quad);

    match HUD_VERTEX_BUFFER.set(panel_buffer) {
        Ok(_) => {}
        Err(_) => {
            panic!("Unable to set the hud panel.");
        }
    };
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
