use ash::khr::swapchain;
use log::debug;

mod input;
mod setup;

use setup::xcb_window;
use std::thread;
use xcb::x::Window;

use crate::{
    input::input::KeyStroke,
    setup::instance::{self, instance, khr_surface_instance, select_presentation_queues},
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

    // let entry = setup::instance::init();
    // let instance = setup::instance::instance(&entry);
    //
    // let best_dev = setup::instance::enumerate_physical_devs(&instance);
    // let physical_exts = setup::instance::find_extensions_supported_by_pdev(&instance, best_dev);
    // debug!("Physical extensions: ");
    // physical_exts.iter().for_each(|ext| debug!("\t{}", ext));
    //
    // let device_queues = setup::instance::select_physical_device_queues(&best_dev, &instance);
    // let logical_device =
    //     setup::instance::make_logical_device(&instance, &best_dev, &physical_exts, &device_queues);
    //
    // let surface_instance = setup::instance::xcb_surface_instance(&entry, &instance);
    // let khr_surface_instance = setup::instance::khr_surface_instance(&entry, &instance);
    // let vk_surface = setup::instance::xcb_surface(&surface_instance, _xcb_ptr, &window);
    //
    // let caps = setup::instance::map_physical_device_to_surface_properties(
    //     &khr_surface_instance,
    //     &best_dev,
    //     &vk_surface,
    // );
    // setup::instance::test_capabilities(&caps);
    // let surface_formats =
    //     setup::instance::find_formats_and_colorspaces(&khr_surface_instance, best_dev, &vk_surface);
    // let surface_queues = select_presentation_queues(
    //     &best_dev,
    //     &vk_surface,
    //     &device_queues,
    //     &khr_surface_instance,
    // );
    //
    // let swapchain_device = setup::instance::make_surface_device(&instance, &logical_device);
    // let swapchain = setup::instance::make_swapchain(
    //     &swapchain_device,
    //     vk_surface,
    //     &surface_formats,
    //     &device_queues,
    //     &caps,
    // );
    //
    // let swapchain_images = setup::instance::swapchain_images(&swapchain_device, swapchain);
    // debug!(
    //     "Count of images provided by the swapchain: {}",
    //     swapchain_images.len()
    // );
    //
    // let swapchain_views =
    //     setup::instance::image_views(&logical_device, &swapchain_images, surface_formats.format);
    //
    // debug!("Count of image views: {}", swapchain_views.len());

    // debug!(
    //     "Extents? {}x{}",
    //     caps.current_extent.width, caps.current_extent.height
    // );
    // debug!("Available transforms: {:?}", caps.supported_transforms);
    // debug!("Available image usages: {:?}", caps.supported_usage_flags);
    // debug!(
    //     "Available alpha values: {:?}",
    //     caps.supported_composite_alpha
    // );
    // debug!("Buffers? {}-{}", caps.min_image_count, caps.max_image_count);
    //
    // debug!("The...instance was created?");
    //
    // unsafe { swapchain_device.destroy_swapchain(swapchain, None) };
    // unsafe { khr_surface_instance.destroy_surface(vk_surface, None) };
    // unsafe { instance.destroy_instance(None) };
    //
    debug!("Vulkan instance destroyed...");
}
