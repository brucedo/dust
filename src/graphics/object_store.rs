use ash::vk::{DeviceMemory, MemoryAllocateInfo, MemoryPropertyFlags};

use crate::setup::instance::VkContext;

use super::{image::DustImage, transfer::match_memory_type};

pub struct DustObjectStore {
    images: [Option<DustImage>; 1000],
    free_image_slots: Vec<usize>,
    free_buffer_slots: Vec<usize>,
    heap: DeviceMemory,
    unused_offset: usize,
}

pub fn new(ctxt: &VkContext) -> DustObjectStore {


    let memory_type_index = ctxt.match_memory_type(, MemoryPropertyFlags::DEVICE_LOCAL) ;

    let allocate_memory_info = MemoryAllocateInfo::default()
        .allocation_size(104857600)
        .memory_type_index(memory_type_index)
    ctxt.logical_device.allocate_memory(allocate_info, allocation_callbacks)

    DustObjectStore {
        images: [None; 1000], 
        free_image_slots: vec![0..999], 
        free_buffer_slots: vec![0..999], 
        heap: , 
        unused_offset: 0
    }
}

