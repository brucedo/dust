use std::{array, mem::size_of, usize};

use crate::dust_errors::DustError;
use ash::vk::{
    Buffer, BufferCreateFlags, BufferCreateInfo, BufferUsageFlags, DeviceMemory,
    MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags, PhysicalDeviceMemoryProperties,
    SharingMode,
};
use log::debug;

use crate::setup::instance::VkContext;

pub fn copy_to_buffer<T>(data: &[T], ctxt: &VkContext) -> Buffer
where
    T: Sized + Copy + Clone,
{
    let size_in_bytes = std::mem::size_of_val(data) as u64;

    let transfer_buffer = make_buffer(
        ctxt,
        size_in_bytes,
        BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST,
    );

    let mem_handle = back_buffer_with_memory(
        ctxt,
        &transfer_buffer,
        size_in_bytes,
        &(MemoryPropertyFlags::HOST_VISIBLE
            | MemoryPropertyFlags::HOST_COHERENT
            | MemoryPropertyFlags::DEVICE_LOCAL),
    );

    let void_ptr = match unsafe {
        ctxt.logical_device
            .map_memory(mem_handle, 0, size_in_bytes, MemoryMapFlags::empty())
    } {
        Ok(ptr) => ptr,
        Err(msg) => {
            panic!("Failed to map the buffer backed memory to host: {:?}", msg);
        }
    };

    let t_ptr = void_ptr as *mut T;
    // let u8_ptr = void_ptr as *mut u8;
    let t_buf = unsafe { std::slice::from_raw_parts_mut(t_ptr, size_in_bytes as usize) };

    t_buf.copy_from_slice(data);

    unsafe {
        ctxt.logical_device.destroy_buffer(transfer_buffer, None);
    }

    transfer_buffer
}

fn back_buffer_with_memory(
    ctxt: &VkContext,
    buffer: &Buffer,
    buffer_size: u64,
    desired_properties: &MemoryPropertyFlags,
) -> DeviceMemory {
    let transfer_memory_requirements =
        unsafe { ctxt.logical_device.get_buffer_memory_requirements(*buffer) };

    let transfer_mem_type_index = match match_memory_type(
        &ctxt.physical_memory_properties,
        transfer_memory_requirements.memory_type_bits,
        desired_properties,
    ) {
        Ok(props) => props,
        Err(msg) => {
            panic!("A viable memory heap could not be found: {:?}", msg);
        }
    };

    let mem_alloc_info = MemoryAllocateInfo::default()
        .allocation_size(buffer_size)
        .memory_type_index(transfer_mem_type_index);

    let mem_handle = match unsafe { ctxt.logical_device.allocate_memory(&mem_alloc_info, None) } {
        Ok(handle) => handle,
        Err(msg) => {
            panic!("Unable to allocate buffer sized {}: {:?}", buffer_size, msg);
        }
    };

    match unsafe {
        ctxt.logical_device
            .bind_buffer_memory(*buffer, mem_handle, 0)
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!(
                "Failed to bind the buffer to the allocated memory: {:?}",
                msg
            );
        }
    }

    mem_handle
}

fn make_buffer(ctxt: &VkContext, buffer_size: u64, flags: BufferUsageFlags) -> Buffer {
    let transfer_buffer_info = BufferCreateInfo::default()
        .size(buffer_size)
        .usage(flags)
        .sharing_mode(SharingMode::EXCLUSIVE)
        .flags(BufferCreateFlags::empty());

    let transfer_buffer = match unsafe {
        ctxt.logical_device
            .create_buffer(&transfer_buffer_info, None)
    } {
        Ok(buffer) => buffer,
        Err(msg) => {
            panic!("Transfer buffer creation failed: {:?}", msg);
        }
    };

    transfer_buffer
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
