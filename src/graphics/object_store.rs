use ash::{vk::{Buffer, DeviceMemory, HeadlessSurfaceCreateFlagsEXT, Image, MemoryAllocateInfo, MemoryPropertyFlags, MemoryRequirements, MemoryType, PhysicalDeviceMemoryProperties}, Device};

use log::debug;

use std::sync::Arc;

use crate::{dust_errors::DustError, setup::instance::VkContext};

pub type ImageId = usize;
pub type BufferId = usize;

struct DeviceMemoryTombstone {
    pub device_memory: DeviceMemory,
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
    allocation_size: usize,
}

pub struct DustObjectStore {
    device: Arc<Device>,
    heap_growth: Vec<usize>,
    heaps: Vec<Vec<DeviceMemoryTombstone>>,
    types: Vec<MemoryType>,
    images: Vec<Option<ImageTombstone>>, 
    buffers: Vec<Option<BufferTombstone>>
}

pub fn new(device: Arc<Device>, physical_memory_properties: &PhysicalDeviceMemoryProperties) -> DustObjectStore {
    let mut type_vec = Vec::new();
    let mut heaps = Vec::new();
    let mut heap_growth = Vec::new();
    for memory_type in physical_memory_properties.memory_types
    {
        type_vec.push(memory_type);
        heaps.push(Vec::new());
        heap_growth.push(10485760);
    }

    DustObjectStore {
        device ,
        heap_growth,
        heaps,
        types: type_vec,
        images: Vec::with_capacity(1000), 
        buffers: Vec::with_capacity(1000)
    }
}

