use std::sync::OnceLock;

use ash::prelude::VkResult;
use ash::vk::{
    AllocationCallbacks, Buffer, BufferCreateFlags, BufferCreateInfo, BufferUsageFlags,
    CommandBuffer, CommandBufferAllocateInfo, CommandPool, SharingMode,
};
use ash::Device;

use crate::setup::instance::VkContext;

use super::image::new;

// type CommandBufferAllocator = fn(&CommandBufferAllocateInfo) -> VkResult<CommandBuffer>;
type CommandBufferAllocator = Fn<&CommandBufferAllocator> -> CommandBuffer;

static GRAPHICS_POOL: OnceLock<CommandPool> = OnceLock::new();
static TRANSFER_POOL: OnceLock<CommandPool> = OnceLock::new();

pub fn init(graphics_pool: CommandPool, transfer_pool: CommandPool, logical_device: Device) {
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

    let allocator: CommandBufferAllocator =
        |allocate_info| unsafe { logical_device.allocate_command_buffers(allocate_info) };
}

pub fn destroy(ctxt: &VkContext) {}

// pub fn reserve_graphics_buffer(ctxt: &VkContext) -> CommandBuffer {
// ctxt.logical_device.allocate_command_buffers()
// }
