use crate::setup::instance::VkContext;

use ash::vk::{
    Fence, FenceCreateFlags, FenceCreateInfo, Semaphore, SemaphoreCreateFlags, SemaphoreCreateInfo,
};

pub fn create_fence(ctxt: &VkContext) -> Fence {
    match unsafe {
        ctxt.logical_device.create_fence(
            &FenceCreateInfo::default().flags(FenceCreateFlags::empty()),
            None,
        )
    } {
        Ok(fence) => fence,
        Err(msg) => {
            panic!("Unable to create fence: {:?}", msg);
        }
    }
}

pub fn create_binary_semaphore(ctxt: &VkContext) -> Semaphore {
    match unsafe {
        ctxt.logical_device.create_semaphore(
            &SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::empty()),
            None,
        )
    } {
        Ok(sem) => sem,
        Err(msg) => {
            panic!("Unable to create semaphore: {:?}", msg);
        }
    }
}