impl DustObjectStore {
    pub fn deallocate_image(&mut self, image_id: ImageId) {
        let tombstone_opt = match self.images.get_mut(image_id) {
            Some(tombstone) => tombstone, 
            None => {return}
        };

        match tombstone_opt {
            Some(tombstone) => {
                unsafe {self.device.destroy_image(tombstone.image, None);}
            }
            None => {}
        }

        self.images.insert(image_id, None);
    }
    pub fn allocate_device_image(&mut self, image: Image) -> Result<ImageId, DustError> {
        let image_memory_requirements = unsafe {
            self.device
                .get_image_memory_requirements(image)
        };

        let memory_type_index = self.match_memory_type(
            image_memory_requirements.memory_type_bits,
            MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        match self.try_allocate(image, &image_memory_requirements, memory_type_index) {
            Err(DustError::NoValidHeapForAllocation) => {
                self.allocate_new_heap(memory_type_index)?;
                self.try_allocate(image, &image_memory_requirements, memory_type_index)
            }, 
            any_other => any_other
        }
    }

    pub fn get_image(&self, image_id: ImageId) -> Option<Image> {
        match self.images.get(image_id) {
            Some(image_tombstone_opt) =>
                match image_tombstone_opt {
                    Some(image_tombstone) => Some(image_tombstone.image), 
                    None => None, 
                } 
            None => None 
        }
    }

    fn try_allocate(&mut self, image: Image, memory_requirements: &MemoryRequirements, type_index: usize) -> Result<ImageId, DustError> {
        // let image_memory_requirements = unsafe {
        //     self.device
        //         .get_image_memory_requirements(image)
        // };

        let image_alignment = memory_requirements.alignment as usize;
        let image_size = memory_requirements.size as usize;

        // let memory_type_index = self.match_memory_type(
        //     image_memory_requirements.memory_type_bits,
        //     MemoryPropertyFlags::DEVICE_LOCAL,
        // )?;

        let (tombstone, alignment_padding, heap_index) = match self.heaps.get_mut(type_index) {
            Some(heaps) => {
                let future_index = heaps.len() - 1;
                let tombstone :&mut DeviceMemoryTombstone = match heaps.last_mut() {
                    Some(tombstone) =>  {
                        tombstone
                    }, 
                    None => {
                        return Err(DustError::NoValidHeapForAllocation);
                    }
                };

                let alignment_padding = image_alignment - (tombstone.first_free % image_alignment);
                if tombstone.first_free + alignment_padding + image_size > tombstone.size {
                    return Err(DustError::NoValidHeapForAllocation);
                }
                else {
                    (tombstone, alignment_padding, future_index)
                }
            }, 
            None => {unreachable!("There will always be a vec of heaps, though it may be empty.");}
        };

        unsafe {self.device.bind_image_memory(image, tombstone.device_memory, (tombstone.first_free + alignment_padding) as u64)}; 
        tombstone.first_free += alignment_padding + image_size; 
            
        let image_tombstone = ImageTombstone {
            image, 
            heap_type_index: type_index, 
            heap_index , 
            allocated_offset: tombstone.first_free + alignment_padding, 
            allocation_size: memory_requirements.size as usize
        };

        self.images.push(Some(image_tombstone));

        Ok(self.images.len() - 1)
    }

    fn allocate_new_heap(
        &mut self,
        memory_type_index: usize,
    ) -> Result<(), DustError> {
        let growth_amount = match 
            // self.types.get(memory_type_index),
            self.heap_growth.get(memory_type_index)
         {
            Some(growth_amount) => growth_amount,
            _ => {
                return Err(DustError::NoMatchingMemoryType);
            }
        };

        let allocation_size = *growth_amount ;

        let allocate_info = MemoryAllocateInfo::default()
            .memory_type_index(memory_type_index as u32)
            .allocation_size(allocation_size as u64);

        let device_memory = match unsafe {
            self.device
                .allocate_memory(&allocate_info, None)
        } {
            Ok(dm) => dm,
            Err(vk_result) => return Err(DustError::DeviceMemoryAllocationFailed(vk_result)),
        };

        let tombstone = DeviceMemoryTombstone {
            device_memory, 
            size: allocation_size, 
            first_free: 0
        };

        match self.heaps.get_mut(memory_type_index) {
            Some(heaps_for_type) => {
                heaps_for_type.push(tombstone);
            }
            None => {
                unreachable!("All memory type indices were initialized to a heap.");
            }
        }

        self.heap_growth.insert(memory_type_index, 2 * growth_amount);
        
        Ok(())
    }

    pub fn deallocate_buffer(&mut self, buffer_id: BufferId) {
        let buffer_opt = match self.buffers.get(buffer_id) {
            Some(buffer_opt) => buffer_opt, 
            None => {return}
        };

        if let Some(buffer_tombstone) = buffer_opt {
            unsafe {self.device.destroy_buffer(buffer_tombstone.buffer, None)};
            self.buffers.insert(buffer_id, None);
        }
    }

    pub fn allocate_device_buffer(&mut self, buffer: Buffer) -> Result<BufferId, DustError> {
        self.allocate_buffer(buffer, MemoryPropertyFlags::DEVICE_LOCAL)
    }

    pub fn allocate_local_cached_buffer(&mut self, buffer: Buffer) -> Result<BufferId, DustError> {
        self.allocate_buffer(buffer, MemoryPropertyFlags::DEVICE_LOCAL | MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_CACHED)
    }

    pub fn allocate_local_coherent_buffer(&mut self, buffer: Buffer) -> Result<BufferId, DustError> {
        self.allocate_buffer(buffer, MemoryPropertyFlags::DEVICE_LOCAL | MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT)
    }

    pub fn allocate_local_visible_buffer(&mut self, buffer: Buffer) -> Result<BufferId, DustError> {
        self.allocate_buffer(buffer, MemoryPropertyFlags::DEVICE_LOCAL | MemoryPropertyFlags::HOST_VISIBLE)
    }

    pub fn get_buffer(&self, buffer_id: BufferId) -> Option<Buffer> {
        match self.buffers.get(buffer_id) {
            Some(buffer_tombstone_opt) => {
                match buffer_tombstone_opt {
                    Some(buffer_tombstone) => Some(buffer_tombstone.buffer), 
                    None => None
                }
            }, 
            None => None
        }
    }

    fn allocate_buffer(&mut self, buffer: Buffer, required_flags: MemoryPropertyFlags) -> Result<BufferId, DustError> {
        let mem_requirements = unsafe {self.device.get_buffer_memory_requirements(buffer)};

        let type_index = self.match_memory_type(mem_requirements.memory_type_bits, required_flags)?;

        match self.try_allocate_buffer(buffer, &mem_requirements, type_index) {
            Err(DustError::NoValidHeapForAllocation) => {
                self.allocate_new_heap(type_index)?;
                self.try_allocate_buffer(buffer, &mem_requirements, type_index)
            }
             anything_else => anything_else
        }
    }

    fn try_allocate_buffer(&mut self, buffer: Buffer, requirements: &MemoryRequirements, type_index: usize) -> Result<BufferId, DustError> {

        let buffer_size = requirements.size as usize;
        let required_alignment = requirements.alignment as usize;
        
        let (device_memory_tombstone, alignment_padding, heap_index) = match self.heaps.get_mut(type_index) {
            Some(heaps) => {
                let next_index = heaps.len() - 1;

                match heaps.last_mut() {
                    Some(tombstone) => {
                        let alignment_padding = required_alignment - (tombstone.first_free % required_alignment);
                        if tombstone.size < tombstone.first_free + alignment_padding + buffer_size {
                            return Err(DustError::NoValidHeapForAllocation); 
                        }
                        else {
                            (tombstone, alignment_padding, next_index)
                        }
                    }
                    None => {return Err(DustError::NoValidHeapForAllocation); }
                }
            }, 
            None => {
                unreachable!("Every heap is initialized with an empty vec, therefore a type_indexed lookup should never find None.");
            }
        };

        unsafe {self.device.bind_buffer_memory(
            buffer, 
            device_memory_tombstone.device_memory, 
            (device_memory_tombstone.first_free + alignment_padding) as u64);
        }

        let buffer_tombstone = BufferTombstone {
            buffer, 
            allocated_offset: device_memory_tombstone.first_free + alignment_padding, 
            allocation_size: buffer_size, 
            heap_type_index: type_index, 
            heap_index: heap_index, 
        };

        self.buffers.push(Some(buffer_tombstone));

        Ok(self.buffers.len() - 1)

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
