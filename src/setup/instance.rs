use ash::vk::{
    ApplicationInfo,
    // BufferCreateInfo,
    CommandBuffer,
    CommandBufferAllocateInfo,
    CommandBufferLevel,
    CommandPool,
    CommandPoolCreateFlags,
    CommandPoolCreateInfo,
    ComponentSwizzle,
    CompositeAlphaFlagsKHR,
    DeviceCreateInfo,
    DeviceQueueCreateInfo,
    Format,
    Image,
    ImageAspectFlags,
    // ImageCreateInfo,
    ImageUsageFlags,
    ImageView,
    ImageViewCreateInfo,
    ImageViewType,
    InstanceCreateInfo,
    MemoryPropertyFlags,
    PhysicalDevice,
    PhysicalDeviceMemoryProperties,
    PhysicalDeviceProperties,
    PhysicalDeviceType,
    PresentModeKHR,
    Queue,
    QueueFamilyProperties,
    QueueFlags,
    SharingMode,
    SurfaceCapabilitiesKHR,
    SurfaceFormatKHR,
    SurfaceKHR,
    SurfaceTransformFlagsKHR,
    SwapchainCreateFlagsKHR,
    SwapchainCreateInfoKHR,
    SwapchainKHR,
    XcbSurfaceCreateInfoKHR,
    // QUEUE_FAMILY_EXTERNAL,
};
use ash::{Device, Entry, Instance};
use core::panic;
use log::{debug, error};
use std::ffi::{c_void, CStr, CString};
use xcb::ffi::xcb_connection_t;
use xcb::x::Window;
use xcb::Xid;

use crate::dust_errors::DustError;

type Index = usize;
type Count = u32;
pub struct VkContext {
    entry: ash::Entry,
    instance: ash::Instance,
    physical_device: PhysicalDevice,
    pub physical_memory_properties: PhysicalDeviceMemoryProperties,
    physical_ext_names: Vec<String>,
    // device_queue_create_info: Vec<DeviceQueueCreateInfo<'a>>,
    pub graphics_queues: Vec<u32>,
    graphics_counts: Vec<u32>,
    graphics_priorities: Vec<Vec<f32>>,
    pub transfer_queues: Vec<u32>,
    transfer_counts: Vec<u32>,
    transfer_priorities: Vec<Vec<f32>>,
    // graphics_queue_create_infos: Vec<DeviceQueueCreateInfo<'a>>,
    // transfer_queue_create_infos: Vec<DeviceQueueCreateInfo<'a>>,
    pub logical_device: Device,
    pub graphics_queue: Queue,
    pub transfer_queue: Queue,
    khr_surface_instance: ash::khr::surface::Instance,
    surface: SurfaceKHR,
    pub surface_capabilities: SurfaceCapabilitiesKHR,
    surface_formats: SurfaceFormatKHR,
    // presentation_queues: Vec<&'a DeviceQueueCreateInfo<'a>>,
    pub swapchain_device: ash::khr::swapchain::Device,
    pub swapchain: SwapchainKHR,
    pub swapchain_images: Vec<Image>,
    pub swapchain_views: Vec<ImageView>,
    pub graphics_queue_command_pools: Vec<CommandPool>,
    pub transfer_queue_command_pools: Vec<CommandPool>,
    pub buffers: Vec<CommandBuffer>,
}

