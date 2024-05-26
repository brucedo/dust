use log::debug;

mod input;
mod setup;

fn main() {
    env_logger::init();

    let entry = setup::instance::init();
    let instance = setup::instance::instance(&entry);

    debug!("The...instance was created?");

    // let instance = unsafe { vk_entry.create_instance(&instance_info_bldr, allocation_callbacks) };
    unsafe { instance.destroy_instance(None) };

    debug!("Vulkan instance destroyed...");
}
