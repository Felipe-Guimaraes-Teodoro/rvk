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
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::swapchain::Surface;
use winit::event_loop::EventLoop;

mod vk_utils;
mod buffer;
mod vk_pipeline;

use crate::vk_pipeline::Pipeline;

mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec3 position;

            void main() {
                gl_Position = vec4(position, 1.0);
            }
        ",
    }
}

mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        src: "
            #version 460

            layout(location = 0) out vec4 f_color;

            void main() {
                f_color = vec4(1.0, 0.0, 0.0, 1.0);
            }
        ",
    }
}


#[repr(C)]
#[derive(BufferContents, Vertex)]
struct FVertex3d {
    #[format(R32G32_SFLOAT)]
    position: [f32; 3],
}

fn main() {
    // Initialization // 
    let mut vk = vk_utils::VK;

    // test

    let vs = vs::load(vk.device.clone()).expect("failed to create shader module");
    let fs = fs::load(vk.device.clone()).expect("failed to create shader module");

    let verts = vec![
        FVertex3d { position: [-0.5, -0.5, 0.0 ] }, 
        FVertex3d { position: [0.0, 0.5, 0.0 ] }, 
        FVertex3d { position: [0.5, -0.25, 0.0 ] }, 
    ];

    let vertex_buffer = Buffer::from_iter(
        vk.memory_allocator.clone(),
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        verts
    )
    .unwrap();

    let image = vk.image([1024, 1024, 1]);
    let image_buffer = vk.buf_iter((0..1024 * 1024 * 4).map(|_| 0u8));

    let vk_pipeline = vk.pipeline(vs, fs);
    let framebuffer = vk.framebuffer_from_image(image.clone(), vk_pipeline.clone());

    let mut builder = vk.builder();
    builder
        .begin_render_pass(
            vulkano::command_buffer::RenderPassBeginInfo {
                clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
                ..vulkano::command_buffer::RenderPassBeginInfo::framebuffer(framebuffer.clone())
            },
            vulkano::command_buffer::SubpassBeginInfo {
                contents: vulkano::command_buffer::SubpassContents::Inline,
                ..Default::default()
            },
        )
        .unwrap()

        .bind_pipeline_graphics(vk_pipeline.lock().unwrap().pipeline.clone())
        .unwrap()
        .bind_vertex_buffers(0, vertex_buffer.clone())
        .unwrap()
        .draw(
            3, 1, 0, 0, 
        )
        .unwrap()
        .end_render_pass(vulkano::command_buffer::SubpassEndInfo::default())
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(image, image_buffer.clone()))
        .unwrap();

    let command_buffer = builder.build().unwrap();

    vk.sync(command_buffer);

    let buffer_content = image_buffer.read().unwrap();

    let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
    image.save("image.png").unwrap();

    println!("succeed!");
}

