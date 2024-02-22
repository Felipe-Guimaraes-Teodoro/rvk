use std::sync::Arc;

use vulkano::buffer::{BufferContents, Subbuffer};
use vulkano::command_buffer::{
    AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
    SubpassBeginInfo, SubpassContents,
};
use vulkano::image::view::ImageView;
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::ShaderModule;

use crate::vk_utils::Vk;

#[repr(C)]
#[derive(BufferContents, Vertex)]
pub struct FVertex3d {
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],
}

pub fn vert(x: f32, y: f32, z: f32) -> FVertex3d {
    FVertex3d {
        position: [x, y, z],
    }
}

#[derive(Clone)]
pub struct Pipeline {
    pub viewport: Arc<Viewport>,
    pub render_pass: Arc<RenderPass>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub framebuffer: Option<Arc<Framebuffer>>,
}

impl Vk {
    pub fn get_render_pass(&self) -> Arc<RenderPass> {
        vulkano::single_pass_renderpass!(
            self.device.clone(),
            attachments: {
                color: {
                    // Set the format the same as the swapchain.
                    format: self.swapchain.clone().unwrap().image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {},
            },
        )
        .unwrap()
    }


    pub fn get_framebuffers(
        &self,
        // images: &[Arc<Image>],
        render_pass: &Arc<RenderPass>,
    ) -> Vec<Arc<Framebuffer>> {
        self.images.clone().unwrap().as_slice()
            .iter()
            .map(|image| {
                let view = ImageView::new_default(image.clone()).unwrap();
                Framebuffer::new(
                    render_pass.clone(),
                    FramebufferCreateInfo {
                        attachments: vec![view],
                        ..Default::default()
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>()
    }

    pub fn get_pipeline(
        &self,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> (Arc<GraphicsPipeline>, Arc<PipelineLayout>) {
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();

        let vertex_input_state = FVertex3d::per_vertex()
            .definition(&vs.info().input_interface)
            .unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            self.device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(self.device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        (GraphicsPipeline::new(
            self.device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState::default()),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout.clone())
            },
        )
        .unwrap(), layout)
    }

    pub fn get_command_buffers(
        &self,
        pipeline: &Arc<GraphicsPipeline>,
        framebuffers: &[Arc<Framebuffer>],
        vertex_buffer: &Subbuffer<[FVertex3d]>,
        layout: Arc<PipelineLayout>,
        push_constant: crate::vk_present::fs::PushConstantData,
    ) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
        framebuffers
            .iter()
            .map(|framebuffer| {
                let mut builder = AutoCommandBufferBuilder::primary(
                    &self.mem_allocators.command_buffer_allocator,
                    self.queue.queue_family_index(),
                    CommandBufferUsage::MultipleSubmit,
                )
                .unwrap();

                builder
                    .begin_render_pass(
                        RenderPassBeginInfo {
                            clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                            ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                        },
                        SubpassBeginInfo {
                            contents: SubpassContents::Inline,
                            ..Default::default()
                        },
                    )
                    .unwrap()
                    .push_constants(layout.clone(), 0, push_constant)
                    .unwrap()
                    .bind_pipeline_graphics(pipeline.clone())
                    .unwrap()
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .unwrap()
                    .draw(vertex_buffer.len() as u32, 1, 0, 0)
                    .unwrap()
                    .end_render_pass(Default::default())
                    .unwrap();

                builder.build().unwrap()
            })
            .collect()
    }
}
