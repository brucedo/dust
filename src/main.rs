use ash::vk::{
    AccessFlags, BufferCreateInfo, BufferImageCopy, BufferUsageFlags, CommandBufferBeginInfo,
    CommandBufferResetFlags, DependencyFlags, Extent3D, FenceCreateFlags, FenceCreateInfo,
    ImageAspectFlags, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers,
    ImageSubresourceRange, MemoryAllocateInfo, MemoryBarrier, MemoryMapFlags, MemoryPropertyFlags,
    Offset3D, PipelineStageFlags, PresentInfoKHR, SemaphoreCreateInfo, SharingMode, SubmitInfo,
    QUEUE_FAMILY_IGNORED,
};
use log::debug;

mod dust_errors;
mod graphics;
mod input;
mod setup;

use setup::{instance::VkContext, xcb_window};
use std::{
    thread::{self, sleep},
    time::Duration,
};
use xcb::x::Window;

use crate::{
    input::input::KeyStroke,
    setup::instance::{self},
};

fn main() {
    env_logger::init();

    // window system setup
    debug!("Starting X-Windows initialization...");
    let (conn, screen_num) = xcb_window::connect();
    xcb_window::interrogate_keymaps(&conn);
    xcb_window::extension_data(&conn);
    let window: Window = xcb_window::create_window(&conn, screen_num);
    let (upper_left, window_size) = xcb_window::interrogate_randr(&conn, window);
    xcb_window::resize_window(&conn, window, upper_left, window_size);

    let _xcb_ptr = conn.get_raw_conn();
    let (sender, _receiver) = std::sync::mpsc::sync_channel::<KeyStroke>(16);
    thread::spawn(move || xcb_window::event_loop(conn, sender));

    let vk_context = instance::default(_xcb_ptr, &window);
    show_physical_memory_stats(&vk_context);
    display_image(&vk_context);

    debug!("Vulkan instance destroyed...");
}

fn show_physical_memory_stats(vk_ctxt: &VkContext) {
    let temp = vk_ctxt.physical_memory_properties;

    for index in 0..temp.memory_type_count as usize {
        debug!(
            "mem_type {}: {:?}",
            temp.memory_types[index].heap_index, temp.memory_types[index].property_flags
        );
    }

    for index in 0..temp.memory_heap_count as usize {
        if !temp.memory_heaps[index].flags.is_empty() {
            debug!(
                "Memory heap {:?}: {}",
                temp.memory_heaps[index].flags, temp.memory_heaps[index].size
            );
        } else {
            debug!(
                "Memory heap flags empty, size {}",
                temp.memory_heaps[index].size
            );
        }
    }
}