#[cfg(all(target_os = "linux", not(target_os = "windows")))]
pub fn default(xcb_ptr: *mut xcb_connection_t, xcb_window: &Window) -> VkContext {
    let entry: ash::Entry = init();
    let instance: ash::Instance = instance(&entry);

    let physical_device: PhysicalDevice = enumerate_physical_devs(&instance);
    let physical_memory_properties = get_physical_memory_properties(&instance, &physical_device);
    let physical_ext_names: Vec<String> =
        find_extensions_supported_by_pdev(&instance, physical_device);

    let queue_family_properties =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    show_queue_family_properties(&queue_family_properties);

    let (transfer_queues, transfer_queue_counts, transfer_priorities) = fill_queue_bits(
        &queue_family_properties,
        5,
        &QueueFlags::TRANSFER,
        &QueueFlags::GRAPHICS,
    );

    let (graphics_queues, graphics_queue_counts, graphics_priorities) = fill_queue_bits(
        &queue_family_properties,
        1,
        &QueueFlags::GRAPHICS,
        &QueueFlags::empty(),
    );

    debug!("Graphics queue indices: {:?}", graphics_queues);
    debug!("Graphics queue counts: {:?}", graphics_queue_counts);
    debug!("Graphics queue priorities: {:?}", graphics_priorities);

    // let transfer_queues = select_transfer_queues(&queue_family_properties);
    // let transfer_queue_counts =
    //     choose_transfer_queue_counts(&queue_family_properties, &transfer_queues);
    // let graphics_queues = select_graphics_queues(&queue_family_properties);
    // let graphics_queue_counts =
    //     choose_graphics_queue_counts(&queue_family_properties, &graphics_queues);
    //
    // let mut transfer_priorities = Vec::<Vec<f32>>::new();
    // for count in &transfer_queue_counts {
    //     transfer_priorities.push(vec![0.5; *count as usize]);
    // }

    // let mut graphics_priorities = Vec::<Vec<f32>>::new();
    // for count in &graphics_queue_counts {
    //     graphics_priorities.push(vec![0.5; *count as usize]);
    // }

    let transfer_queue_create_infos = construct_queue_create_info(
        &transfer_queues,
        &transfer_queue_counts,
        transfer_priorities.as_slice(),
    );

    transfer_queue_create_infos
        .iter()
        .for_each(|dq| debug!("Transfer queue priorities: {:?} ", dq.p_queue_priorities));

    let graphics_queue_create_infos = construct_queue_create_info(
        &graphics_queues,
        &graphics_queue_counts,
        &graphics_priorities,
    );

    graphics_queue_create_infos
        .iter()
        .for_each(|dq| debug!("Graphics queue priorities: {:?} ", dq.p_queue_priorities));

    let mut all_queue_create_info = Vec::new();
    for queue in &transfer_queue_create_infos {
        all_queue_create_info.push(*queue);
    }
    for queue in &graphics_queue_create_infos {
        all_queue_create_info.push(*queue);
    }

    let logical_device: Device = make_logical_device(
        &instance,
        &physical_device,
        &physical_ext_names,
        &all_queue_create_info,
    );

    let graphics_queue: Queue = get_queue(
        &logical_device,
        graphics_queues[0], // graphics_queue_create_infos.first().unwrap(),
    );

    let transfer_queue: Queue = get_queue(&logical_device, transfer_queues[0]);

    let xcb_surface_instance: ash::khr::xcb_surface::Instance =
        ash::khr::xcb_surface::Instance::new(&entry, &instance);
    let khr_surface_instance: ash::khr::surface::Instance =
        ash::khr::surface::Instance::new(&entry, &instance);
    let surface: SurfaceKHR = xcb_surface(&xcb_surface_instance, xcb_ptr, xcb_window);

    let surface_capabilities: SurfaceCapabilitiesKHR = map_physical_device_to_surface_properties(
        &khr_surface_instance,
        &physical_device,
        &surface,
    );
    let surface_formats: SurfaceFormatKHR =
        find_formats_and_colorspaces(&khr_surface_instance, physical_device, &surface);

    debug!("Checking queues for presentation-worthiness.");
    let presentation_queues: Vec<u32> = select_presentation_queues(
        &physical_device,
        &surface,
        &graphics_queues,
        &khr_surface_instance,
    );
    debug!("Input graphics queues: {:?}", graphics_queues);
    debug!("Presentation worthy queues: {:?}", presentation_queues);

    let swapchain_device: ash::khr::swapchain::Device =
        ash::khr::swapchain::Device::new(&instance, &logical_device);
    let swapchain: SwapchainKHR = make_swapchain(
        &swapchain_device,
        surface,
        &surface_formats,
        // &device_queue_create_info,
        &graphics_queues,
        &surface_capabilities,
    );

    let swapchain_images: Vec<Image> = swapchain_images(&swapchain_device, swapchain);
    let swapchain_views: Vec<ImageView> =
        image_views(&logical_device, &swapchain_images, surface_formats.format);

    let mut graphics_queue_command_pools = Vec::new();
    for queue_family in &graphics_queues {
        graphics_queue_command_pools.push(build_pools(*queue_family, &logical_device));
    }

    let mut transfer_queue_command_pools = Vec::new();
    for queue_family in &transfer_queues {
        transfer_queue_command_pools.push(build_pools(*queue_family, &logical_device));
    }

    let buffers = allocate_command_buffer(
        graphics_queue_command_pools.first().unwrap(),
        &logical_device,
    );

    VkContext {
        entry,
        instance,
        physical_device,
        physical_memory_properties,
        physical_ext_names,
        // device_queue_create_info,
        graphics_priorities,
        graphics_counts: graphics_queue_counts,
        graphics_queues,
        // graphics_queue_create_infos,
        transfer_queues,
        transfer_counts: transfer_queue_counts,
        transfer_priorities,
        // transfer_queue_create_infos,
        logical_device,
        graphics_queue,
        transfer_queue,
        khr_surface_instance,
        surface,
        surface_capabilities,
        surface_formats,
        // presentation_queues,
        swapchain_device,
        swapchain,
        swapchain_images,
        swapchain_views,
        graphics_queue_command_pools,
        transfer_queue_command_pools,
        buffers,
    }
}

