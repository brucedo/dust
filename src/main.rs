use log::debug;

mod input;
mod setup;

use setup::xcb_window;
use std::thread;
use xcb::x::Window;

use crate::input::input::KeyStroke;

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

    let entry = setup::instance::init();
    let instance = setup::instance::instance(&entry);
    let best_dev = setup::instance::enumerate_physical_devs(&instance);
    setup::instance::query_physical_device_queues(&best_dev, &instance);
    let surface_instance = setup::instance::xcb_surface_instance(&entry, &instance);
    let khr_surface_instance = setup::instance::khr_surface_instance(&entry, &instance);
    let _vk_surface = setup::instance::xcb_surface(&surface_instance, _xcb_ptr, &window);
    let caps = setup::instance::map_physical_device_to_surface_properties(
        &khr_surface_instance,
        &best_dev,
        &_vk_surface,
    );

    debug!(
        "Extents? {}x{}",
        caps.current_extent.width, caps.current_extent.height
    );
    debug!("Buffers? {}-{}", caps.min_image_count, caps.max_image_count);

    debug!("The...instance was created?");

    // let instance = unsafe { vk_entry.create_instance(&instance_info_bldr, allocation_callbacks) };

    let surface_instance = ash::khr::surface::Instance::new(&entry, &instance);
    unsafe { surface_instance.destroy_surface(_vk_surface, None) };
    unsafe { instance.destroy_instance(None) };

    debug!("Vulkan instance destroyed...");
}
