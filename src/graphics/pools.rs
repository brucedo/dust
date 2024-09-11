use std::sync::Arc;
use std::sync::OnceLock;

use ash::prelude::VkResult;
use ash::vk::{
    AllocationCallbacks, Buffer, BufferCreateFlags, BufferCreateInfo, BufferUsageFlags,
    CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool, SharingMode,
};
use ash::Device;

use crate::setup::instance::VkContext;

use super::image::new;

// type CommandBufferAllocator = fn(&CommandBufferAllocateInfo) -> VkResult<CommandBuffer>;

static GRAPHICS_POOL: OnceLock<CommandPool> = OnceLock::new();
static TRANSFER_POOL: OnceLock<CommandPool> = OnceLock::new();

pub fn init(graphics_pool: CommandPool, transfer_pool: CommandPool, logical_device: Arc<Device>) {
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
