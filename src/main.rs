use ash::vk::{
    AccessFlags, BufferCreateInfo, BufferImageCopy, BufferUsageFlags, CommandBufferAllocateInfo, CommandBufferBeginInfo, CommandBufferLevel, CommandBufferResetFlags, CommandBufferUsageFlags, DependencyFlags, Extent3D, Fence, FenceCreateFlags, FenceCreateInfo, Format, Image, ImageAspectFlags, ImageCopy, ImageCreateFlags, ImageCreateInfo, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers, ImageSubresourceRange, ImageTiling, ImageType, ImageUsageFlags, MemoryAllocateInfo, MemoryBarrier, MemoryMapFlags, MemoryPropertyFlags, Offset3D, PipelineStageFlags, PresentInfoKHR, SampleCountFlags, Semaphore, SemaphoreCreateFlags, SemaphoreCreateInfo, SharingMode, SubmitInfo, QUEUE_FAMILY_IGNORED
};
use graphics::{image::DustImage, swapchain};
use graphics::{pools, transfer};
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


    let (gradient, semaphore) = load_gradient(&vk_context);
    graphics::render::composite_test(&vk_context, &gradient.view, gradient.format, semaphore);
    // display_image(&vk_context);
    // display_gradient(&vk_context);
    sleep(Duration::from_secs(3));

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

fn display_gradient(ctxt: &VkContext) {
    debug!("Creating gradient source image...");
    let gradient_src = load_gradient(ctxt);

    let make_semaphore = SemaphoreCreateInfo::default();
    let signal_previous_draw_complete = match unsafe {ctxt.logical_device.create_semaphore(&make_semaphore, None)} {
        Ok(semaphore) => semaphore, 
        Err(msg) => {panic!("Failed to create new semaphore: {:?}", msg); }
    };

    let (swapchain_image_index, swapchain_image, _suboptimal) = 
        graphics::swapchain::next_swapchain_image(signal_previous_draw_complete, Fence::null());
    // let swapchain_image = ctxt.swapchain_images.get(swapchain_image_index).unwrap();

    let buffer = crate::graphics::pools::reserve_graphics_buffer(ctxt);
    // let buffer = match unsafe {
    //     ctxt.logical_device.allocate_command_buffers(
    //         &CommandBufferAllocateInfo::default()
    //             .command_pool(*ctxt.graphics_queue_command_pools.first().unwrap())
    //             .command_buffer_count(1)
    //             .level(CommandBufferLevel::PRIMARY),
    //     )
    // } {
    //     Ok(buffer) => buffer,
    //     Err(msg) => {
    //         panic!("Unable to allocate command buffer: {:?}", msg);
    //     }
    // };

    match unsafe {
        ctxt.logical_device.begin_command_buffer(buffer, 
            // *buffer.first().unwrap(),
            &CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT),
        )
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Unable to begin command buffer: {:?}", msg);
        }
    };

    let src_subresource = ImageSubresourceLayers::default()
        .aspect_mask(ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1)
        .mip_level(0);

    let image_to_image_info = ImageCopy::default()
        .src_offset(Offset3D::default().x(0).y(0).z(0))
        .dst_offset(Offset3D::default().x(0).y(0).z(0))
        .src_subresource(src_subresource)
        .dst_subresource(src_subresource)
        .extent(Extent3D::default().width(1920).height(1080).depth(1))
    ;

    let transfer_barrier = ImageMemoryBarrier::default()
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .old_layout(ImageLayout::UNDEFINED)
        .new_layout(ImageLayout::TRANSFER_DST_OPTIMAL)
        .src_access_mask(AccessFlags::NONE)
        .dst_access_mask(AccessFlags::TRANSFER_WRITE)
        .subresource_range(ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .level_count(1)
            .base_array_layer(0)
            .base_mip_level(0)
            .layer_count(1)
        );
        // .image(*swapchain_image);

    let presentation_barrier = ImageMemoryBarrier::default()
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .old_layout(ImageLayout::UNDEFINED)
        .new_layout(ImageLayout::PRESENT_SRC_KHR)
        .src_access_mask(AccessFlags::TRANSFER_WRITE)
        .dst_access_mask(AccessFlags::COLOR_ATTACHMENT_READ)
        .subresource_range(ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .level_count(1)
            .base_array_layer(0)
            .base_mip_level(0)
            .layer_count(1)
        );
        // .image(*swapchain_image);

    let mem_barriers = vec![];
    let buffer_barriers = vec![];
    let copy_to_barriers = vec![transfer_barrier];
    let presentation_barriers = vec![presentation_barrier];

    unsafe {
        ctxt.logical_device.cmd_pipeline_barrier(
            // *buffer.first().unwrap(), 
            buffer, 
            PipelineStageFlags::TOP_OF_PIPE, 
            PipelineStageFlags::TRANSFER, 
            DependencyFlags::empty(), 
            &mem_barriers, 
            &buffer_barriers, 
            &copy_to_barriers
        );
        // ctxt.logical_device.cmd_copy_image(*buffer.first().unwrap(), 
        //     gradient_src.image, 
        //     ImageLayout::TRANSFER_SRC_OPTIMAL, 
        //     *swapchain_image, 
        //     ImageLayout::TRANSFER_DST_OPTIMAL, 
        //     &[image_to_image_info]);

        ctxt.logical_device.cmd_pipeline_barrier(
            // *buffer.first().unwrap(), 
            buffer, 
            PipelineStageFlags::TRANSFER, 
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, 
            DependencyFlags::empty(), 
            &mem_barriers, 
            &buffer_barriers, 
            &presentation_barriers);

        // ctxt.logical_device.end_command_buffer(*buffer.first().unwrap());
        ctxt.logical_device.end_command_buffer(buffer);
    }

    let command_submission_fence = 
        match unsafe {ctxt.logical_device.create_fence(&FenceCreateInfo::default().flags(FenceCreateFlags::empty()), None)} {
        Ok(fence) => fence, 
        Err(msg) => { panic!("Unable to create fence to signal end of draw commands: {:?}", msg); }
    };

    let command_submission_semaphore = 
        match unsafe {ctxt.logical_device.create_semaphore(&SemaphoreCreateInfo::default().flags(SemaphoreCreateFlags::empty()), None)} {
            Ok(sem) => sem, 
            Err(msg) => { panic! ("Unable to create semaphore to signal end of draw commands: {:?}", msg); }
    };

    let semaphores = [command_submission_semaphore];
    let submission_blockers = [signal_previous_draw_complete];
    // let submission_buffers = [*buffer.first().unwrap()];
    let submission_buffers = [buffer];
    let submit_info = SubmitInfo::default()
        .wait_semaphores(&submission_blockers)
        .wait_dst_stage_mask(&[PipelineStageFlags::TRANSFER])
        .command_buffers(&submission_buffers)
        .signal_semaphores(&semaphores)
    ;


    unsafe {
        match ctxt.logical_device.queue_submit(ctxt.graphics_queue, &[submit_info], command_submission_fence) 
        {
            Ok(_) => {}, 
            Err(msg) => {
                panic!("Unable to submit gradient copy to screen: {:?}", msg);
            }
        }
        match ctxt.logical_device.wait_for_fences(&[command_submission_fence], true, 100000) {
            Ok(_) => {}, 
            Err(msg) => {
                panic!("Failed to wait on fence: {:?}", msg);
            }
        }
    }

    swapchain::present_swapchain_image(swapchain_image_index as u32, &ctxt.graphics_queue, &semaphores);


    // ** DESTRUCTION ** //
    unsafe {
        ctxt.logical_device.destroy_semaphore(command_submission_semaphore, None);
        ctxt.logical_device.destroy_fence(command_submission_fence, None);
        ctxt.logical_device.destroy_semaphore(signal_previous_draw_complete, None);
    }
}

