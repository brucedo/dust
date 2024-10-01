use ash::vk::{
    AccessFlags, AttachmentDescription, AttachmentDescriptionFlags, AttachmentLoadOp,
    AttachmentReference, AttachmentStoreOp, ClearColorValue, ClearValue, ColorComponentFlags,
    CommandBufferBeginInfo, CommandBufferUsageFlags, CullModeFlags, Extent2D, Fence, Format,
    Framebuffer, FramebufferCreateInfo, FrontFace, GraphicsPipelineCreateInfo, ImageLayout,
    ImageView, Offset2D, Pipeline, PipelineBindPoint, PipelineCache,
    PipelineColorBlendAttachmentState, PipelineColorBlendStateCreateInfo, PipelineCreateFlags,
    PipelineInputAssemblyStateCreateFlags, PipelineInputAssemblyStateCreateInfo, PipelineLayout,
    PipelineLayoutCreateFlags, PipelineLayoutCreateInfo, PipelineMultisampleStateCreateFlags,
    PipelineMultisampleStateCreateInfo, PipelineRasterizationStateCreateFlags,
    PipelineRasterizationStateCreateInfo, PipelineShaderStageCreateFlags,
    PipelineShaderStageCreateInfo, PipelineStageFlags, PipelineVertexInputStateCreateFlags,
    PipelineVertexInputStateCreateInfo, PipelineViewportStateCreateFlags,
    PipelineViewportStateCreateInfo, PolygonMode, PrimitiveTopology, Rect2D, RenderPass,
    RenderPassBeginInfo, RenderPassCreateFlags, RenderPassCreateInfo, SampleCountFlags, Semaphore,
    ShaderStageFlags, SubmitInfo, SubpassContents, SubpassDependency, SubpassDescription,
    SubpassDescriptionFlags, Viewport, SUBPASS_EXTERNAL,
};

use log::debug;

use crate::{graphics::shaders, setup::instance::VkContext};

use super::{swapchain, util};

pub fn composite_hud(
    ctxt: &VkContext,
    hud_image: &ImageView,
    view_fmt: Format,
    image_ready: Semaphore,
) {
    // Steps to win:
    // 1.  Get swapchain image.
    //     a.  Create a swapchain-drawing-on-this-image-complete Semaphore
    //     b.  Issue request for the Swapchain image.
    // 2.  Build DescriptorSetLayout
    //     a.  Set the type: INPUT_ATTACHMENT
    //     b.  This is for a single binding, and a single descriptor within the binding.
    //     c.  Ensure the binding number is 0.
    // 3.  Get the DescriptorSet.
    //     a.  There's not much more to this, other than making sure the DescriptorSet and the
    //         DescriptorSetLayout are used in the correct places.
    // 4.  Build RenderPass
    //     a.  Construct the AttachmentReferences
    //     b.  Construct the AttachmentDescriptions
    //     c.  Construct the render subpass
    // 5.  Build Framebuffer.
    //     a.  Set the attachments in order swapchain, hud
    //     b.  Set the width and height of the framebuffer
    //     c.  Set the render pass
    // 6.  Build PipelineLayout.
    //     a.  Associate the DescriptorSetLayouts with the PipelineLayoutCreateInfo
    //     b.  That's actually about it.
    // 7.  Build GraphicsPipeline.
    //     a.  There's a lot here.
    //     b.  Shader stage, input assembly, vertex, viewport,
    // 8.  Begin recording command buffer.
    // 9.  Begin render pass.
    // 10. Bind Pipeline to command buffer.
    // 11. Bind DescriptorSets to the command command_buffer
    // 12. Issue cmdDraw with no vertices
    // 13. End render pass
    // 14. End command buffer recording.
    // 15. Issue command buffer on the Graphics queue
    // 16. Present the swapchain image to the presentation engine.
}

