use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, Subpass};
use vulkano::command_buffer::CopyImageToBufferInfo;
use vulkano::shader::ShaderModule;
use vulkano::render_pass::RenderPass;
use vulkano::image::Image;

use std::sync::{Arc, Mutex};

use crate::vk_utils::Vk;

#[repr(C)]
#[derive(BufferContents, Vertex)]
struct FVertex2d {
    #[format(R32G32_SFLOAT)]
    position: [f32; 2],
}

#[derive(Clone)]
pub struct Pipeline {
    pub viewport: Arc<Viewport>,
    pub render_pass: Arc<RenderPass>,
    pub pipeline: Arc<GraphicsPipeline>,
    pub framebuffer: Option<Arc<Framebuffer>>,
}

impl Vk {
    pub fn pipeline(&self, vs: Arc<ShaderModule>, fs: Arc<ShaderModule>) -> 
        Arc<Mutex<Pipeline>> 
    {
        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: self.resolution,
            depth_range: 0.0..=1.0,
        };

        let render_pass = vulkano::single_pass_renderpass!(
            self.device.clone(),
            attachments: {
                color: {
                    format: Format::R8G8B8A8_UNORM,
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
        .unwrap();

        let pipeline = {
            let vs = vs.entry_point("main").unwrap();
            let fs = fs.entry_point("main").unwrap();

            let vertex_input_state = FVertex2d::per_vertex()
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

            GraphicsPipeline::new(
                self.device.clone(),
                None,
                GraphicsPipelineCreateInfo {
                    stages: stages.into_iter().collect(),
                    vertex_input_state: Some(vertex_input_state),
                    input_assembly_state: Some(InputAssemblyState::default()),
                    viewport_state: Some(ViewportState {
                       viewports: [viewport.clone()].into_iter().collect(),
                        ..Default::default()
                    }),
                    rasterization_state: Some(vulkano::pipeline::graphics::rasterization::RasterizationState::default()),
                    multisample_state: Some(vulkano::pipeline::graphics::multisample::MultisampleState::default()),
                    color_blend_state: Some(vulkano::pipeline::graphics::color_blend::ColorBlendState::with_attachment_states(
                        subpass.num_color_attachments(),
                        vulkano::pipeline::graphics::color_blend::ColorBlendAttachmentState::default(),
                    )),
                    subpass: Some(subpass.into()),
                    ..GraphicsPipelineCreateInfo::layout(layout)
                },
            )
            .unwrap()
        };

        Arc::new(Mutex::new(Pipeline {
            viewport: viewport.into(),
            render_pass,
            pipeline,
            framebuffer: None
        }))
   
    }

    pub fn framebuffer_from_image(
        &self, 
        image: Arc<Image>, 
        pipeline: Arc<Mutex<Pipeline>>
    ) -> Arc<Framebuffer> {
        let view = ImageView::new_default(image.clone()).unwrap();
        Framebuffer::new(
            pipeline.lock().unwrap().render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![view],
                ..Default::default()
            },
        )
        .unwrap()
    }
}
