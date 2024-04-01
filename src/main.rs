use std::str;

fn main() {
    let vk_entry = unsafe { ash::Entry::load() }.unwrap();
    match vk_entry.try_enumerate_instance_version() {
        Ok(version_opt) => {
            if let Some(version) = version_opt {
                println!(
                    "Vulkan version detected: {}.{}.{}",
                    ash::vk::api_version_major(version),
                    ash::vk::api_version_minor(version),
                    ash::vk::api_version_patch(version),
                )
            } else {
                println!("None version returned")
            }
        }
        Err(msg) => {
            println!("VkResult::err: {}", msg)
        }
    }

    let props = match vk_entry.enumerate_instance_layer_properties() {
        Ok(prop_list) => prop_list
            .iter()
            .map(|cstr| unsafe { std::ffi::CStr::from_ptr(cstr.layer_name.as_ptr()) })
            .collect(),
        Err(msg) => {
            panic!("ERR");
        }
    };
}