pub fn old_composite_test(
    ctxt: &VkContext,
    bg_image_view: &ImageView,
    view_fmt: Format,
    image_ready: Semaphore,
) {
    // let block_till_acquired = util::create_fence(ctxt);
    let signal_acquired = util::create_binary_semaphore(ctxt);

    let (image_index, image, _optimal) =
        swapchain::next_swapchain_image(signal_acquired, Fence::null());

    let attachments = vec![*image, *bg_image_view];
    // let attachments = vec![*image];

    let render_pass = make_render_pass(ctxt, view_fmt);
    let framebuffer = make_framebuffer(ctxt, render_pass, &attachments);

    let render_complete = util::create_binary_semaphore(ctxt);

    let mut clear_color = ClearColorValue::default();
    clear_color.float32 = [1.0f32, 1.0f32, 1.0f32, 1.0f32];
    clear_color.int32 = [i32::MIN, i32::MIN, i32::MIN, 1];
    clear_color.uint32 = [u32::MAX, u32::MAX, u32::MAX, 1];

    let mut clear_value = ClearValue::default();
    clear_value.color = clear_color;
    // let clear_values = [clear_value, clear_value];
    let clear_values = [clear_value];

    let pipeline_layout = create_pipeline_layout(ctxt);
    let pipeline = make_pipeline(ctxt, render_pass, pipeline_layout);

    let buffer = crate::graphics::pools::reserve_graphics_buffer(ctxt);

    let begin_info =
        CommandBufferBeginInfo::default().flags(CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    let wait_for_arr = [signal_acquired, image_ready];
    let signal_on_complete_arr = [render_complete];
    let command_buffers = [buffer];
    let submit_info = SubmitInfo::default()
        .wait_semaphores(&wait_for_arr)
        .wait_dst_stage_mask(&[PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
        .signal_semaphores(&signal_on_complete_arr)
        .command_buffers(&command_buffers);

    let block_till_queue_complete = util::create_fence(ctxt);

    unsafe {
        match ctxt
            .logical_device
            .begin_command_buffer(buffer, &begin_info)
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed to begin command buffer recording: {:?}", msg);
            }
        };

        let render_pass_begin = RenderPassBeginInfo::default()
            .framebuffer(framebuffer)
            .render_pass(render_pass)
            .render_area(ash::vk::Rect2D {
                offset: Offset2D::default().x(0).y(0),
                extent: Extent2D::default().width(1920).height(1080),
            })
            .clear_values(&clear_values);

        ctxt.logical_device.cmd_begin_render_pass(
            buffer,
            &render_pass_begin,
            SubpassContents::INLINE,
        );

        ctxt.logical_device
            .cmd_bind_pipeline(buffer, PipelineBindPoint::GRAPHICS, pipeline);

        ctxt.logical_device.cmd_draw(buffer, 3, 1, 0, 0);

        ctxt.logical_device.cmd_end_render_pass(buffer);

        match ctxt.logical_device.end_command_buffer(buffer) {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed to end command buffer recording: {:?}", msg);
            }
        }

        debug!("Submitting the render pass and framebuffer to the graphics queue.");

        match ctxt.logical_device.queue_submit(
            ctxt.graphics_queue,
            &[submit_info],
            block_till_queue_complete,
        ) {
            Ok(_) => {}
            Err(msg) => {
                panic!(
                    "Failed to submit the render buffer to the graphics queue: {:?}",
                    msg
                );
            }
        };
        // sleep(Duration::from_secs(3));
    }

    debug!("Attempting to present the swapchain image which should be cleared...");
    match swapchain::present_swapchain_image(image_index, &ctxt.graphics_queue, &[render_complete])
    {
        Ok(_) => {}
        Err(msg) => {
            panic!("Failed on swapchain present command: {:?}", msg);
        }
    };

    // sleep(Duration::from_secs(3));

    unsafe {
        match ctxt
            .logical_device
            .wait_for_fences(&[block_till_queue_complete], true, 10000000)
        {
            Ok(_) => {}
            Err(msg) => {
                panic!("Failed on wait for fence: {:?}", msg);
            }
        }
    };

    debug!("Reached end of render function.");

    unsafe {
        // ctxt.logical_device.destroy_fence(block_till_acquired, None);
        ctxt.logical_device
            .destroy_fence(block_till_queue_complete, None);
        ctxt.logical_device.destroy_semaphore(signal_acquired, None);
        ctxt.logical_device.destroy_semaphore(render_complete, None);
        ctxt.logical_device.destroy_semaphore(image_ready, None);
        ctxt.logical_device.destroy_framebuffer(framebuffer, None);
        ctxt.logical_device.destroy_render_pass(render_pass, None);
        ctxt.logical_device
            .destroy_pipeline_layout(pipeline_layout, None);
        ctxt.logical_device.destroy_pipeline(pipeline, None);
    }
}

