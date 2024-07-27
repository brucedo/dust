use ash::{
    khr::swapchain,
    vk::{BufferCreateInfo, BufferUsageFlags, MemoryType, SharingMode},
};
use log::{debug, warn};

mod input;
mod setup;

use setup::{instance::VkContext, xcb_window};
use std::thread;
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
    let (sender, receiver) = std::sync::mpsc::sync_channel::<KeyStroke>(16);
    thread::spawn(move || xcb_window::event_loop(conn, sender));

    let vk_context = instance::default(_xcb_ptr, &window);
    show_physical_memory_stats(&vk_context);

    debug!("Vulkan instance destroyed...");
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

fn display_image<'a>(vulkan_context: &'a VkContext<'a>) {
    let image_width = vulkan_context.surface_capabilities.current_extent.width;
    let image_height = vulkan_context.surface_capabilities.current_extent.height;

    let buffer_info = BufferCreateInfo::default()
        .size((image_width * image_height * 4) as u64)
        .usage(BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER)
        .sharing_mode(SharingMode::EXCLUSIVE);

    let buffer = unsafe {
        vulkan_context
            .logical_device
            .create_buffer(&buffer_info, None)
    };
}
