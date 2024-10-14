use ash::vk::{Buffer, DeviceMemory, Image, MemoryAllocateInfo, MemoryPropertyFlags, MemoryType};

use log::debug;

use std::sync::Arc;

use crate::{dust_errors::DustError, setup::instance::VkContext};

use super::{image::DustImage, transfer::match_memory_type};

pub type ImageId = usize;
pub type BufferId = usize;

struct DeviceMemoryTombstone {
    pub device_meory: DeviceMemory,
    pub size: usize,
    pub first_free: usize,
}

struct ImageTombstone {
    image: Image,
    heap_type_index: usize,
    heap_index: usize,
    allocated_offset: usize,
    allocation_size: usize,
}

struct BufferTombstone {
    buffer: Buffer,
    heap_type_index: usize,
    heap_index: usize,
    allocated_offset: usize,
    alloation_size: usize,
}

pub struct DustObjectStore {
    ctxt: Arc<VkContext>,
    heap_growth: Vec<usize>,
    heaps: Vec<Vec<DeviceMemoryTombstone>>,
    types: Vec<MemoryType>,
}

pub fn new(ctxt: Arc<VkContext>) -> DustObjectStore {
    let mut type_vec = Vec::new();
    let mut heaps = Vec::new();
    let mut heap_growth = Vec::new();
    for memory_type in ctxt.physical_memory_properties.memory_types {
        type_vec.push(memory_type);
        heaps.push(Vec::new());
        heap_growth.push(10);
    }

    DustObjectStore {
        ctxt,
        heap_growth,
        heaps,
        types: type_vec,
    }
}

impl DustObjectStore {
    pub fn allocate_device_image(&self, image: Image) -> Result<ImageId, DustError> {
        let image_memory_requirements = unsafe {
            self.ctxt
                .logical_device
                .get_image_memory_requirements(image)
        };

        let memory_type_index = self.match_memory_type(
            image_memory_requirements.memory_type_bits,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );
    }

    fn allocate_new_heap(&self, memory_type_index: usize) -> Result<DeviceMemory, DustError> {
        let (memory_type, growth_amount) = match (
            self.types.get(memory_type_index),
            self.heap_growth.get(memory_type_index),
        ) {
            (Some(mem_type), Some(growth_amount)) => (mem_type, growth_amount),
            _ => {
                return Err(DustError::NoMatchingMemoryType);
            }
        };

        let allocate_info = MemoryAllocateInfo::default()
            .memory_type_index(memory_type_index as u32)
            .allocation_size(*growth_amount as u64);

        match unsafe {
            self.ctxt
                .logical_device
                .allocate_memory(&allocate_info, None)
        } {
            Ok(dm) => Ok(dm),
            Err(vk_result) => Err(DustError::DeviceMemoryAllocationFailed(vk_result)),
        }
    }

    pub fn match_memory_type(
        &self,
        filter: u32,
        matcher: MemoryPropertyFlags,
    ) -> Result<usize, DustError> {
        debug!("Testing for memory properties {:?}", matcher);
        for index in 0..self.types.len() {
            if (filter & 1 << index) == (1 << index)
                && (self.types[index as usize].property_flags & matcher) == matcher
            {
                return Ok(index);
            }
        }
        Err(DustError::NoMatchingMemoryType)
    }
}
