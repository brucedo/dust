use crate::dust_errors::DustError;
use ash::vk::{
    AccessFlags, AccessFlags2, Buffer, BufferCopy, BufferCreateFlags, BufferCreateInfo,
    BufferImageCopy, BufferMemoryBarrier, BufferUsageFlags, CommandBuffer, CommandBufferBeginInfo,
    CommandBufferUsageFlags, DependencyFlags, DependencyInfo, DeviceMemory, FenceCreateFlags,
    FenceCreateInfo, Image, ImageAspectFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier2,
    ImageSubresourceRange, MemoryAllocateInfo, MemoryMapFlags, MemoryPropertyFlags,
    PhysicalDeviceMemoryProperties, PipelineStageFlags, PipelineStageFlags2, Semaphore,
    SharingMode, SubmitInfo, QUEUE_FAMILY_IGNORED,
};
use log::debug;

use crate::setup::instance::VkContext;

use super::{image::DustImage, pools, util};

pub fn copy_to_image<T>(
    data: &[T],
    ctxt: &VkContext,
    image_props: &ImageCreateInfo,
    target_layout: ImageLayout,
    target_queue_family: u32,
) -> (DustImage, Semaphore)
where
    T: Sized + Clone + Copy,
{
    let (transfer_buffer, memory_handle) = make_buffer_and_copy(data, ctxt);

    let image_target = match unsafe { ctxt.logical_device.create_image(image_props, None) } {
        Ok(image) => image,
        Err(msg) => {
            panic!("Failed to create image: {:?}", msg);
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

    let to_transfer_dst_layout = ImageMemoryBarrier2::default()
        // .src_stage_mask(PipelineStageFlags2::TOP_OF_PIPE)
        // .src_access_mask(AccessFlags2::NONE)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        // .dst_queue_family_index(pools::get_transfer_queue_family())
        .dst_stage_mask(PipelineStageFlags2::TRANSFER)
        .dst_access_mask(AccessFlags2::TRANSFER_WRITE)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .old_layout(ImageLayout::UNDEFINED)
        .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
        .image(image_target)
        .subresource_range(transfer_subresource_range);

    let transfer_barriers = vec![to_transfer_dst_layout];

    // 2. Copy image via buffer_iamge_copy
    // 3. Transition image from transfer-optimal BACK to initial state.
    let from_transfer_dst_layout = ImageMemoryBarrier2::default()
        .src_stage_mask(PipelineStageFlags2::TRANSFER)
        .src_access_mask(AccessFlags2::TRANSFER_WRITE)
        .src_queue_family_index(pools::get_transfer_queue_family())
        // .dst_stage_mask(PipelineStageFlags2::FRAGMENT_SHADER)
        // .dst_access_mask(dst_mask)
        .dst_queue_family_index(target_queue_family)
        .old_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
        .new_layout(target_layout)
        .image(image_target)
        .subresource_range(transfer_subresource_range);
    let transfer_back_barriers = vec![from_transfer_dst_layout];

    let cmd_buffer = crate::graphics::pools::reserve_transfer_buffer(ctxt);

    let copy_into_dependency_info = DependencyInfo::default()
        .memory_barriers(&[])
        .image_memory_barriers(&transfer_barriers)
        .buffer_memory_barriers(&[])
        .dependency_flags(DependencyFlags::empty());

    let transfer_to_final_dependency_info = DependencyInfo::default()
        .memory_barriers(&[])
        .image_memory_barriers(&transfer_back_barriers)
        .buffer_memory_barriers(&[])
        .dependency_flags(DependencyFlags::empty());

    let copy_and_transition_complete_semaphore = util::create_binary_semaphore(ctxt);

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

        ctxt.logical_device.cmd_pipeline_barrier2(
            cmd_buffer,
            &copy_into_dependency_info,
            // PipelineStageFlags::TOP_OF_PIPE,
            // PipelineStageFlags::TRANSFER,
            // DependencyFlags::empty(),
            // &[],
            // &[],
            // &transfer_barriers,
        );
        ctxt.logical_device.cmd_copy_buffer_to_image(
            cmd_buffer,
            transfer_buffer,
            image_target,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions,
        );
        ctxt.logical_device.cmd_pipeline_barrier2(
            cmd_buffer,
            &transfer_to_final_dependency_info,
            // PipelineStageFlags::TRANSFER,
            // PipelineStageFlags::TRANSFER,
            // DependencyFlags::empty(),
            // &[],
            // &[],
            // &transfer_back_barriers,
        );
        match ctxt.logical_device.end_command_buffer(cmd_buffer) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Unable to end the image copy command buffer: {:?}", msg);
            }
        };
    }

    let buffers = [cmd_buffer];
    let signal_semaphores = [copy_and_transition_complete_semaphore];
    run_commands_blocking(ctxt, &buffers, &signal_semaphores);

    // cleanup
    unsafe {
        ctxt.logical_device.destroy_buffer(transfer_buffer, None);
        ctxt.logical_device.free_memory(memory_handle, None);
    }

    (
        crate::graphics::image::new(
            image_target,
            image_props.format,
            device_memory,
            ctxt.logical_device.clone(),
        ),
        copy_and_transition_complete_semaphore,
    )
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

    let cmd_buffer = crate::graphics::pools::reserve_transfer_buffer(ctxt);

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

    unsafe {
        match ctxt
            .logical_device
            // .begin_command_buffer(*cmd_buffer.first().unwrap(), &begin_info)
            .begin_command_buffer(cmd_buffer, &begin_info)
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed to begin buffer: {:?}", msg);
            }
        }
        ctxt.logical_device.cmd_copy_buffer(
            // *cmd_buffer.first().unwrap(),
            cmd_buffer,
            transfer_buffer,
            perm_buffer,
            &copy_region,
        );
        ctxt.logical_device.cmd_pipeline_barrier(
            // *cmd_buffer.first().unwrap(),
            cmd_buffer,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::ALL_COMMANDS,
            DependencyFlags::empty(),
            &[],
            &[buffer_write_barrier],
            &[],
        );
        match ctxt
            .logical_device
            // .end_command_buffer(*cmd_buffer.first().unwrap())
            .end_command_buffer(cmd_buffer)
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Ending command buffer for transfer failed: {:?}", msg);
            }
        }
    }

    // let commands_to_run = &cmd_buffer[0..1];

    run_commands_blocking(ctxt, &[cmd_buffer], &[]);

    unsafe {
        ctxt.logical_device.destroy_buffer(transfer_buffer, None);
        ctxt.logical_device.free_memory(mem_handle, None);
        // ctxt.logical_device.destroy_fence(fence, None);
    }
    perm_buffer
}

