use std::usize;

use crate::dust_errors::DustError;
use ash::vk::{
    AccessFlags, Buffer, BufferCopy, BufferCreateFlags, BufferCreateInfo, BufferImageCopy,
    BufferMemoryBarrier, BufferUsageFlags, CommandBuffer, CommandBufferAllocateInfo,
    CommandBufferBeginInfo, CommandBufferLevel, CommandBufferUsageFlags, DependencyFlags,
    DeviceMemory, FenceCreateFlags, FenceCreateInfo, Image, ImageAspectFlags, ImageCreateInfo,
    ImageLayout, ImageMemoryBarrier, ImageSubresourceRange, MemoryAllocateInfo, MemoryMapFlags,
    MemoryPropertyFlags, PhysicalDeviceMemoryProperties, PipelineStageFlags, SharingMode,
    SubmitInfo, QUEUE_FAMILY_IGNORED,
};
use log::debug;

use crate::setup::instance::VkContext;

pub fn copy_to_image<T>(data: &[T], ctxt: &VkContext, image_props: &ImageCreateInfo) -> Image
where
    T: Sized + Clone + Copy,
{
    let (transfer_buffer, memory_handle) = make_buffer_and_copy(data, ctxt);

    let image_target = match unsafe { ctxt.logical_device.create_image(image_props, None) } {
        Ok(image) => image,
        Err(msg) => {
            panic!("Failed to create image: {:?}", image_props);
        }
    };

    let device_memory =
        back_image_with_memory(ctxt, &image_target, &MemoryPropertyFlags::DEVICE_LOCAL);

    let image_subresource = ash::vk::ImageSubresourceLayers::default()
        .mip_level(0)
        .aspect_mask(ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1);

    let buffer_image_copy = BufferImageCopy::default()
        .image_offset(ash::vk::Offset3D { x: 0, y: 0, z: 0 })
        .image_extent(image_props.extent)
        .buffer_offset(0)
        .buffer_row_length(image_props.extent.width)
        .image_subresource(image_subresource)
        .buffer_image_height(image_props.extent.height);

    let regions = vec![buffer_image_copy];

    // Image copy steps
    // 1. Transition image to transfer-optimal
    let transfer_subresource_range = ImageSubresourceRange::default()
        .level_count(1)
        .base_mip_level(0)
        .layer_count(1)
        .base_array_layer(0)
        .aspect_mask(ImageAspectFlags::COLOR);

    let transfer_image_barrier = ImageMemoryBarrier::default()
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .old_layout(ImageLayout::UNDEFINED)
        .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_access_mask(AccessFlags::NONE)
        .dst_access_mask(AccessFlags::TRANSFER_READ | AccessFlags::TRANSFER_WRITE)
        .image(image_target)
        .subresource_range(transfer_subresource_range);
    let transfer_barriers = vec![transfer_image_barrier];

    // 2. Copy image via buffer_iamge_copy
    // 3. Transition image from transfer-optimal BACK to initial state.
    let transfer_back_image_barrier = ImageMemoryBarrier::default()
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .old_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(image_props.initial_layout)
        .src_access_mask(AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(AccessFlags::TRANSFER_READ)
        .image(image_target)
        .subresource_range(transfer_subresource_range);
    let transfer_back_barriers = vec![transfer_back_image_barrier];

    let cmd_buffer = match unsafe {
        ctxt.logical_device.allocate_command_buffers(
            &CommandBufferAllocateInfo::default()
                .command_pool(*ctxt.transfer_queue_command_pools.first().unwrap())
                .command_buffer_count(1)
                .level(CommandBufferLevel::PRIMARY),
        )
    } {
        Ok(mut buffers) => buffers.pop().unwrap(),
        Err(msg) => {
            panic!("Unable to allocate command buffers: {:?}", msg);
        }
    };

    unsafe {
        match ctxt.logical_device.begin_command_buffer(
            cmd_buffer,
            &CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        ) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Unable to begin command buffer: {:?}", msg);
            }
        };
        ctxt.logical_device.cmd_pipeline_barrier(
            cmd_buffer,
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            &[],
            &[],
            &transfer_barriers,
        );
        ctxt.logical_device.cmd_copy_buffer_to_image(
            cmd_buffer,
            transfer_buffer,
            image_target,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions,
        );
        ctxt.logical_device.cmd_pipeline_barrier(
            cmd_buffer,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            DependencyFlags::empty(),
            &[],
            &[],
            &transfer_back_barriers,
        );
        match ctxt.logical_device.end_command_buffer(cmd_buffer) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Unable to end the image copy command buffer: {:?}", msg);
            }
        };
    }

    let buffers = [cmd_buffer];
    run_commands_blocking(ctxt, &buffers);

    // cleanup
    unsafe {
        ctxt.logical_device.free_memory(memory_handle, None);
        ctxt.logical_device.destroy_buffer(transfer_buffer, None);
    }

    image_target
}

