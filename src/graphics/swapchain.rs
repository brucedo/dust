use ash::vk::Fence;
use ash::vk::Image;
use ash::vk::ImageView;
use ash::vk::PresentInfoKHR;
use ash::vk::Queue;
use ash::vk::Semaphore;
use ash::vk::SurfaceFormatKHR;
use ash::vk::SwapchainKHR;
use ash::Device;
use log::debug;
use std::sync::OnceLock;

use crate::setup::instance::VkContext;

// static DEVICE: OnceLock<Device> = OnceLock::new();
static SWAPCHAIN: OnceLock<SwapchainKHR> = OnceLock::new();
static SWAPCHAIN_DEVICE: OnceLock<ash::khr::swapchain::Device> = OnceLock::new();
static SWAPCHAIN_IMAGES: OnceLock<Vec<Image>> = OnceLock::new();
static SWAPCHAIN_SURFACE_FORMAT: OnceLock<SurfaceFormatKHR> = OnceLock::new();
static mut SWAPCHAIN_VIEWS: OnceLock<Vec<ImageView>> = OnceLock::new();

pub fn init(
    // logical_device: Device,
    swapchain: SwapchainKHR,
    swapcahin_device: ash::khr::swapchain::Device,
    swapchain_images: Vec<Image>,
    swapchain_views: Vec<ImageView>,
    swapchain_format: SurfaceFormatKHR,
) {
    // match DEVICE.set(logical_device) {
    //     Ok(_) => {}
    //     Err(_device) => {
    //         panic!("Failed to set the DEVICE static in the swapchain module.");
    //     }
    // };

    match SWAPCHAIN.set(swapchain) {
        Ok(_) => {}
        Err(_) => {
            panic!("Failed to set the SWAPCHAIN static in the swapchain module.");
        }
    };

    match SWAPCHAIN_DEVICE.set(swapcahin_device) {
        Ok(_) => {}
        Err(_) => {
            panic!("Failed to set the SWAPCHAIN_DEVICE static in the swapchain module.");
        }
    };

    match SWAPCHAIN_SURFACE_FORMAT.set(swapchain_format) {
        Ok(_) => {}
        Err(_) => {
            panic!("Failed to set the SWAPCHAIN_SURFACE_FORMAT static in the swapchain module");
        }
    };

    match SWAPCHAIN_IMAGES.set(swapchain_images) {
        Ok(_) => {}
        Err(_) => {
            panic!("Failed to assign the swapchain images to the static storage vector");
        }
    }

    match unsafe { SWAPCHAIN_VIEWS.set(swapchain_views) } {
        Ok(_) => {}
        Err(_) => {
            panic!("Failed to assign the swapchain views to the static storage vector",);
        }
    }
}

pub fn get_swapchain_format() -> &'static SurfaceFormatKHR {
    match SWAPCHAIN_SURFACE_FORMAT.get() {
        Some(fmt) => fmt,
        None => {
            panic!("It appears you have attempted to utilize the Swapchain module without initializing Vulkan.");
        }
    }
}

pub fn next_swapchain_image(
    signal_acquired: Semaphore,
    block_till_acquired: Fence,
) -> (u32, &'static ImageView, bool) {
    debug!(
        "Out of curiosity, how big are semaphore and fence? {} & {}",
        std::mem::size_of::<Semaphore>(),
        std::mem::size_of::<Fence>()
    );

    debug!(
        "Also out of curiosity - how big are swapchain device & swapchain? {}, {}",
        std::mem::size_of::<SwapchainKHR>(),
        std::mem::size_of::<ash::khr::swapchain::Device>()
    );

    debug!(
        "One more question: how big is this Image? {}",
        std::mem::size_of::<Image>()
    );

    debug!(
        "Last question: how big is just plain Device? {}",
        std::mem::size_of::<Device>()
    );

    let (image_index, suboptimal) = match unsafe {
        SWAPCHAIN_DEVICE.get().unwrap().acquire_next_image(
            *SWAPCHAIN.get().unwrap(),
            100000,
            signal_acquired,
            block_till_acquired,
        )
    } {
        Ok(index) => index,
        Err(msg) => {
            panic!("Failed to acquire next image index: {:?}", msg);
        }
    };

    let image = match unsafe { SWAPCHAIN_VIEWS.get().unwrap().get(image_index as usize) } {
        Some(image) => image,
        None => {
            panic!(
                "Failed to retrieve the actual image from the static - no image at index {} was found.", image_index
            );
        }
    };

    (image_index, image, suboptimal)
}

pub fn present_swapchain_image(
    image_index: u32,
    present_on: &Queue,
    wait_semaphores: &[Semaphore],
) -> Result<bool, ash::vk::Result> {
    let swapchain = [*SWAPCHAIN.get().unwrap(); 1];
    let images = [image_index; 1];
    let present_info = PresentInfoKHR::default()
        .swapchains(&swapchain)
        .image_indices(&images)
        .wait_semaphores(wait_semaphores);

    unsafe {
        SWAPCHAIN_DEVICE
            .get()
            .unwrap()
            .queue_present(*present_on, &present_info)
    }
}

pub fn destroy(ctxt: &VkContext) {
    debug!("Swapchain objects being destroyed.");
    unsafe {
        SWAPCHAIN_DEVICE
            .get()
            .unwrap()
            .destroy_swapchain(*SWAPCHAIN.get().unwrap(), None);

        let size = SWAPCHAIN_VIEWS.get().unwrap().len();
        SWAPCHAIN_VIEWS
            .get_mut()
            .unwrap()
            .drain(0..size)
            .for_each(|view| ctxt.logical_device.destroy_image_view(view, None));
    }
}