fn display_image(vk_ctxt: &VkContext) {
    let image_width = vk_ctxt.surface_capabilities.current_extent.width;
    let image_height = vk_ctxt.surface_capabilities.current_extent.height;

    let buffer_info = BufferCreateInfo::default()
        .size((image_width * image_height * 4) as u64)
        .usage(BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::STORAGE_BUFFER)
        .sharing_mode(SharingMode::EXCLUSIVE);

    let buffer = match unsafe { vk_ctxt.logical_device.create_buffer(&buffer_info, None) } {
        Ok(buffer) => buffer,
        Err(msg) => {
            panic!("Buffer creation failed: {:?}", msg);
        }
    };

    let mem_req = unsafe {
        vk_ctxt
            .logical_device
            .get_buffer_memory_requirements(buffer)
    };

    let mem_type_index = match vk_ctxt.match_memory_type(
        mem_req.memory_type_bits,
        &(MemoryPropertyFlags::HOST_VISIBLE
            | MemoryPropertyFlags::HOST_COHERENT
            | MemoryPropertyFlags::DEVICE_LOCAL),
    ) {
        Ok(mem_type_index) => mem_type_index,
        Err(msg) => {
            panic!(
                "Could not find memory type matching requirements {:?}: {:?}",
                mem_req.memory_type_bits, msg
            );
        }
    };

    let mem_alloc_info = MemoryAllocateInfo::default()
        .allocation_size(buffer_info.size)
        .memory_type_index(mem_type_index);

    let mem_handle = match unsafe {
        vk_ctxt
            .logical_device
            .allocate_memory(&mem_alloc_info, None)
    } {
        Ok(handle) => handle,
        Err(msg) => {
            panic!(
                "Unable to allocate buffer sized {}: {:?}",
                buffer_info.size, msg
            );
        }
    };

    match unsafe {
        vk_ctxt
            .logical_device
            .bind_buffer_memory(buffer, mem_handle, 0)
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!(
                "Failed to bind the buffer to the allocated memory: {:?}",
                msg
            );
        }
    }

    // now make the dumping array available to me
    let void_ptr = match unsafe {
        vk_ctxt
            .logical_device
            .map_memory(mem_handle, 0, buffer_info.size, MemoryMapFlags::empty())
    } {
        Ok(ptr) => ptr,
        Err(msg) => {
            panic!("Failed to map the buffer backed memory to host: {:?}", msg);
        }
    };

    let u8_ptr = void_ptr as *mut u8;
    let u8_buf = unsafe { std::slice::from_raw_parts_mut(u8_ptr, buffer_info.size as usize) };

    for index in 0..buffer_info.size as usize {
        u8_buf[index] = 255;
    }

    // slowly edging on towards drawing...
    let swapchain_grab_semaphore = match unsafe {
        vk_ctxt
            .logical_device
            .create_semaphore(&ash::vk::SemaphoreCreateInfo::default(), None)
    } {
        Ok(semaphore) => semaphore,
        Err(msg) => {
            panic!("Failed to create semaphore: {:?}", msg);
        }
    };

    let draw_complete_semaphore = match unsafe {
        vk_ctxt
            .logical_device
            .create_semaphore(&SemaphoreCreateInfo::default(), None)
    } {
        Ok(semaphore) => semaphore,
        Err(msg) => {
            panic!("Failed to create draw_complete semaphore: {:?}", msg);
        }
    };

    let fence_create_info = FenceCreateInfo::default().flags(FenceCreateFlags::empty());

    let frame_drawn_fence = match unsafe {
        vk_ctxt
            .logical_device
            .create_fence(&fence_create_info, None)
    } {
        Ok(fence) => fence,
        Err(msg) => {
            panic!("Could not create fence: {:?}", msg);
        }
    };

    let swapchain_image_acq_fence = match unsafe {
        vk_ctxt
            .logical_device
            .create_fence(&fence_create_info, None)
    } {
        Ok(fence) => fence,
        Err(msg) => {
            panic!("Could not create swapchain-grab fence: {:?}", msg);
        }
    };

    let (swapchain_index, suboptimal) = match unsafe {
        vk_ctxt.swapchain_device.acquire_next_image(
            vk_ctxt.swapchain,
            100,
            swapchain_grab_semaphore,
            swapchain_image_acq_fence,
        )
    } {
        Ok(index) => index,
        Err(msg) => {
            panic!("Unable to acquire next swapchian image: {:?}", msg);
        }
    };

    debug!("next swapchain image: {}", swapchain_index);
    debug!("next image is suboptimal: {}", suboptimal);

    match unsafe {
        vk_ctxt
            .logical_device
            .wait_for_fences(&[swapchain_image_acq_fence], true, 100)
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Waiting for swapchain fence failed: {:?}", msg);
        }
    };

    debug!("Passed image grab wait.");

    let command_buffer = vk_ctxt.buffers.first().unwrap();

    match unsafe {
        vk_ctxt
            .logical_device
            .reset_command_buffer(*command_buffer, CommandBufferResetFlags::empty())
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Command buffer reset failed: {:?}", msg);
        }
    };

    let dst_image = vk_ctxt
        .swapchain_images
        .get(swapchain_index as usize)
        .unwrap();

    let dst_img_subresource_range = ImageSubresourceRange::default()
        .aspect_mask(ImageAspectFlags::COLOR)
        .layer_count(1)
        .level_count(1);

    let buffer_image_copy = BufferImageCopy::default()
        .buffer_offset(0)
        .buffer_row_length(1920)
        .buffer_image_height(1080)
        .image_offset(Offset3D::default().x(0).y(0).z(0))
        .image_extent(Extent3D::default().depth(1).height(1080).width(1920))
        .image_subresource(
            ImageSubresourceLayers::default()
                .mip_level(0)
                .layer_count(1)
                .base_array_layer(0)
                .aspect_mask(ImageAspectFlags::COLOR),
        );

    match unsafe {
        vk_ctxt
            .logical_device
            .begin_command_buffer(*command_buffer, &CommandBufferBeginInfo::default())
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Command buffer recording failed: {:?} ", msg);
        }
    };

    let copy_xition_image_barrier = ImageMemoryBarrier::default()
        .old_layout(ImageLayout::UNDEFINED)
        .new_layout(ImageLayout::GENERAL)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .src_access_mask(AccessFlags::NONE)
        .dst_access_mask(AccessFlags::TRANSFER_WRITE)
        .image(*dst_image)
        .subresource_range(dst_img_subresource_range);

    let presentation_transition_image_barrier = ImageMemoryBarrier::default()
        .old_layout(ImageLayout::GENERAL)
        .new_layout(ImageLayout::PRESENT_SRC_KHR)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .src_access_mask(AccessFlags::TRANSFER_WRITE)
        .image(*dst_image)
        .subresource_range(dst_img_subresource_range);

    let memory_barriers = Vec::new();
    let mut copy_transition_image_barriers = Vec::new();
    let mut presentation_transition_image_barriers = Vec::new();
    let buffer_barriers = Vec::new();

    copy_transition_image_barriers.push(copy_xition_image_barrier);
    presentation_transition_image_barriers.push(presentation_transition_image_barrier);

    unsafe {
        vk_ctxt.logical_device.cmd_pipeline_barrier(
            *command_buffer,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            memory_barriers.as_slice(),
            buffer_barriers.as_slice(),
            copy_transition_image_barriers.as_slice(),
        );
        vk_ctxt.logical_device.cmd_copy_buffer_to_image(
            *command_buffer,
            buffer,
            *dst_image,
            ImageLayout::GENERAL,
            &[buffer_image_copy; 1],
        );
        vk_ctxt.logical_device.cmd_pipeline_barrier(
            *command_buffer,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            DependencyFlags::empty(),
            memory_barriers.as_slice(),
            buffer_barriers.as_slice(),
            presentation_transition_image_barriers.as_slice(),
        );
        match vk_ctxt.logical_device.end_command_buffer(*command_buffer) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed to end command buffer: {:?}", msg);
            }
        }
    }

    let semaphore_array = [swapchain_grab_semaphore; 1];
    let draw_semaphore_array = [draw_complete_semaphore; 1];
    let buffer_array = [*command_buffer; 1];

    let queue_submit_info = SubmitInfo::default()
        .wait_semaphores(&semaphore_array)
        .wait_dst_stage_mask(&[PipelineStageFlags::TOP_OF_PIPE; 1])
        .signal_semaphores(&draw_semaphore_array)
        .command_buffers(&buffer_array);

    match unsafe {
        vk_ctxt.logical_device.queue_submit(
            vk_ctxt.graphics_queue,
            &[queue_submit_info; 1],
            frame_drawn_fence,
        )
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Queue submission failed: {:?}", msg);
        }
    };

    let swapchain_array = [vk_ctxt.swapchain; 1];
    let image_index_array = [swapchain_index; 1];

    let present_info = PresentInfoKHR::default()
        .wait_semaphores(&draw_semaphore_array)
        .swapchains(&swapchain_array)
        .image_indices(&image_index_array);

    match unsafe {
        vk_ctxt
            .swapchain_device
            .queue_present(vk_ctxt.graphics_queue, &present_info)
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!(
                "Attempting to present the swapchain image failed: {:?}",
                msg
            );
        }
    };

    debug!("Waiting for frame_drawn_fence to trigger...");
    match unsafe {
        vk_ctxt
            .logical_device
            .wait_for_fences(&[frame_drawn_fence], true, 9000)
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Waiting for frame drawn fence failed: {:?}", msg);
        }
    }
    debug!("Frame_drawn_fence has triggered??");

    sleep(Duration::from_secs(10));

    // Destruction section
    unsafe {
        vk_ctxt
            .logical_device
            .destroy_semaphore(swapchain_grab_semaphore, None);
        vk_ctxt
            .logical_device
            .destroy_semaphore(draw_complete_semaphore, None);
        vk_ctxt.logical_device.destroy_buffer(buffer, None);
        vk_ctxt.logical_device.free_memory(mem_handle, None);
        vk_ctxt
            .logical_device
            .destroy_fence(frame_drawn_fence, None);
        vk_ctxt
            .logical_device
            .destroy_fence(swapchain_image_acq_fence, None);
    }
}
