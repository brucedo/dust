use ash::vk::{
    ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo, Format, InstanceCreateInfo,
    PhysicalDevice, PhysicalDeviceFeatures, PhysicalDeviceProperties, PhysicalDeviceType,
    QueueFamilyProperties, QueueFlags, SurfaceCapabilitiesKHR, SurfaceKHR, SwapchainCreateFlagsKHR,
    SwapchainCreateInfoKHR, SwapchainKHR, XcbSurfaceCreateInfoKHR,
};
use ash::{Device, Entry, Instance};
use core::panic;
use log::{debug, error, warn};
use std::borrow::Borrow;
use std::ffi::{c_void, CStr, CString};
use std::ops::Deref;
use xcb::ffi::xcb_connection_t;
use xcb::x::Window;
use xcb::Xid;

use crate::setup::xcb_keymapper::new;

type Index = usize;
type Count = u32;

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

    // sleep(Duration::from_secs(10));

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

pub fn find_extensions_supported_by_pdev(
    instance: &Instance,
    p_dev: PhysicalDevice,
) -> Vec<String> {
    let mut extension_list = Vec::new();
    match unsafe { instance.enumerate_device_extension_properties(p_dev) } {
        Ok(props) => {
            debug!("Supported layers for physical device: ");
            for prop in props {
                let extension_name = unsafe { CStr::from_ptr(prop.extension_name.as_ptr()) };
                match extension_name.to_str() {
                    Ok(extension_name_str) => {
                        if is_wanted_extension(extension_name_str) {
                            extension_list.push(extension_name_str.to_string())
                        }
                        debug!("\t{}", extension_name_str);
                    }
                    Err(_) => {
                        debug!("\tUnconvertable extension name.");
                    }
                }
            }
        }
        Err(msg) => {
            error!("enumerate_device_extension_properties emitted an error when used with the selected physical device: {:?}", msg);
        }
    }

    extension_list
}

fn is_wanted_extension(ext_name: &str) -> bool {
    match ext_name {
        "VK_KHR_swapchain" => true,
        _ => false,
    }
}

pub fn make_logical_device(
    instance: &Instance,
    p_dev: PhysicalDevice,
    exts: Vec<String>,
    queue_selection: Vec<DeviceQueueCreateInfo>,
) -> Device {
    // This initial copy operation is required to give the CStrings a long-lived
    // place to stay.  Previous tries resulted in garbage being captured for the
    // extension names, because the CStrings are being allocated on the heap, then
    // dropped at the end of an arbitrary scope (either as a result of a closure or
    // because of the end of a loop scope, etc).  This meant that the raw pointers
    // we are capturing to give to Vulkan are up for grabs and are being reused
    // almost immediately - leading to the destruction of the text data.
    let mut cstr_temp_exts: Vec<CString> = exts
        .iter()
        .map(|ext| CString::new(ext.as_str()))
        .filter(Result::is_ok)
        .map(Result::unwrap)
        .collect();
    let exts_arr: Vec<*const i8> = cstr_temp_exts
        .iter()
        .map(CString::as_c_str)
        .map(|c_str| c_str.as_ptr())
        .collect();

    // let physical_features = setup_physical_features(instance);
    let physical_features = unsafe { instance.get_physical_device_features(p_dev) };

    let mut create_info = DeviceCreateInfo::default()
        .queue_create_infos(queue_selection.as_slice())
        .enabled_extension_names(&exts_arr)
        .enabled_features(&physical_features);

    match unsafe { instance.create_device(p_dev, &create_info, None) } {
        Ok(device) => device,
        Err(msg) => {
            panic!(
                "Could not construct logical device for physical device and options: {:?}",
                msg
            )
        }
    }
}

pub fn select_physical_device_queues<'a>(
    device: &'a PhysicalDevice,
    instance: &'a Instance,
) -> Vec<DeviceQueueCreateInfo<'a>> {
    let mut queue_selection = Vec::new();
    let queue_families = unsafe { instance.get_physical_device_queue_family_properties(*device) };

    for (index, family) in queue_families.iter().enumerate() {
        debug!(
            "Testing queue {}.  Properties/count: {:?}/{}",
            index, family.queue_flags, family.queue_count
        );
        if family.queue_flags == QueueFlags::TRANSFER | QueueFlags::COMPUTE | QueueFlags::GRAPHICS {
            debug!("Found matching queue.");
            let queue_count = u32::min(family.queue_count, 3);
            debug!("Requesting {} queues.", queue_count);
            let mut queue_create_info =
                DeviceQueueCreateInfo::default().queue_family_index(index as u32);
            queue_create_info.queue_count = queue_count;
            queue_create_info.queue_priorities(vec![0.5; queue_count as usize].as_slice());

            queue_selection.push(queue_create_info);
        }
    }
    queue_selection
}

pub fn find_formats_and_colorspaces(
    instance: &ash::khr::surface::Instance,
    p_dev: PhysicalDevice,
    surface: &SurfaceKHR,
) -> ash::vk::SurfaceFormatKHR {
    let formats = match unsafe { instance.get_physical_device_surface_formats(p_dev, *surface) } {
        Ok(formats) => formats,
        Err(msg) => {
            panic!(
                "Querying physical device & surface for supported formats failed: {}",
                msg
            );
        }
    };

    match formats
        .iter()
        .filter(|format| {
            format.format == ash::vk::Format::B8G8R8A8_SRGB
                || format.format == ash::vk::Format::R8G8B8A8_SRGB
        })
        .nth(0)
    {
        Some(format) => *format,
        None => {
            panic!("No 32-bit SRGB format was discovered - aborting launch.");
        }
    }
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
