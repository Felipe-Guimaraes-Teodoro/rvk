// initialization

use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer};

// syncing
use vulkano::sync::{self, GpuFuture};

// image creation 
use vulkano::image::{Image, ImageCreateInfo, ImageUsage};
use vulkano::format::Format;
use vulkano::command_buffer::ClearColorImageInfo;
use vulkano::format::ClearColorValue;
use vulkano::command_buffer::CopyImageToBufferInfo;

use vulkano::pipeline::compute::ComputePipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    ComputePipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};

use vulkano::image::view::ImageView;

mod vk_utils;
mod buffer;

mod compute_shader {
    vulkano_shaders::shader!{
        ty: "compute",
        src: r#"
            #version 460

            layout (local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

            layout (set = 0, binding = 0, rgba8) uniform writeonly image2D img;

            void main() {
                vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) / vec2(imageSize(img));
                float zoom = 1.5;
                vec2 pos = vec2(1.0, 0.0);
                vec2 c = (norm_coordinates - vec2(0.5)) * 1.0 / zoom - pos;

                vec2 z = vec2(0.0, 0.0);
                float i;
                for (i = 0.0; i < 1.0; i += 0.005) {
                    z = vec2(
                        z.x * z.x - z.y * z.y + c.x,
                        z.y * z.x + z.x * z.y + c.y
                    );

                    if (length(z) > 4.0) {
                        break;
                    }
                }

                vec4 to_write = vec4(vec3(i), 1.0);
                imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
            }
        "#,
    }
}

fn main() {
    // Initialization // 
    let vk = vk_utils::VK;
    let device = &vk.device;
    let queue = &vk.queue;
    let memory_allocator = &vk.memory_allocator;

    let shader = compute_shader::load(device.clone()).expect("failed to create shader module");

    let cs = shader.entry_point("main").unwrap();
    let stage = PipelineShaderStageCreateInfo::new(cs);
    let layout = PipelineLayout::new(
        device.clone(),
        PipelineDescriptorSetLayoutCreateInfo::from_stages([&stage])
            .into_pipeline_layout_create_info(device.clone())
            .unwrap(),
    )
    .unwrap();

    let compute_pipeline = ComputePipeline::new(
        device.clone(),
        None,
        ComputePipelineCreateInfo::stage_layout(stage, layout),
    )
    .expect("failed to create compute pipeline");

    let res = 30000;

    let image = vk.image([res, res, 1]);

    let view = ImageView::new_default(image.clone()).unwrap();
    let layout = compute_pipeline.layout().set_layouts().get(0).unwrap();
    let set = vulkano::descriptor_set::PersistentDescriptorSet::new(
            &vk.descriptor_set_allocator,
            layout.clone(),
            [vulkano::descriptor_set::WriteDescriptorSet::image_view(0, view)],
            [],
        )
        .unwrap();

    let buf = vk.buf_iter((0..res * res * 4).map(|_| 0u8)); // buffer must be the size of the
                                                              // image


    let mut command_buffer_builder = vk.builder(); 
    command_buffer_builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .unwrap()
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0, 
            set
        )
        .unwrap()
        .dispatch([res / 8, res / 8, 1])
        .unwrap()
        .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(image, buf.clone()))
        .unwrap();

    let command_buffer = command_buffer_builder.build().unwrap();

    vk.sync(command_buffer);


    let buf_content = buf.read().unwrap();
    let image = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(res, res, &buf_content[..]).unwrap();
    image.save("image.png").unwrap();

    println!("succeed!");
}
