use ash::vk::Fence;
use ash::vk::PresentInfoKHR;
use ash::vk::Queue;
use ash::vk::Semaphore;
use ash::vk::SwapchainKHR;
use ash::Device;
use log::debug;
use std::ops::Deref;
use std::sync::OnceLock;

// static DEVICE: OnceLock<Device> = OnceLock::new();
static SWAPCHAIN: OnceLock<SwapchainKHR> = OnceLock::new();
static SWAPCHAIN_DEVICE: OnceLock<ash::khr::swapchain::Device> = OnceLock::new();

pub fn init(
    // logical_device: Device,
    swapchain: SwapchainKHR,
    swapcahin_device: ash::khr::swapchain::Device,
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
}

pub fn next_swapchain_image(
    signal_acquired: Semaphore,
    block_till_acquired: Fence,
) -> (usize, bool) {
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

    (image_index as usize, suboptimal)
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

pub fn destroy() {
    debug!("Swapchain objects being destroyed.");
    unsafe {
        SWAPCHAIN_DEVICE
            .get()
            .unwrap()
            .destroy_swapchain(*SWAPCHAIN.get().unwrap(), None);
    }
}
