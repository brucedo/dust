use std::rc::Rc;
use std::sync::Arc;

use ash::{
    vk::{DeviceMemory, Image},
    Device,
};

pub struct DustImage {
    pub image: Image,
    memory: DeviceMemory,
    logical_device: Rc<Device>,
}

pub fn new(image: Image, memory: DeviceMemory, logical_device: Rc<Device>) -> DustImage {
    DustImage {
        image,
        memory,
        logical_device,
    }
}

impl Drop for DustImage {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.destroy_image(self.image, None);
            self.logical_device.free_memory(self.memory, None);
        }
    }
}
