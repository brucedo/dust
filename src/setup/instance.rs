use ash::vk::{ApplicationInfo, InstanceCreateInfo};
use std::ffi::{CStr, CString};

#[cfg(all(target_os = "linux", not(target_os = "windows")))]
pub fn instance() {
    let app_name = CString::new("Dust for Linux").unwrap();
    let app_info = app_info(app_name.as_c_str());

    let instance_info = InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_layer_names(&[CString::new("VK_KHR_xcb_surface")
            .unwrap()
            .as_c_str()
            .as_ptr()]);
    todo!();
}

#[cfg(all(target_os = "windows", not(target_os = "linux")))]
pub fn instance() {
    let app_info = app_info(CString::new("Dust for Windows").unwrap().as_c_str());
    todo!();
}

pub fn app_info(app_name: &CStr) -> ApplicationInfo {
    ApplicationInfo::default()
        .application_name(app_name)
        .application_version(1)
        .api_version(ash::vk::make_api_version(0, 1, 3, 0))
        .engine_name(app_name)
        .engine_version(1)
}