fn load_black(ctxt: &VkContext) -> (DustImage, Semaphore) {
    let image_width = ctxt.surface_capabilities.current_extent.width;
    let image_height = ctxt.surface_capabilities.current_extent.height;

    let pixel_stride = 4;
    let buffer_size_in_bytes = image_width * image_height * pixel_stride;
    
    let mut host_buffer: Vec<u8> = vec![0; buffer_size_in_bytes as usize];

    let target_image = &ImageCreateInfo::default()
        .initial_layout(ImageLayout::UNDEFINED)
        .sharing_mode(SharingMode::EXCLUSIVE)
        .image_type(ImageType::TYPE_2D)
        .array_layers(1)
        .format(Format::R8G8B8A8_SRGB)
        .extent(
            Extent3D::default()
                .height(image_height)
                .width(image_width)
                .depth(1),
        )
        .mip_levels(1)
        .samples(SampleCountFlags::TYPE_1)
        .flags(ImageCreateFlags::empty())
        .usage(
            ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::INPUT_ATTACHMENT,
        )
        .tiling(ImageTiling::OPTIMAL);

    transfer::copy_to_image(
        &host_buffer,
        ctxt,
        target_image,
        ImageLayout::TRANSFER_SRC_OPTIMAL,
        pools::get_graphics_queue_family()
    )
}