fn fill_queue_bits(
    queue_family_properties: &[QueueFamilyProperties],
    desired_queue_count: u32,
    include_queue_types: &QueueFlags,
    exclude_queue_types: &QueueFlags,
) -> (Vec<u32>, Vec<u32>, Vec<Vec<f32>>) {
    debug!(
        "Selecting {} queues of type {:?}, excluding {:?}",
        desired_queue_count, include_queue_types, exclude_queue_types
    );
    let queue_indices = select_queue_families(
        queue_family_properties,
        include_queue_types,
        exclude_queue_types,
    );

    let mut counts = Vec::new();

    for index in &queue_indices {
        let temp = queue_family_properties.get(*index as usize).unwrap();
        let count = std::cmp::min(desired_queue_count, temp.queue_count);
        counts.push(count);
    }

    let mut queue_group_priorities = Vec::<Vec<f32>>::new();
    for count in &counts {
        queue_group_priorities.push(vec![0.5; *count as usize]);
    }

    (queue_indices, counts, queue_group_priorities)
}

#[cfg(all(target_os = "windows", not(target_os = "linux")))]
pub fn default() -> VkContext<'a> {}

#[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
pub fn default() {
    panic!("No support for OSes other than Windows or Linux")
}

impl Drop for VkContext {
    fn drop(&mut self) {
        debug!("Killing Vulkan objects.");
        unsafe {
            self.buffers.clear();
            self.transfer_queue_command_pools
                .drain(0..self.transfer_queue_command_pools.len())
                .for_each(|pool| self.logical_device.destroy_command_pool(pool, None));
            self.graphics_queue_command_pools
                .drain(0..self.graphics_queue_command_pools.len())
                .for_each(|pool| self.logical_device.destroy_command_pool(pool, None));
            self.swapchain_views
                .drain(0..self.swapchain_views.len())
                .for_each(|view| self.logical_device.destroy_image_view(view, None));
            self.swapchain_device
                .destroy_swapchain(self.swapchain, None);
            self.khr_surface_instance
                .destroy_surface(self.surface, None);
            self.logical_device.destroy_device(None);
            self.instance.destroy_instance(None);
        };
        debug!("Vulkan objects destroyed.");
    }
}

impl VkContext {
    pub fn match_memory_type(
        &self,
        filter: u32,
        matcher: &MemoryPropertyFlags,
    ) -> Result<u32, DustError> {
        debug!("Testing for memory properties {:?}", matcher);
        for index in 0..self.physical_memory_properties.memory_type_count {
            if (filter & 1 << index) == (1 << index)
                && (self.physical_memory_properties.memory_types[index as usize].property_flags
                    & *matcher)
                    == *matcher
            {
                return Ok(index);
            }
        }
        Err(DustError::NoMatchingMemoryType)
    }
}