fn make_framebuffer(
    ctxt: &VkContext,
    render_pass: RenderPass,
    attachments: &[ImageView],
) -> Framebuffer {
    match unsafe {
        ctxt.logical_device.create_framebuffer(
            &FramebufferCreateInfo::default()
                .width(ctxt.surface_capabilities.current_extent.width)
                .height(ctxt.surface_capabilities.current_extent.height)
                .attachments(attachments)
                .attachment_count(attachments.len() as u32)
                .layers(1)
                .render_pass(render_pass),
            None,
        )
    } {
        Ok(fb) => fb,
        Err(msg) => {
            panic!("Unable to create a framebuffer for our image: {:?}", msg);
        }
    }
}

fn make_render_pass(ctxt: &VkContext, view_fmt: Format) -> RenderPass {
    let sc_image_desc = make_color_description(swapchain::get_swapchain_format().format);
    let bg_image_desc = make_input_description(view_fmt);

    let attachment_descs = vec![sc_image_desc, bg_image_desc];
    // let attachment_descs = vec![sc_image_desc];

    let sc_image_attachment_ref = AttachmentReference::default()
        .attachment(0)
        .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    let bg_image_attachment_ref = AttachmentReference::default()
        .attachment(1)
        .layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL);

    let input_attachment_refs = [bg_image_attachment_ref];
    // let input_attachment_refs = [];
    let color_attachment_refs = [sc_image_attachment_ref];

    let subpass_one = make_subpass_description(&input_attachment_refs, &color_attachment_refs);
    let subpasses = [subpass_one];

    let src_dependency = make_dependency(SUBPASS_EXTERNAL, 0);
    // let dst_dependency = make_dependency(0, SUBPASS_EXTERNAL);
    // let dependencies = [src_dependency, dst_dependency];
    let dependencies = [src_dependency];

    match unsafe {
        ctxt.logical_device.create_render_pass(
            &RenderPassCreateInfo::default()
                .attachments(&attachment_descs)
                .flags(RenderPassCreateFlags::empty())
                .subpasses(&subpasses)
                .dependencies(&dependencies),
            None,
        )
    } {
        Ok(rp) => rp,
        Err(msg) => {
            panic!("Failed to construct a render pass: {:?}", msg);
        }
    }
}

fn make_dependency(src_id: u32, dst_id: u32) -> SubpassDependency {
    SubpassDependency::default()
        .src_subpass(src_id)
        .dst_subpass(dst_id)
        .src_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        // .src_access_mask(AccessFlags::MEMORY_WRITE)
        .dst_access_mask(AccessFlags::MEMORY_WRITE)
}

fn make_subpass_description<'a>(
    input_attachments: &'a [AttachmentReference],
    color_attachments: &'a [AttachmentReference],
) -> SubpassDescription<'a> {
    debug!("=== Subpass Description");
    debug!("    Input Attachment count: {}", input_attachments.len());
    for attachment in input_attachments {
        debug!(
            "    Input Attachment: {} - {:?}",
            attachment.attachment, attachment.layout
        );
    }

    debug!("    Color Attachment count: {}", color_attachments.len());
    for attachment in color_attachments {
        debug!(
            "    Input Attachment: {} - {:?}",
            attachment.attachment, attachment.layout
        );
    }

    SubpassDescription::default()
        .flags(SubpassDescriptionFlags::empty())
        .input_attachments(input_attachments)
        // .resolve_attachments(&[])
        .color_attachments(color_attachments)
        .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
        .preserve_attachments(&[])
}

