use std::sync::Arc;
use std::sync::OnceLock;

use ash::vk::{CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool};
use ash::Device;

use crate::setup::instance::VkContext;

// type CommandBufferAllocator = fn(&CommandBufferAllocateInfo) -> VkResult<CommandBuffer>;

static GRAPHICS_POOL: OnceLock<CommandPool> = OnceLock::new();
static GRAPHICS_QUEUE_FAMILY: OnceLock<u32> = OnceLock::new();
static TRANSFER_POOL: OnceLock<CommandPool> = OnceLock::new();
static TRANSFER_QUEUE_FAMILY: OnceLock<u32> = OnceLock::new();

pub fn init(
    graphics_pool: CommandPool,
    graphics_queue_family: u32,
    transfer_pool: CommandPool,
    transfer_queue_family: u32,
    logical_device: Arc<Device>,
) {
    match GRAPHICS_POOL.set(graphics_pool) {
        Ok(_) => {}
        Err(_) => {
            panic!("Unable to set the graphics pool static.");
        }
    };

    match TRANSFER_POOL.set(transfer_pool) {
        Ok(_) => {}
        Err(_) => {
            panic!("Unable to set the transfer pool static.");
        }
    };

    match GRAPHICS_QUEUE_FAMILY.set(graphics_queue_family) {
        Ok(_) => {}
        Err(_) => {
            panic!("Unable to set the graphics queue family number.");
        }
    }

    match TRANSFER_QUEUE_FAMILY.set(transfer_queue_family) {
        Ok(_) => {}
        Err(_) => {
            panic!("Unable to set the transfer queue family number.");
        }
    }
}

pub fn destroy(ctxt: &VkContext) {
    unsafe {
        ctxt.logical_device
            .destroy_command_pool(*GRAPHICS_POOL.get().unwrap(), None);
        ctxt.logical_device
            .destroy_command_pool(*TRANSFER_POOL.get().unwrap(), None);

        // ctxt.transfer_queue_command_pools
        //     .drain(0..self.transfer_queue_command_pools.len())
        //     .for_each(|pool| self.logical_device.destroy_command_pool(pool, None));
        // ctxt.graphics_queue_command_pools
        //     .drain(0..self.graphics_queue_command_pools.len())
        //     .for_each(|pool| self.logical_device.destroy_command_pool(pool, None));
    }
}

pub fn reserve_graphics_buffer(ctxt: &VkContext) -> CommandBuffer {
    let alloc_info = CommandBufferAllocateInfo::default()
        .level(CommandBufferLevel::PRIMARY)
        .command_pool(*GRAPHICS_POOL.get().unwrap())
        .command_buffer_count(1);
    match unsafe { ctxt.logical_device.allocate_command_buffers(&alloc_info) } {
        Ok(mut buffer) => buffer.pop().unwrap(),
        Err(msg) => {
            panic!("Vulkan error while retrieving command buffer: {:?}", msg);
        }
    }
}

pub fn reserve_transfer_buffer(ctxt: &VkContext) -> CommandBuffer {
    let alloc_info = CommandBufferAllocateInfo::default()
        .level(CommandBufferLevel::PRIMARY)
        .command_pool(*TRANSFER_POOL.get().unwrap())
        .command_buffer_count(1);

    match unsafe { ctxt.logical_device.allocate_command_buffers(&alloc_info) } {
        Ok(mut buffer) => buffer.pop().unwrap(),
        Err(msg) => {
            panic!("Vulkan error while allocating transfer buffer: {:?}", msg);
        }
    }
}

pub fn get_transfer_queue_family() -> u32 {
    match TRANSFER_QUEUE_FAMILY.get() {
        Some(family) => *family,
        None => {
            panic!("Transfer family was never allocated.");
        }
    }
}

pub fn unsafe_get_transfer_queue_family() -> u32 {
    *TRANSFER_QUEUE_FAMILY.get().unwrap()
}

pub fn get_graphics_queue_family() -> u32 {
    match GRAPHICS_QUEUE_FAMILY.get() {
        Some(family) => *family,
        None => {
            panic!("Transfer family was never allocated.");
        }
    }
}
pub fn unsafe_get_graphics_queue_family() -> u32 {
    *GRAPHICS_QUEUE_FAMILY.get().unwrap()
}
