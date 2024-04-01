use ash::{
    vk::{ApplicationInfo, ApplicationInfoBuilder, InstanceCreateInfo, InstanceCreateInfoBuilder},
    Entry,
};
use log::debug;
use std::ffi::{CStr, CString};

fn main() {
    env_logger::init();

    let vk_entry = unsafe { ash::Entry::load() }.unwrap();

    scan(&vk_entry);

    let app_info_bldr = ApplicationInfo::builder()
        .application_name(&CString::new("Dust").unwrap())
        .application_version(1)
        .api_version(ash::vk::make_api_version(0, 1, 3, 0))
        .engine_name(&CString::new("Dust").unwrap())
        .engine_version(1);

    let instance_info_bldr = InstanceCreateInfo::builder();

    let instance = unsafe { vk_entry.create_instance(app_info_bldr.build(), allocation_callbacks) };
}

fn scan(vk_entry: &Entry) {
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

    let vk_layer_props = match vk_entry.enumerate_instance_layer_properties() {
        Ok(props) => props,
        Err(msg) => {
            panic!("ERR: {}", msg);
        }
    };

    let props: Vec<&CStr> = vk_layer_props
        .iter()
        .map(|prop| unsafe { CStr::from_ptr(prop.layer_name.as_ptr()) })
        .collect();

    debug!("Size of found props: {}", props.len());

    for prop in props {
        debug!("{:?}", prop);
        match vk_entry.enumerate_instance_extension_properties(Some(prop)) {
            Ok(ext_props) => {
                debug!(
                    "Inspecting layer {:?} properties.  Count: {}",
                    prop,
                    ext_props.len()
                );
                for ext_prop in ext_props {
                    let prop_name_cstr =
                        unsafe { CStr::from_ptr(ext_prop.extension_name.as_ptr()) };
                    if let (Ok(prop_str), Ok(prop_ext_str)) =
                        (prop.to_str(), prop_name_cstr.to_str())
                    {
                        debug!(
                            "Found extension property {} for layer {}",
                            prop_ext_str, prop_str
                        );
                    }
                }
            }
            Err(msg) => {
                panic!("ERR for {}: {}", prop.to_str().unwrap(), msg)
            }
        }
    }
}
