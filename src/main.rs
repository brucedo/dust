use ash::vk::{
    Extent3D, Format, ImageCreateFlags, ImageCreateInfo, ImageLayout, ImageTiling, ImageType,
    ImageUsageFlags, SampleCountFlags, Semaphore, SharingMode,
};
use graphics::image::DustImage;
use graphics::pools::{get_graphics_queue_family, get_transfer_queue_family};
use graphics::{bitmap, pools, transfer};
use log::debug;

mod dust_errors;
mod graphics;
mod input;
mod setup;

use setup::{instance::VkContext, xcb_window};
use std::fs::File;
use std::io::Read;
use std::{
    thread::{self, sleep},
    time::Duration,
};
use xcb::x::Window;

use crate::{
    input::input::KeyStroke,
    setup::instance::{self},
};

fn main() {
    env_logger::init();

    // window system setup
    debug!("Starting X-Windows initialization...");
    let (conn, screen_num) = xcb_window::connect();
    xcb_window::interrogate_keymaps(&conn);
    xcb_window::extension_data(&conn);
    let window: Window = xcb_window::create_window(&conn, screen_num);
    let (upper_left, window_size) = xcb_window::interrogate_randr(&conn, window);
    xcb_window::resize_window(&conn, window, upper_left, window_size);

    let _xcb_ptr = conn.get_raw_conn();
    let (sender, _receiver) = std::sync::mpsc::sync_channel::<KeyStroke>(16);
    thread::spawn(move || xcb_window::event_loop(conn, sender));

    let vk_context = instance::default(_xcb_ptr, &window);
    // show_physical_memory_stats(&vk_context);

    let sample_bmp_data = load_sample_bmp();
    let hud_bar = match bitmap::new(&sample_bmp_data) {
        Ok(bar) => bar,
        Err(bitmap_error) => {
            panic!("The bitmap failed to load: {:?}", bitmap_error);
        }
    };

    let (finished, transfer_complete_semaphore) = transfer::copy_to_image(
        hud_bar.get_pixel_array(),
        &vk_context,
        &ImageCreateInfo::default()
            .format(Format::R8G8B8A8_SRGB)
            .flags(ImageCreateFlags::empty())
            .extent(
                Extent3D::default()
                    .depth(1)
                    .width(hud_bar.get_width() as u32)
                    .height(hud_bar.get_height() as u32),
            )
            .usage(ImageUsageFlags::INPUT_ATTACHMENT | ImageUsageFlags::TRANSFER_DST)
            .tiling(ImageTiling::OPTIMAL)
            .samples(SampleCountFlags::TYPE_1)
            .mip_levels(1)
            .sharing_mode(SharingMode::EXCLUSIVE)
            .array_layers(1)
            .image_type(ImageType::TYPE_2D)
            .initial_layout(ImageLayout::UNDEFINED),
        ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        get_graphics_queue_family(),
    );

    graphics::render::composite_hud(
        &vk_context,
        &finished.view,
        finished.format,
        transfer_complete_semaphore,
    );

    sleep(Duration::from_secs(3));

    debug!("Vulkan instance destroyed...");
}

fn load_sample_bmp() -> Vec<u8> {
    let mut buffer = Vec::<u8>::new();
    if let Ok(mut hud_file) = File::open("resources/doom_bar.bmp") {
        hud_file.read_to_end(&mut buffer);
    }

    buffer
}

fn show_physical_memory_stats(vk_ctxt: &VkContext) {
    let temp = vk_ctxt.physical_memory_properties;

    for index in 0..temp.memory_type_count as usize {
        debug!(
            "mem_type {}: {:?}",
            temp.memory_types[index].heap_index, temp.memory_types[index].property_flags
        );
    }

    for index in 0..temp.memory_heap_count as usize {
        if !temp.memory_heaps[index].flags.is_empty() {
            debug!(
                "Memory heap {:?}: {}",
                temp.memory_heaps[index].flags, temp.memory_heaps[index].size
            );
        } else {
            debug!(
                "Memory heap flags empty, size {}",
                temp.memory_heaps[index].size
            );
        }
    }
}
