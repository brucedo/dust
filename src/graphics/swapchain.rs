use ash::vk::SwapchainKHR;
use ash::Device;
use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::OnceLock;

static DEVICE: OnceLock<Arc<Device>> = OnceLock::new();
static SWAPCHAIN: OnceLock<SwapchainKHR> = OnceLock::new();
static SWAPCHAIN_DEVICE: OnceLock<ash::khr::swapchain::Device> = OnceLock::new();

pub fn init(
    logical_device: Arc<Device>,
    swapchain: SwapchainKHR,
    swapcahin_device: ash::khr::swapchain::Device,
) {
    match DEVICE.set(logical_device) {
        Ok(_) => {}
        Err(_device) => {
            panic!("Failed to set the DEVICE static in the swapchain module.");
        }
    };

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

pub fn destroy() {
    unsafe {
        SWAPCHAIN_DEVICE
            .get()
            .unwrap()
            .destroy_swapchain(SWAPCHAIN.take().unwrap(), None);
    }
}