fn init() -> ash::Entry {
    debug!("Starting initialization");
    match unsafe { ash::Entry::load() } {
        Ok(entry) => entry,
        Err(msg) => {
            panic!("The Vulkan initialization process has failed: {}", msg);
        }
    }
}

#[cfg(all(target_os = "linux", not(target_os = "windows")))]
fn instance(entry: &ash::Entry) -> ash::Instance {
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

#[cfg(all(target_os = "windows", not(target_os = "linux")))]
fn instance() {
    let app_info = app_info(CString::new("Dust for Windows").unwrap().as_c_str());
    todo!();
}

#[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
fn instance() {
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

fn xcb_surface(
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

fn enumerate_physical_devs(instance: &Instance) -> PhysicalDevice {
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

fn map_physical_device_to_surface_properties(
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

fn get_physical_memory_properties(
    instance: &Instance,
    physical_device: &PhysicalDevice,
) -> PhysicalDeviceMemoryProperties {
    unsafe { instance.get_physical_device_memory_properties(*physical_device) }
}

fn find_extensions_supported_by_pdev(instance: &Instance, p_dev: PhysicalDevice) -> Vec<String> {
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

// Just for now, to shut clippy up, this is matches.
// However, in the future we may want additional extensions
// to be loaded and I don't know how well matches! is going
// to scale across maybe a lot of branches.
fn is_wanted_extension(ext_name: &str) -> bool {
    matches!(ext_name, "VK_KHR_swapchain")
}

fn make_logical_device(
    instance: &Instance,
    p_dev: &PhysicalDevice,
    exts: &[String],
    queue_selection: &[DeviceQueueCreateInfo],
) -> Device {
    // This initial copy operation is required to give the CStrings a long-lived
    // place to stay.  Previous tries resulted in garbage being captured for the
    // extension names, because the CStrings are being allocated on the heap, then
    // dropped at the end of an arbitrary scope (either as a result of a closure or
    // because of the end of a loop scope, etc).  This meant that the raw pointers
    // we are capturing to give to Vulkan are up for grabs and are being reused
    // almost immediately - leading to the destruction of the text data.
    let cstr_temp_exts: Vec<CString> = exts
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
    let physical_features = unsafe { instance.get_physical_device_features(*p_dev) };

    let create_info = DeviceCreateInfo::default()
        .queue_create_infos(queue_selection)
        .enabled_extension_names(&exts_arr)
        .enabled_features(&physical_features);

    match unsafe { instance.create_device(*p_dev, &create_info, None) } {
        Ok(device) => device,
        Err(msg) => {
            panic!(
                "Could not construct logical device for physical device and options: {:?}",
                msg
            )
        }
    }
}

fn show_queue_family_properties(queue_families: &[QueueFamilyProperties]) {
    for (index, queue) in queue_families.iter().enumerate() {
        debug!("Queue Family {}", index);
        debug!(" Flags         -> {:?}", queue.queue_flags);
        debug!(" Count         -> {:?}", queue.queue_count);
        debug!(
            " Granularity   -> {:?}",
            queue.min_image_transfer_granularity
        );
    }
}

fn select_transfer_queues(queue_families: &[QueueFamilyProperties]) -> Vec<u32> {
    select_queue_families(queue_families, &QueueFlags::TRANSFER, &QueueFlags::GRAPHICS)
    // let mut transfer_indices = Vec::new();
    //
    // for (index, element) in queue_families.iter().enumerate() {
    //     if element.queue_flags.contains(QueueFlags::TRANSFER)
    //         && !element.queue_flags.contains(QueueFlags::GRAPHICS)
    //     {
    //         debug!("Found pure transfer queue.");
    //         transfer_indices.push(index as u32);
    //     }
    // }
    //
    // transfer_indices
}

fn select_queue_families(
    queue_families: &[QueueFamilyProperties],
    include_types: &QueueFlags,
    exclude_types: &QueueFlags,
) -> Vec<u32> {
    let mut transfer_indices = Vec::new();

    debug!(
        "Attempting to select queues including family type {:?}, excluding {:?}",
        include_types, exclude_types
    );

    for (index, element) in queue_families.iter().enumerate() {
        if element.queue_flags & *include_types == *include_types
            && element.queue_flags & *exclude_types == QueueFlags::empty()
        // if element.queue_flags.contains(*include_types)
        //     && !element.queue_flags.contains(*exclude_types)
        {
            debug!("Found pure transfer queue.");
            transfer_indices.push(index as u32);
        }
    }

    transfer_indices
}

fn choose_transfer_queue_counts(
    queue_families: &[QueueFamilyProperties],
    transfer_indices: &[u32],
) -> Vec<u32> {
    let mut counts = Vec::new();

    for index in transfer_indices {
        let temp = queue_families.get(*index as usize).unwrap();
        let count = std::cmp::min(5, temp.queue_count);
        counts.push(count);
    }

    counts
}

fn select_graphics_queues(queue_families: &[QueueFamilyProperties]) -> Vec<u32> {
    select_queue_families(queue_families, &QueueFlags::TRANSFER, &QueueFlags::empty())
    // let mut graphics_indices = Vec::new();
    //
    // for (index, element) in queue_families.iter().enumerate() {
    //     if element.queue_flags.contains(QueueFlags::GRAPHICS) {
    //         debug!("Found graphics queue.");
    //         graphics_indices.push(index as u32);
    //     }
    // }
    //
    // graphics_indices
}

fn choose_graphics_queue_counts(
    _queue_families: &[QueueFamilyProperties],
    queue_indices: &[u32],
) -> Vec<u32> {
    let mut graphics_counts = Vec::new();

    for _index in queue_indices {
        // let temp = queue_families[(*index) as usize];
        graphics_counts.push(1);
    }

    graphics_counts
}

fn construct_queue_create_info<'a, 'b>(
    queue_indices: &[u32],
    queue_counts: &[u32],
    priorities: &'a [Vec<f32>],
) -> Vec<DeviceQueueCreateInfo<'b>>
where
    'a: 'b,
{
    let mut queue_create_infos = Vec::new();

    for (iter_index, queue_index) in queue_indices.iter().enumerate() {
        let queue_count = queue_counts.get(iter_index).unwrap();
        let queue_priorities = priorities.get(iter_index).unwrap();

        debug!("Creating queue request struct for queue_index {}; requesting {} queues at priorities {:?}", queue_index, queue_count, queue_priorities);
        let mut queue_create_info = DeviceQueueCreateInfo::default()
            .queue_family_index(*queue_index)
            .queue_priorities(queue_priorities);
        queue_create_info.queue_count = *queue_count;

        debug!("queue_priority: {:?}", queue_create_info.p_queue_priorities);
        queue_create_infos.push(queue_create_info);
    }

    queue_create_infos
}

fn get_queue(device: &Device, reference_info: u32) -> Queue {
    // let family_index = reference_info.queue_family_index;
    let queue_index = 0;

    unsafe { device.get_device_queue(reference_info, queue_index) }
}

fn select_presentation_queues(
    device: &'_ PhysicalDevice,
    surface: &'_ SurfaceKHR,
    queues_to_test: &[u32],
    instance: &ash::khr::surface::Instance,
) -> Vec<u32> {
    debug!("Filtering selected queues for those that are presentation queues");
    debug!(
        "{} queues have been presented for review.",
        queues_to_test.len()
    );
    let mut presentation_queues = Vec::new();

    for queue_family in queues_to_test {
        debug!("  Testing queue {}", queue_family);
        match unsafe {
            instance.get_physical_device_surface_support(*device, *queue_family, *surface)
        } {
            Ok(true) => {
                debug!(
                    "Queue index {} supports writing to a surface.",
                    queue_family
                );
                presentation_queues.push(*queue_family);
            }
            Ok(false) => {
                debug!(
                    "Queue index {} does not support writing to a surface.",
                    queue_family
                )
            }
            Err(msg) => {
                error!("Querying the physical device's support for surface-writing queues generated error {:?}", msg)
            }
        }
    }

    presentation_queues
}

fn find_formats_and_colorspaces(
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

    match formats.iter().find(|format| {
        format.format == ash::vk::Format::B8G8R8A8_SRGB
            || format.format == ash::vk::Format::R8G8B8A8_SRGB
    }) {
        Some(format) => *format,
        None => {
            panic!("No 32-bit SRGB format was discovered - aborting launch.");
        }
    }
}

fn test_capabilities(surface_capabilities: &SurfaceCapabilitiesKHR) {
    if surface_capabilities.min_image_count < 2 {
        panic!("Double buffering unsupported by the surface.");
    }
    if !surface_capabilities
        .supported_usage_flags
        .contains(ImageUsageFlags::TRANSFER_DST)
    {
        panic!("Unable to use surface as a transfer destination.")
    }
    if !surface_capabilities
        .supported_transforms
        .contains(SurfaceTransformFlagsKHR::IDENTITY)
    {
        panic!("Unable to display image with 0° rotation.");
    }
    if !surface_capabilities
        .supported_composite_alpha
        .contains(CompositeAlphaFlagsKHR::OPAQUE)
    {
        panic!("Unable to display opaque/non-transparent image.");
    }
}

fn make_swapchain(
    device: &ash::khr::swapchain::Device,
    surface: SurfaceKHR,
    formatting: &SurfaceFormatKHR,
    queue_families: &[u32],
    surface_capabilities: &SurfaceCapabilitiesKHR,
) -> SwapchainKHR {
    let swapchain_info = SwapchainCreateInfoKHR::default()
        .flags(SwapchainCreateFlagsKHR::empty())
        .surface(surface)
        .min_image_count(surface_capabilities.min_image_count)
        .image_format(formatting.format)
        .image_color_space(formatting.color_space)
        .image_extent(surface_capabilities.current_extent)
        .image_array_layers(1)
        .image_usage(ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(SharingMode::EXCLUSIVE)
        .queue_family_indices(queue_families)
        .pre_transform(surface_capabilities.current_transform)
        .composite_alpha(CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(PresentModeKHR::MAILBOX)
        .clipped(true);

    match unsafe { device.create_swapchain(&swapchain_info, None) } {
        Ok(sc) => sc,
        Err(msg) => {
            panic!("Swapchain creation failed: {:?}", msg);
        }
    }
}

fn swapchain_images(
    device: &ash::khr::swapchain::Device,
    swapchain: ash::vk::SwapchainKHR,
) -> Vec<Image> {
    match unsafe { device.get_swapchain_images(swapchain) } {
        Ok(images) => images,
        Err(msg) => {
            panic!(
                "Could not retrieve the collection of swapchian images: {:?}",
                msg
            )
        }
    }
}

fn image_views(device: &ash::Device, images: &[Image], surface_format: Format) -> Vec<ImageView> {
    let mut views = Vec::with_capacity(images.len());

    for image in images {
        let create_info = ImageViewCreateInfo::default()
            .image(*image)
            .view_type(ImageViewType::TYPE_2D)
            .format(surface_format)
            .components(ash::vk::ComponentMapping {
                r: ComponentSwizzle::IDENTITY,
                g: ComponentSwizzle::IDENTITY,
                b: ComponentSwizzle::IDENTITY,
                a: ComponentSwizzle::IDENTITY,
            })
            .subresource_range(ash::vk::ImageSubresourceRange {
                aspect_mask: ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        match unsafe { device.create_image_view(&create_info, None) } {
            Ok(view) => views.push(view),
            Err(msg) => {
                panic!("An image view creation failed: {:?}", msg);
            }
        }
    }

    views
}

fn build_pools(queue_family: u32, device: &ash::Device) -> CommandPool {
    let pool_create_info = CommandPoolCreateInfo::default()
        .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(queue_family);

    match unsafe { device.create_command_pool(&pool_create_info, None) } {
        Ok(pool) => pool,
        Err(msg) => {
            panic!("The command pool creation step failed: {:?}", msg);
        }
    }
}

fn allocate_command_buffer(command_pool: &CommandPool, device: &ash::Device) -> Vec<CommandBuffer> {
    let buffer_info = CommandBufferAllocateInfo::default()
        .command_pool(*command_pool)
        .level(CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    match unsafe { device.allocate_command_buffers(&buffer_info) } {
        Ok(buffer) => buffer,
        Err(msg) => {
            panic!("Command buffer allocation from pool failed: {:?}", msg);
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
        .map(Some)
        // .map(|prop_name| Some(prop_name))
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