fn make_input_description(format: Format) -> AttachmentDescription {
    make_description(format)
        .load_op(AttachmentLoadOp::LOAD)
        .store_op(AttachmentStoreOp::NONE)
        .initial_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .final_layout(ImageLayout::SHADER_READ_ONLY_OPTIMAL)
}

fn make_color_description(format: Format) -> AttachmentDescription {
    make_description(format)
        .load_op(AttachmentLoadOp::CLEAR)
        .store_op(AttachmentStoreOp::STORE)
        .stencil_load_op(AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(AttachmentStoreOp::DONT_CARE)
        .initial_layout(ImageLayout::UNDEFINED)
        .final_layout(ImageLayout::PRESENT_SRC_KHR)
}

fn make_description(format: Format) -> AttachmentDescription {
    AttachmentDescription::default()
        .format(format)
        .samples(SampleCountFlags::TYPE_1)
        .flags(AttachmentDescriptionFlags::empty())
}

fn make_pipeline(
    ctxt: &VkContext,
    render_pass: RenderPass,
    pipeline_layout: PipelineLayout,
) -> Pipeline {
    let shader_stage_infos = fill_pipeline_shader_stage_infos();
    let rasterization_state_info = create_rasterization_state();
    let input_assembly_state_info = create_input_assembly_state();
    let vertex_input_state_info = create_vertex_input_state();

    let swapchain_geometry = Rect2D::default()
        .extent(Extent2D::default().height(1080).width(1920))
        .offset(Offset2D::default().y(0).x(0));

    let viewport_geometry = Viewport::default()
        .width(1920f32)
        .height(1080f32)
        // .width(1.0f32)
        // .height(1.0f32)
        .x(0.0)
        .y(0.0)
        .min_depth(0.0f32)
        .max_depth(1.0f32);

    let fullscreen_scissors = vec![swapchain_geometry];
    let fullscreen_viewport = vec![viewport_geometry];

    let viewport_state = create_viewport_state(&fullscreen_scissors, &fullscreen_viewport);
    let multisample_state = create_multisample_state();

    let attachment_colorblend = create_attachment_colorblend_state();
    let attachment_colorblends = [attachment_colorblend];
    let pipeline_colorblend = create_pipeline_colorblend_state(&attachment_colorblends);

    let pipeline_create_info = GraphicsPipelineCreateInfo::default()
        .flags(PipelineCreateFlags::empty())
        .stages(&shader_stage_infos)
        .layout(pipeline_layout)
        .subpass(0)
        .render_pass(render_pass)
        // .dynamic_state(dynamic_state)
        .viewport_state(&viewport_state)
        .multisample_state(&multisample_state)
        .color_blend_state(&pipeline_colorblend)
        // .base_pipeline_index(base_pipeline_index)
        // .base_pipeline_handle(base_pipeline_handle)
        .vertex_input_state(&vertex_input_state_info)
        // .tessellation_state(tessellation_state)
        .rasterization_state(&rasterization_state_info)
        // .depth_stencil_state(depth_stencil_state)
        .input_assembly_state(&input_assembly_state_info);

    match unsafe {
        ctxt.logical_device.create_graphics_pipelines(
            PipelineCache::null(),
            &[pipeline_create_info],
            None,
        )
    } {
        Ok(pipelines) => *pipelines.first().unwrap(),
        Err(msg) => {
            panic!("Unable to construct the graphics pipeline: {:?}", msg);
        }
    }
}

fn fill_pipeline_shader_stage_infos<'a>() -> Vec<PipelineShaderStageCreateInfo<'a>> {
    // Yes I know.  unwrap bad.  This is speedrun territory.
    let fragment_shader = shaders::shader_by_name("compositor").unwrap();
    let vertex_shader = shaders::shader_by_name("passthrough").unwrap();

    let vertex_shader_stage_info = PipelineShaderStageCreateInfo::default()
        .name(vertex_shader.name.as_c_str())
        .flags(PipelineShaderStageCreateFlags::empty())
        .stage(ShaderStageFlags::VERTEX)
        .module(vertex_shader.shader_module);

    let compositor_shader_stage_info = PipelineShaderStageCreateInfo::default()
        .name(fragment_shader.name.as_c_str())
        .flags(PipelineShaderStageCreateFlags::empty())
        .stage(ShaderStageFlags::FRAGMENT)
        .module(fragment_shader.shader_module);

    vec![vertex_shader_stage_info, compositor_shader_stage_info]
}

fn create_pipeline_layout(ctxt: &VkContext) -> PipelineLayout {
    let create_info = PipelineLayoutCreateInfo::default()
        .flags(PipelineLayoutCreateFlags::empty())
        .push_constant_ranges(&[])
        .set_layouts(&[]);

    match unsafe {
        ctxt.logical_device
            .create_pipeline_layout(&create_info, None)
    } {
        Ok(layout) => layout,
        Err(msg) => {
            panic!("Failed to create the pipeline layout: {:?}", msg);
        }
    }
}

fn create_rasterization_state<'a>() -> PipelineRasterizationStateCreateInfo<'a> {
    PipelineRasterizationStateCreateInfo::default()
        .flags(PipelineRasterizationStateCreateFlags::empty())
        .depth_clamp_enable(false)
        .cull_mode(CullModeFlags::BACK)
        .front_face(FrontFace::CLOCKWISE)
        .polygon_mode(PolygonMode::FILL)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0f32)
        .depth_bias_clamp(0.0f32)
        .depth_bias_slope_factor(0.0f32)
        .rasterizer_discard_enable(false)
        .line_width(1.0f32)
}

