use std::sync::Arc;
use std::sync::OnceLock;

use ash::vk::DescriptorPool;
use ash::vk::DescriptorPoolCreateInfo;
use ash::vk::DescriptorPoolSize;
use ash::vk::DescriptorSet;
use ash::vk::DescriptorSetAllocateInfo;
use ash::vk::DescriptorSetLayout;
use ash::vk::DescriptorType;
use ash::vk::{CommandBuffer, CommandBufferAllocateInfo, CommandBufferLevel, CommandPool};
use ash::Device;

use crate::setup::instance::VkContext;

// type CommandBufferAllocator = fn(&CommandBufferAllocateInfo) -> VkResult<CommandBuffer>;

static GRAPHICS_POOL: OnceLock<CommandPool> = OnceLock::new();
static GRAPHICS_QUEUE_FAMILY: OnceLock<u32> = OnceLock::new();
static TRANSFER_POOL: OnceLock<CommandPool> = OnceLock::new();
static TRANSFER_QUEUE_FAMILY: OnceLock<u32> = OnceLock::new();
static DESCRIPTOR_SET_POOL: OnceLock<DescriptorPool> = OnceLock::new();
static LOGICAL_DEVICE: OnceLock<Arc<Device>> = OnceLock::new();

pub fn init(
    graphics_pool: CommandPool,
    graphics_queue_family: u32,
    transfer_pool: CommandPool,
    transfer_queue_family: u32,
    logical_device: Arc<Device>,
) {
    match (
        GRAPHICS_POOL.set(graphics_pool),
        TRANSFER_POOL.set(transfer_pool),
        GRAPHICS_QUEUE_FAMILY.set(graphics_queue_family),
        TRANSFER_QUEUE_FAMILY.set(transfer_queue_family),
        DESCRIPTOR_SET_POOL.set(allocate_descriptor_set_pool(&logical_device)),
        LOGICAL_DEVICE.set(logical_device),
    ) {
        (Ok(_), Ok(_), Ok(_), Ok(_), Ok(_), Ok(_)) => {}
        _ => {
            panic!("Unable to set the pool statics.");
        }
    };

    // match TRANSFER_POOL.set(transfer_pool) {
    //     Ok(_) => {}
    //     Err(_) => {
    //         panic!("Unable to set the transfer pool static.");
    //     }
    // };
    //
    // match GRAPHICS_QUEUE_FAMILY.set(graphics_queue_family) {
    //     Ok(_) => {}
    //     Err(_) => {
    //         panic!("Unable to set the graphics queue family number.");
    //     }
    // }
    //
    // match TRANSFER_QUEUE_FAMILY.set(transfer_queue_family) {
    //     Ok(_) => {}
    //     Err(_) => {
    //         panic!("Unable to set the transfer queue family number.");
    //     }
    // }
    //
    // match DESCRIPTOR_SET_POOL.set(allocate_descriptor_set_pool(logical_device)) {
    //     Ok(_) => {}
    //     Err(_) => {
    //         panic!("Unable to set the descriptor pool.");
    //     }
    // }
    //
    // match LOGICAL_DEVICE.set(logical_device) {
    //     Ok(_) => {},
    // }
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

fn allocate_descriptor_set_pool(device: &Arc<Device>) -> DescriptorPool {
    let descriptor_pool_sizes = [
        DescriptorPoolSize::default()
            .ty(DescriptorType::INPUT_ATTACHMENT)
            .descriptor_count(2),
        DescriptorPoolSize::default()
            .ty(DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(2),
        DescriptorPoolSize::default()
            .ty(DescriptorType::SAMPLER)
            .descriptor_count(2),
        DescriptorPoolSize::default()
            .ty(DescriptorType::SAMPLED_IMAGE)
            .descriptor_count(2),
        DescriptorPoolSize::default()
            .ty(DescriptorType::STORAGE_IMAGE)
            .descriptor_count(2),
    ];

    let pool_create_info = DescriptorPoolCreateInfo::default()
        .max_sets(10) // 10 is picked at random for now.
        .pool_sizes(&descriptor_pool_sizes);

    match unsafe { device.create_descriptor_pool(&pool_create_info, None) } {
        Ok(pool) => pool,
        Err(msg) => {
            panic!(
                "The call to construct a new descriptor pool has failed: {:?}",
                msg
            );
        }
    }
}

pub fn allocate_image_descriptor_set(layouts: &[DescriptorSetLayout]) -> Vec<DescriptorSet> {
    match (LOGICAL_DEVICE.get(), DESCRIPTOR_SET_POOL.get()) {
        (Some(device), Some(pool)) => {
            let allocate_info = DescriptorSetAllocateInfo::default()
                .descriptor_pool(*pool)
                .set_layouts(layouts);
            match unsafe { device.allocate_descriptor_sets(&allocate_info) } {
                Ok(descriptor_set) => descriptor_set,
                Err(msg) => {
                    panic!("Fix this in the future, but for now we cannot allocate a descriptor set so panic: {:?}", msg);
                }
            }
        }
        _ => {
            panic!("The device has not been set.  The Vulkan environment is not configured.  Cannot continue.");
        }
    }
}
