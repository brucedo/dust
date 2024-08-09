use std::mem::size_of;

use crate::dust_errors::DustError;
use ash::vk::{
    Buffer, BufferCreateFlags, BufferCreateInfo, BufferUsageFlags, MemoryPropertyFlags,
    PhysicalDeviceMemoryProperties, SharingMode,
};
use log::debug;

use crate::setup::instance::VkContext;

pub fn copy_to_buffer<T>(data: &[T], ctxt: &VkContext) -> Buffer
where
    T: Sized,
{
    let host_facing_buffer_info = BufferCreateInfo::default()
        .size(size_of::<T>() as u64)
        .usage(BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST)
        .sharing_mode(SharingMode::EXCLUSIVE)
        .flags(BufferCreateFlags::empty());

    let transfer_buffer = match unsafe {
        ctxt.logical_device
            .create_buffer(&host_facing_buffer_info, None)
    } {
        Ok(buffer) => buffer,
        Err(msg) => {
            panic!("Transfer buffer creation failed: {:?}", msg);
        }
    };

    let reqs = unsafe {
        ctxt.logical_device
            .get_buffer_memory_requirements(transfer_buffer)
    };

    let properties = match match_memory_type(
        &ctxt.physical_memory_properties,
        reqs.memory_type_bits,
        &(MemoryPropertyFlags::HOST_VISIBLE
            | MemoryPropertyFlags::HOST_COHERENT
            | MemoryPropertyFlags::DEVICE_LOCAL),
    ) {
        Ok(props) => props,
        Err(msg) => {
            panic!("A viable memory heap could not be found: {:?}", msg);
        }
    };

    unsafe {
        ctxt.logical_device.destroy_buffer(transfer_buffer, None);
    }
}

pub fn match_memory_type(
    memory_properties: &PhysicalDeviceMemoryProperties,
    filter: u32,
    matcher: &MemoryPropertyFlags,
) -> Result<u32, DustError> {
    debug!("Testing for memory properties {:?}", matcher);
    for index in 0..memory_properties.memory_type_count {
        if (filter & 1 << index) == (1 << index)
            && (memory_properties.memory_types[index as usize].property_flags & *matcher)
                == *matcher
        {
            return Ok(index);
        }
    }
    Err(DustError::NoMatchingMemoryType)
}
