use ash::vk::{
    AttachmentDescription, AttachmentDescriptionFlags, AttachmentLoadOp, AttachmentReference,
    AttachmentStoreOp, Format, FramebufferCreateInfo, ImageLayout, ImageView, PipelineBindPoint,
    RenderPassCreateFlags, RenderPassCreateInfo, SampleCountFlags, SubpassDescription,
    SubpassDescriptionFlags, ATTACHMENT_UNUSED,
};

use crate::setup::instance::VkContext;

use super::{swapchain, util};

pub fn perform_simple_render(ctxt: &VkContext, bg_image_view: &ImageView, view_fmt: Format) {
    let block_till_acquired = util::create_fence(ctxt);
    let signal_acquired = util::create_binary_semaphore(ctxt);

    let (image_index, image, optimal) =
        swapchain::next_swapchain_image(signal_acquired, block_till_acquired);

    let sc_image_desc = make_description(swapchain::get_swapchain_format().format);
    let bg_image_desc = make_description(view_fmt);

    let sc_image_attachment_ref = AttachmentReference::default()
        .attachment(0)
        .layout(ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let attachments = vec![*image, *bg_image_view];
    let attachment_descs = vec![sc_image_desc, bg_image_desc];

    let render_pass = match unsafe {
        ctxt.logical_device.create_render_pass(
            &RenderPassCreateInfo::default()
                .attachments(&attachment_descs)
                .flags(RenderPassCreateFlags::empty())
                .subpasses(subpasses),
            None,
        )
    } {
        Ok(rp) => rp,
        Err(msg) => {
            panic!("Failed to construct a render pass: {:?}", msg);
        }
    };

    let framebuffer = match unsafe {
        ctxt.logical_device.create_framebuffer(
            &FramebufferCreateInfo::default()
                .width(ctxt.surface_capabilities.current_extent.width)
                .height(ctxt.surface_capabilities.current_extent.height)
                .attachments(&attachments)
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
    };

    let render_complete = util::create_binary_semaphore(ctxt);
    swapchain::present_swapchain_image(image_index, &ctxt.graphics_queue, &[render_complete]);

    unsafe {
        ctxt.logical_device.destroy_fence(block_till_acquired, None);
        ctxt.logical_device.destroy_semaphore(signal_acquired, None);
        ctxt.logical_device.destroy_semaphore(render_complete, None);
        ctxt.logical_device.destroy_framebuffer(framebuffer, None);
    }
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

fn make_description(format: Format) -> AttachmentDescription {
    AttachmentDescription::default()
        .format(format)
        .samples(SampleCountFlags::TYPE_1)
        .flags(AttachmentDescriptionFlags::empty())
        .load_op(AttachmentLoadOp::CLEAR)
        .store_op(AttachmentStoreOp::STORE)
        .initial_layout(ImageLayout::UNDEFINED)
        .final_layout(ImageLayout::PRESENT_SRC_KHR)
}