pub fn copy_to_buffer<T>(data: &[T], ctxt: &VkContext, usage: BufferUsageFlags) -> Buffer
where
    T: Sized + Copy + Clone,
{
    let size_in_bytes = std::mem::size_of_val(data) as u64;

    let (transfer_buffer, mem_handle) = make_buffer_and_copy(data, ctxt);

    let perm_buffer = make_buffer(ctxt, size_in_bytes, usage | BufferUsageFlags::TRANSFER_DST);
    let _perm_handle =
        back_buffer_with_memory(ctxt, &perm_buffer, &MemoryPropertyFlags::DEVICE_LOCAL);

    let copy_region: [BufferCopy; 1] = [BufferCopy::default()
        .size(size_in_bytes)
        .src_offset(0)
        .dst_offset(0)];

    let cmd_buffer_alloc_info = CommandBufferAllocateInfo::default()
        .level(CommandBufferLevel::PRIMARY)
        .command_pool(*ctxt.transfer_queue_command_pools.first().unwrap())
        .command_buffer_count(1);

    let cmd_buffer = match unsafe {
        ctxt.logical_device
            .allocate_command_buffers(&cmd_buffer_alloc_info)
    } {
        Ok(buffer) => buffer,
        Err(msg) => {
            panic!("Failed to allocate buffer from transfer pool: {:?}", msg);
        }
    };

    let begin_info =
        CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    let buffer_write_barrier = BufferMemoryBarrier::default()
        .src_access_mask(AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(AccessFlags::MEMORY_WRITE | AccessFlags::MEMORY_READ)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .size(size_in_bytes)
        .buffer(perm_buffer)
        .offset(0);
    let fence = match unsafe {
        ctxt.logical_device.create_fence(
            &FenceCreateInfo::default().flags(FenceCreateFlags::empty()),
            None,
        )
    } {
        Ok(fence) => fence,
        Err(msg) => {
            panic!("Fence creation failed: {:?}", msg);
        }
    };

    unsafe {
        match ctxt
            .logical_device
            .begin_command_buffer(*cmd_buffer.first().unwrap(), &begin_info)
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed to begin buffer: {:?}", msg);
            }
        }
        ctxt.logical_device.cmd_copy_buffer(
            *cmd_buffer.first().unwrap(),
            transfer_buffer,
            perm_buffer,
            &copy_region,
        );
        ctxt.logical_device.cmd_pipeline_barrier(
            *cmd_buffer.first().unwrap(),
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::ALL_COMMANDS,
            DependencyFlags::empty(),
            &[],
            &[buffer_write_barrier],
            &[],
        );
        match ctxt
            .logical_device
            .end_command_buffer(*cmd_buffer.first().unwrap())
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Ending command buffer for transfer failed: {:?}", msg);
            }
        }
    }

    let commands_to_run = &cmd_buffer[0..1];

    run_commands_blocking(ctxt, commands_to_run);

    unsafe {
        ctxt.logical_device.destroy_buffer(transfer_buffer, None);
        ctxt.logical_device.free_memory(mem_handle, None);
        // ctxt.logical_device.destroy_fence(fence, None);
    }
    perm_buffer
}

fn run_commands_blocking(ctxt: &VkContext, buffers: &[CommandBuffer]) {
    let submit_info = SubmitInfo::default()
        .command_buffers(buffers)
        .wait_semaphores(&[])
        .wait_dst_stage_mask(&[]);
    let submits = vec![submit_info];

    let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::empty());

    let fence = unsafe {
        match ctxt.logical_device.create_fence(&fence_create_info, None) {
            Ok(fence) => fence,
            Err(msg) => {
                panic!("Failed to create fence: {:?}", msg);
            }
        }
    };

    unsafe {
        match ctxt
            .logical_device
            .queue_submit(ctxt.transfer_queue, &submits, fence)
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Queue submission failed: {:?}", msg);
            }
        }
    };

    unsafe {
        match ctxt.logical_device.wait_for_fences(&[fence], true, 1000000) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Waiting for fence at end of transfer errored: {:?}", msg);
            }
        }
        ctxt.logical_device.destroy_fence(fence, None);
    };
}

fn make_buffer_and_copy<T>(data: &[T], ctxt: &VkContext) -> (Buffer, DeviceMemory)
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
        ctxt.logical_device.unmap_memory(mem_handle);
    }

    (transfer_buffer, mem_handle)
}

fn back_image_with_memory(
    ctxt: &VkContext,
    image: &Image,
    desired_properties: &MemoryPropertyFlags,
) -> DeviceMemory {
    let image_memory_requirements =
        unsafe { ctxt.logical_device.get_image_memory_requirements(*image) };

    let image_mem_type_index = match match_memory_type(
        &ctxt.physical_memory_properties,
        image_memory_requirements.memory_type_bits,
        desired_properties,
    ) {
        Ok(props) => props,
        Err(msg) => {
            panic!(
                "Unable to find a memory type matching required properties: {:?}",
                msg
            );
        }
    };

    let mem_alloc_info = MemoryAllocateInfo::default()
        .allocation_size(image_memory_requirements.size)
        .memory_type_index(image_mem_type_index);

    let mem_handle = match unsafe { ctxt.logical_device.allocate_memory(&mem_alloc_info, None) } {
        Ok(handle) => handle,
        Err(msg) => {
            panic!("Unable to allocate memory for image: {:?}", msg);
        }
    };

    match unsafe { ctxt.logical_device.bind_image_memory(*image, mem_handle, 0) } {
        Ok(_) => mem_handle,
        Err(msg) => {
            panic!("Unable to bind device memory to image: {:?}", msg);
        }
    }
}

fn back_buffer_with_memory(
    ctxt: &VkContext,
    buffer: &Buffer,
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
        .allocation_size(transfer_memory_requirements.size)
        .memory_type_index(transfer_mem_type_index);

    let mem_handle = match unsafe { ctxt.logical_device.allocate_memory(&mem_alloc_info, None) } {
        Ok(handle) => handle,
        Err(msg) => {
            panic!(
                "Unable to allocate buffer sized {}: {:?}",
                transfer_memory_requirements.size, msg
            );
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

    match unsafe {
        ctxt.logical_device
            .create_buffer(&transfer_buffer_info, None)
    } {
        Ok(buffer) => buffer,
        Err(msg) => {
            panic!("Transfer buffer creation failed: {:?}", msg);
        }
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
