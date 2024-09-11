use ash::vk::{
    AccessFlags, AttachmentDescription, AttachmentDescriptionFlags, AttachmentLoadOp,
    AttachmentReference, AttachmentStoreOp, ClearColorValue, ClearValue, Extent2D, Format,
    Framebuffer, FramebufferCreateInfo, ImageLayout, ImageView, Offset2D, PipelineBindPoint,
    PipelineStageFlags, RenderPass, RenderPassBeginInfo, RenderPassCreateFlags,
    RenderPassCreateInfo, SampleCountFlags, SubpassContents, SubpassDependency, SubpassDescription,
    SubpassDescriptionFlags, ATTACHMENT_UNUSED, SUBPASS_EXTERNAL,
};

use log::debug;

use crate::setup::instance::VkContext;

use super::{swapchain, util};

pub fn perform_simple_render(ctxt: &VkContext, bg_image_view: &ImageView, view_fmt: Format) {
    let block_till_acquired = util::create_fence(ctxt);
    let signal_acquired = util::create_binary_semaphore(ctxt);

    let (image_index, image, optimal) =
        swapchain::next_swapchain_image(signal_acquired, block_till_acquired);

    let attachments = vec![*image, *bg_image_view];

    let render_pass = make_render_pass(ctxt, view_fmt);
    let framebuffer = make_framebuffer(ctxt, render_pass, &attachments);

    let render_complete = util::create_binary_semaphore(ctxt);

    let mut clear_color = ClearColorValue::default();
    clear_color.uint32 = [0, 0, 0, 0xFFFFFFFF];
    let mut clear_value = ClearValue::default();
    clear_value.color = clear_color;
    let clear_values = [clear_value, clear_value];

    let buffer = crate::graphics::pools::reserve_graphics_buffer(ctxt);

    let render_pass_begin = RenderPassBeginInfo::default()
        .framebuffer(framebuffer)
        .render_pass(render_pass)
        .render_area(ash::vk::Rect2D {
            offset: Offset2D::default().x(0).y(0),
            extent: Extent2D::default().width(1920).height(1080),
        })
        .clear_values(&clear_values);

    unsafe {
        ctxt.logical_device.cmd_begin_render_pass(
            buffer,
            &render_pass_begin,
            SubpassContents::INLINE,
        );
    }

    unsafe {
        ctxt.logical_device.cmd_end_render_pass(buffer);
    }

    swapchain::present_swapchain_image(image_index, &ctxt.graphics_queue, &[render_complete]);

    debug!("Reached end of render function.");

    unsafe {
        ctxt.logical_device.destroy_fence(block_till_acquired, None);
        ctxt.logical_device.destroy_semaphore(signal_acquired, None);
        ctxt.logical_device.destroy_semaphore(render_complete, None);
        ctxt.logical_device.destroy_framebuffer(framebuffer, None);
        ctxt.logical_device.destroy_render_pass(render_pass, None);
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
                .attachment_count(2)
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

    let sc_image_attachment_ref = AttachmentReference::default()
        .attachment(0)
        .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    let bg_image_attachment_ref = AttachmentReference::default()
        .attachment(1)
        .layout(ImageLayout::READ_ONLY_OPTIMAL);
    let input_attachment_refs = [bg_image_attachment_ref];
    let color_attachment_refs = [sc_image_attachment_ref];

    let subpass_one = make_subpass_description(&input_attachment_refs, &color_attachment_refs);
    let subpasses = [subpass_one];

    let src_dependency = make_dependency(SUBPASS_EXTERNAL, 0);
    let dst_dependency = make_dependency(0, SUBPASS_EXTERNAL);
    let dependencies = [src_dependency, dst_dependency];

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
        .src_access_mask(AccessFlags::MEMORY_WRITE)
        .dst_access_mask(AccessFlags::MEMORY_READ)
}

fn make_subpass_description<'a>(
    input_attachments: &'a [AttachmentReference],
    color_attachments: &'a [AttachmentReference],
) -> SubpassDescription<'a> {
    SubpassDescription::default()
        .flags(SubpassDescriptionFlags::empty())
        .input_attachments(input_attachments)
        .color_attachments(color_attachments)
        .pipeline_bind_point(PipelineBindPoint::GRAPHICS)
        .resolve_attachments(&[])
        .preserve_attachments(&[])
}

fn make_input_description(format: Format) -> AttachmentDescription {
    make_description(format)
        .load_op(AttachmentLoadOp::LOAD)
        .store_op(AttachmentStoreOp::NONE)
        .initial_layout(ImageLayout::READ_ONLY_OPTIMAL)
        .final_layout(ImageLayout::READ_ONLY_OPTIMAL)
}

fn make_color_description(format: Format) -> AttachmentDescription {
    make_description(format)
        .load_op(AttachmentLoadOp::CLEAR)
        .store_op(AttachmentStoreOp::STORE)
        .initial_layout(ImageLayout::UNDEFINED)
        .final_layout(ImageLayout::PRESENT_SRC_KHR)
}

fn make_description(format: Format) -> AttachmentDescription {
    AttachmentDescription::default()
        .format(format)
        .samples(SampleCountFlags::TYPE_1)
        .flags(AttachmentDescriptionFlags::empty())
}
