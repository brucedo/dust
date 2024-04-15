use ash::vk::{ApplicationInfo, InstanceCreateInfo};
use std::ffi::{CStr, CString};

#[cfg(all(target_os = "linux", not(target_os = "windows")))]
pub fn instance(entry: &ash::Entry) -> ash::Instance {
    use log::debug;

    let app_name = CString::new("Dust for Linux").unwrap();
    let khr_surface_name = CString::new("VK_KHR_surface").unwrap();
    let khr_xcb_surface_name = CString::new("VK_KHR_xcb_surface").unwrap();
    let xcb_ext_name = [
        khr_surface_name.as_c_str().as_ptr(),
        khr_xcb_surface_name.as_c_str().as_ptr(),
    ];

    let app_info = app_info(app_name.as_c_str());

    let instance_info = InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&xcb_ext_name);

    match unsafe { entry.create_instance(&instance_info, None) } {
        Ok(instance) => instance,
        Err(msg) => panic!("Instance creation failed: {}", msg),
    }
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

pub fn app_info(app_name: &CStr) -> ApplicationInfo {
    ApplicationInfo::default()
        .application_name(app_name)
        .application_version(1)
        .api_version(ash::vk::make_api_version(0, 1, 3, 0))
        .engine_name(app_name)
        .engine_version(1)
}
