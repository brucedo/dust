use ash::vk::{
    ApplicationInfo, InstanceCreateInfo, PhysicalDevice, PhysicalDeviceProperties,
    PhysicalDeviceType, SurfaceCapabilitiesKHR, SurfaceKHR, XcbSurfaceCreateInfoKHR,
};
use ash::{Entry, Instance};
use core::panic;
use log::{debug, warn};
use std::ffi::{c_void, CStr, CString};
use std::ops::Deref;
use xcb::ffi::xcb_connection_t;
use xcb::x::Window;
use xcb::Xid;

pub fn init() -> ash::Entry {
    debug!("Starting initialization");
    match unsafe { ash::Entry::load() } {
        Ok(entry) => entry,
        Err(msg) => {
            panic!("The Vulkan initialization process has failed: {}", msg);
        }
    }
}

#[cfg(all(target_os = "linux", not(target_os = "windows")))]
pub fn instance(entry: &ash::Entry) -> ash::Instance {
    use std::{
        thread::{self, sleep},
        time::Duration,
    };

    sleep(Duration::from_secs(10));

    debug!("Starting instance creation...");
    let app_name = CString::new("Dust for Linux").unwrap();
    let khr_surface_name = CString::new("VK_KHR_surface").unwrap();
    let khr_xcb_surface_name = CString::new("VK_KHR_xcb_surface").unwrap();
    let xcb_ext_name = [
        khr_surface_name.as_c_str().as_ptr(),
        khr_xcb_surface_name.as_c_str().as_ptr(),
    ];

    debug!("Extension names setup...");

    let app_info = app_info(app_name.as_c_str());

    debug!("App info struct filled");

    let instance_info = InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&xcb_ext_name);

    debug!("instance_info struct filled");

    match unsafe { entry.create_instance(&instance_info, None) } {
        Ok(instance) => {
            debug!("Instance successfully created?");
            instance
        }
        Err(msg) => panic!("Instance creation failed: {}", msg),
    }
}

pub fn xcb_surface_instance(entry: &Entry, instance: &Instance) -> ash::khr::xcb_surface::Instance {
    ash::khr::xcb_surface::Instance::new(entry, instance)
}

pub fn khr_surface_instance(entry: &Entry, instance: &Instance) -> ash::khr::surface::Instance {
    ash::khr::surface::Instance::new(entry, instance)
}

#[cfg(all(target_os = "windows", not(target_os = "linux")))]
pub fn instance() {
    let app_info = app_info(CString::new("Dust for Windows").unwrap().as_c_str());
    todo!();
}

#[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
pub fn instance() {
    panic!("No support for OSes other than Windows or Linux")
}

fn app_info(app_name: &CStr) -> ApplicationInfo {
    ApplicationInfo::default()
        .application_name(app_name)
        .application_version(1)
        .api_version(ash::vk::make_api_version(0, 1, 3, 0))
        .engine_name(app_name)
        .engine_version(1)
}

pub fn xcb_surface(
    instance: &ash::khr::xcb_surface::Instance,
    xcb_ptr: *mut xcb_connection_t,
    xcb_window: &Window,
) -> SurfaceKHR {
    let xcb_void: *mut std::ffi::c_void = xcb_ptr as *mut c_void;
    let surface_info_struct = XcbSurfaceCreateInfoKHR::default()
        .window(xcb_window.resource_id())
        .connection(xcb_void);

    match unsafe { instance.create_xcb_surface(&surface_info_struct, None) } {
        Ok(surface) => surface,
        Err(msg) => {
            panic!("Surface creation (predictably) failed: {:?}", msg)
        }
    }
}

pub fn enumerate_physical_devs(instance: &Instance) -> PhysicalDevice {
    let mut p_devs: Vec<PhysicalDevice> =
        if let Ok(enumerable) = unsafe { instance.enumerate_physical_devices() } {
            enumerable
        } else {
            panic!("Unable to retrieve the physical devices associated with this instance.");
        };

    if p_devs.is_empty() {
        panic!("There are no detected Vulkan compatible physical devices: cannot proceed.");
    }

    let mut best_pd: PhysicalDevice = p_devs.pop().unwrap();
    let pd_props = unsafe { instance.get_physical_device_properties(best_pd) };
    let mut best_score = shitty_score_physical_device_properties(&pd_props);
    while let Some(temp) = p_devs.pop() {
        let pd_props = unsafe { instance.get_physical_device_properties(temp) };
        let score = shitty_score_physical_device_properties(&pd_props);
        if score > best_score {
            best_score = score;
            best_pd = temp;
            debug!(
                "Swapping in device {} as best device",
                pd_props.device_name_as_c_str().unwrap().to_str().unwrap()
            )
        }
    }

    best_pd
}

pub fn map_physical_device_to_surface_properties(
    instance: &ash::khr::surface::Instance,
    device: &PhysicalDevice,
    surface: &SurfaceKHR,
) -> SurfaceCapabilitiesKHR {
    match unsafe { instance.get_physical_device_surface_capabilities(*device, *surface) } {
        Ok(surface_props) => surface_props,
        Err(msg) => {
            panic!(
                "Unable to get the physical device-surface capabilities: {}",
                msg
            );
        }
    }
}

fn shitty_score_physical_device_properties(device_props: &PhysicalDeviceProperties) -> usize {
    let mut score = 1;

    match device_props.device_type {
        PhysicalDeviceType::DISCRETE_GPU => {
            score *= 10;
        }
        PhysicalDeviceType::INTEGRATED_GPU => {
            score *= 5;
        }
        PhysicalDeviceType::CPU => {
            score *= 1;
        }
        _ => score *= 0,
    };

    match device_props.vendor_id {
        0x1002 | 0x1022 => {
            score += 5;
        }
        0x10de => {
            score += 3;
        }
        0x8086 => {
            score += 2;
        }
        _ => {
            score += 1;
        }
    }

    score
}

fn scan(vk_entry: &Entry) {
    match unsafe { vk_entry.try_enumerate_instance_version() } {
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

    let vk_layer_props = match unsafe { vk_entry.enumerate_instance_layer_properties() } {
        Ok(props) => props,
        Err(msg) => {
            panic!("ERR: {}", msg);
        }
    };

    let mut layer_names: Vec<Option<&CStr>> = vk_layer_props
        .iter()
        .map(|prop| unsafe { CStr::from_ptr(prop.layer_name.as_ptr()) })
        .map(|prop_name| Some(prop_name))
        .collect();

    layer_names.push(None);

    debug!("Size of found props: {}", layer_names.len());

    for layer_name in layer_names {
        debug!("{:?}", layer_name);
        match unsafe { vk_entry.enumerate_instance_extension_properties(layer_name) } {
            Ok(ext_props) => {
                debug!(
                    "Inspecting layer {:?} properties.  Count: {}",
                    layer_name,
                    ext_props.len()
                );
                for ext_prop in ext_props {
                    let prop_name_cstr =
                        unsafe { CStr::from_ptr(ext_prop.extension_name.as_ptr()) };
                    let layer_name_str = match layer_name {
                        Some(cstr) => cstr.to_str().unwrap(),
                        None => "NONE",
                    };
                    if let Ok(prop_ext_str) = prop_name_cstr.to_str() {
                        debug!(
                            "Found extension property {} for layer {}",
                            prop_ext_str, layer_name_str
                        );
                    }
                }
            }
            Err(msg) => {
                panic!("ERR for {:?}: {}", layer_name.unwrap(), msg)
            }
        }
    }
}