fn create_viewport_state<'a>(
    dest_image_geometry: &'a [Rect2D],
    viewport_geometry: &'a [Viewport],
) -> PipelineViewportStateCreateInfo<'a> {
    PipelineViewportStateCreateInfo::default()
        .flags(PipelineViewportStateCreateFlags::empty())
        .scissors(dest_image_geometry)
        .scissor_count(1)
        .viewports(viewport_geometry)
        .viewport_count(1)
}

fn create_input_assembly_state<'a>() -> PipelineInputAssemblyStateCreateInfo<'a> {
    PipelineInputAssemblyStateCreateInfo::default()
        .flags(PipelineInputAssemblyStateCreateFlags::empty())
        .topology(PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false)
}

fn create_vertex_input_state<'a>() -> PipelineVertexInputStateCreateInfo<'a> {
    PipelineVertexInputStateCreateInfo::default()
        .flags(PipelineVertexInputStateCreateFlags::empty())
        .vertex_binding_descriptions(&[])
        .vertex_attribute_descriptions(&[])
}

fn create_multisample_state<'a>() -> PipelineMultisampleStateCreateInfo<'a> {
    PipelineMultisampleStateCreateInfo::default()
        .flags(PipelineMultisampleStateCreateFlags::empty())
        .sample_shading_enable(false)
        .rasterization_samples(SampleCountFlags::TYPE_1)
        .alpha_to_one_enable(false)
        .alpha_to_coverage_enable(false)
        .min_sample_shading(1.0)
}

fn create_pipeline_colorblend_state(
    attachment_blend: &[PipelineColorBlendAttachmentState],
) -> PipelineColorBlendStateCreateInfo {
    PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .attachments(attachment_blend)
        .blend_constants([0.0f32, 0.0f32, 0.0f32, 0.0f32])
}

fn create_attachment_colorblend_state() -> PipelineColorBlendAttachmentState {
    PipelineColorBlendAttachmentState::default()
        .color_write_mask(ColorComponentFlags::RGBA)
        .blend_enable(false)
}