fn run_commands_blocking(
    ctxt: &VkContext,
    buffers: &[CommandBuffer],
    signal_complete_semaphore: &[Semaphore],
) {
    let submit_info = SubmitInfo::default()
        .command_buffers(buffers)
        // .wait_semaphores(&[])
        .signal_semaphores(signal_complete_semaphore)
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
        match ctxt
            .logical_device
            .wait_for_fences(&[fence], true, 10000000000)
        {
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
    debug!("Size of val: {}", size_in_bytes);
    debug!("Size of data array itself: {}", data.len());

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
    let t_buf = unsafe { std::slice::from_raw_parts_mut(t_ptr, data.len()) };
    debug!("Transfer buffer size: {}", t_buf.len());

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

fn map_access_flags(layout: ImageLayout) -> AccessFlags2 {
    match layout {
        ImageLayout::SHADER_READ_ONLY_OPTIMAL => {
            AccessFlags2::INPUT_ATTACHMENT_READ
                | AccessFlags2::SHADER_READ
                | AccessFlags2::COLOR_ATTACHMENT_READ
                | AccessFlags2::SHADER_SAMPLED_READ
                | AccessFlags2::SHADER_STORAGE_READ
            // | AccessFlags2::SHADER_BINDING_TABLE_READ_KHR
        }
        _ => AccessFlags2::empty(),
    }
}
