use std::rc::Rc;
use std::sync::Arc;

use ash::{
    vk::{
        ComponentMapping, ComponentSwizzle, DeviceMemory, Format, Image, ImageAspectFlags,
        ImageSubresourceRange, ImageView, ImageViewCreateFlags, ImageViewCreateInfo, ImageViewType,
    },
    Device,
};

pub struct DustImage {
    pub image: Image,
    pub view: ImageView,
    pub format: Format,
    memory: DeviceMemory,
    logical_device: Arc<Device>,
}

pub fn new(
    image: Image,
    format: Format,
    memory: DeviceMemory,
    logical_device: Arc<Device>,
) -> DustImage {
    let view = match unsafe {
        logical_device.create_image_view(
            &ImageViewCreateInfo::default()
                .flags(ImageViewCreateFlags::empty())
                .view_type(ImageViewType::TYPE_2D)
                .components(
                    ComponentMapping::default()
                        .r(ComponentSwizzle::IDENTITY)
                        .g(ComponentSwizzle::IDENTITY)
                        .b(ComponentSwizzle::IDENTITY)
                        .a(ComponentSwizzle::IDENTITY),
                )
                .subresource_range(
                    ImageSubresourceRange::default()
                        .aspect_mask(ImageAspectFlags::COLOR)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .base_mip_level(0),
                )
                .image(image)
                .format(format),
            None,
        )
    } {
        Ok(view) => view,
        Err(msg) => {
            panic!("Unable to construct view for image: {:?}", msg);
        }
    };
    DustImage {
        image,
        format,
        view,
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