fn load_gradient(ctxt: &VkContext) -> (DustImage, Semaphore) {
    let image_width = ctxt.surface_capabilities.current_extent.width;
    let image_height = ctxt.surface_capabilities.current_extent.height;

    let horizontal_pixel_incr = 255.0 / image_width as f32;
    let vertical_pixel_incr = 255.0 / image_height as f32;

    debug!("Image width: {}", image_width);
    debug!("Image height: {}", image_height);

    // Just assume RGBA here - 4 bytes per pixel.
    let pixel_stride = 4;
    let buffer_size_in_bytes = image_width * image_height * pixel_stride;

    let mut host_buffer: Vec<u8> = vec![0; buffer_size_in_bytes as usize];

    let mut red_pixel_value: u8;
    let mut green_pixel_value: u8;
    let mut blue_pixel_value: u8;

    for row_index in 0..image_height {
        green_pixel_value = (vertical_pixel_incr * row_index as f32).floor() as u8;
        for col_index in 0..image_width {
            let red_index =
                (row_index * (image_width * pixel_stride) + (col_index * pixel_stride)) as usize;
            let green_index = red_index + 1;
            let blue_index = red_index + 2;
            let alpha_index = red_index + 3;

            red_pixel_value = (horizontal_pixel_incr * col_index as f32).floor() as u8;
            blue_pixel_value =
                ((row_index * col_index) as f32 / (image_width * image_height) as f32 * 255.0)
                    .floor() as u8;

            host_buffer[red_index] = red_pixel_value;
            host_buffer[green_index] = green_pixel_value;
            host_buffer[blue_index] = blue_pixel_value;
            host_buffer[alpha_index] = 255;
        }
    }

    let target_image = &ImageCreateInfo::default()
        .initial_layout(ImageLayout::UNDEFINED)
        .sharing_mode(SharingMode::EXCLUSIVE)
        .image_type(ImageType::TYPE_2D)
        .array_layers(1)
        .format(Format::R8G8B8A8_SRGB)
        .extent(
            Extent3D::default()
                .height(image_height)
                .width(image_width)
                .depth(1),
        )
        .mip_levels(1)
        .samples(SampleCountFlags::TYPE_1)
        .flags(ImageCreateFlags::empty())
        .usage(
            ImageUsageFlags::TRANSFER_SRC
                | ImageUsageFlags::TRANSFER_DST
                | ImageUsageFlags::INPUT_ATTACHMENT,
        )
        .tiling(ImageTiling::OPTIMAL);

    transfer::copy_to_image(
        &host_buffer,
        ctxt,
        target_image,
        ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        pools::get_graphics_queue_family()
    )
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

    let (swapchain_index, swapchain_image, suboptimal) = graphics::swapchain::next_swapchain_image(
        swapchain_grab_semaphore,
        swapchain_image_acq_fence,
    );
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

    // let command_buffer = vk_ctxt.buffers.first().unwrap();
    let command_buffer = crate::graphics::pools::reserve_graphics_buffer(&vk_ctxt);

    match unsafe {
        vk_ctxt
            .logical_device
            .reset_command_buffer(command_buffer, CommandBufferResetFlags::empty())
    } {
        Ok(_) => {}
        Err(msg) => {
            panic!("Command buffer reset failed: {:?}", msg);
        }
    };

    // let dst_image = vk_ctxt
    //     .swapchain_images
    //     .get(swapchain_index )
    //     .unwrap();

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
            .begin_command_buffer(command_buffer, &CommandBufferBeginInfo::default())
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
        // .image(*swapchain_image)
        .subresource_range(dst_img_subresource_range);

    let presentation_transition_image_barrier = ImageMemoryBarrier::default()
        .old_layout(ImageLayout::GENERAL)
        .new_layout(ImageLayout::PRESENT_SRC_KHR)
        .src_queue_family_index(QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(QUEUE_FAMILY_IGNORED)
        .src_access_mask(AccessFlags::TRANSFER_WRITE)
        // .image(*swapchain_image)
        .subresource_range(dst_img_subresource_range);

    let memory_barriers = Vec::new();
    let mut copy_transition_image_barriers = Vec::new();
    let mut presentation_transition_image_barriers = Vec::new();
    let buffer_barriers = Vec::new();

    copy_transition_image_barriers.push(copy_xition_image_barrier);
    presentation_transition_image_barriers.push(presentation_transition_image_barrier);

    unsafe {
        vk_ctxt.logical_device.cmd_pipeline_barrier(
            command_buffer,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            DependencyFlags::empty(),
            memory_barriers.as_slice(),
            buffer_barriers.as_slice(),
            copy_transition_image_barriers.as_slice(),
        );
        // vk_ctxt.logical_device.cmd_copy_buffer_to_image(
        //     *command_buffer,
        //     buffer,
        //     *swapchain_image,
        //     ImageLayout::GENERAL,
        //     &[buffer_image_copy; 1],
        // );
        vk_ctxt.logical_device.cmd_pipeline_barrier(
            command_buffer,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            DependencyFlags::empty(),
            memory_barriers.as_slice(),
            buffer_barriers.as_slice(),
            presentation_transition_image_barriers.as_slice(),
        );
        match vk_ctxt.logical_device.end_command_buffer(command_buffer) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed to end command buffer: {:?}", msg);
            }
        }
    }

    let semaphore_array = [swapchain_grab_semaphore; 1];
    let draw_semaphore_array = [draw_complete_semaphore; 1];
    let buffer_array = [command_buffer; 1];

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

    // let swapchain_array = [vk_ctxt.swapchain; 1];
    // let image_index_array = [swapchain_index; 1];
    //
    // let present_info = PresentInfoKHR::default()
    //     .wait_semaphores(&draw_semaphore_array)
    //     .swapchains(&swapchain_array)
    //     .image_indices(&image_index_array);

    match 
        graphics::swapchain::present_swapchain_image(
            swapchain_index as u32,
            &vk_ctxt.graphics_queue,
            &draw_semaphore_array,
        )
        // vk_ctxt
        //         .swapchain_device
        //         .queue_present(vk_ctxt.graphics_queue, &present_info)
    {
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

    sleep(Duration::from_secs(3));

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
