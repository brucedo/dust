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
    let surface_instance = setup::instance::xcb_surface_instance(&entry, &instance);
    let _vk_surface = setup::instance::xcb_surface(&surface_instance, _xcb_ptr, &window);

    debug!("The...instance was created?");

    // let instance = unsafe { vk_entry.create_instance(&instance_info_bldr, allocation_callbacks) };

    unsafe { instance.destroy_instance(None) };

    debug!("Vulkan instance destroyed...");
}
