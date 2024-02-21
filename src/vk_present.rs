use std::sync::{Arc, Mutex};

use winit::event_loop::EventLoop;
use winit::event::*;
use winit::window::WindowBuilder;

use vulkano::swapchain::Surface;

use vulkano::swapchain;
use vulkano::{Validated, VulkanError};
use vulkano::swapchain::{SwapchainPresentInfo, SwapchainAcquireFuture, PresentFuture};

use vulkano::sync::{self, GpuFuture};
use vulkano::sync::future::FenceSignalFuture;

use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::{
    CommandBufferExecFuture,
    PrimaryAutoCommandBuffer, 
    allocator::StandardCommandBufferAllocator
};

use vulkano::pipeline::GraphicsPipeline;
use vulkano::sync::future::JoinFuture;
use vulkano::render_pass::Framebuffer;

use crate::vk_pipeline::FVertex3d;
use crate::vk_utils::Vk;

pub mod vs {
    vulkano_shaders::shader!{
        ty: "vertex",
        src: r"
            #version 460

            layout(location = 0) in vec3 position;
            layout(location = 0) out vec3 pos;

            void main() {
                vec2 outUV = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
                gl_Position = vec4(outUV * 2.0 - 1.0, 0.0, 1.0);

                pos = vec3(outUV * 2.0 - 1.0, 0.0);
            }
        ",
    }
}

pub mod fs {
    vulkano_shaders::shader!{
        ty: "fragment",
        src: "
            #version 460

            layout(location = 0) out vec4 f_color;
            layout(location = 0) in vec3 pos;

            layout(push_constant) uniform PushConstantData {
                highp float time;
                highp vec2 cpos;
                highp vec2 ires;
                highp float zoom;
            } pc;

            float mandelbrot(vec2 c) {
                highp vec2 z = vec2(0.0, 0.0);
                highp float i;

                for (i = 0.0; i < 1.0; i += 0.01) {
                    z = vec2(
                        z.x * z.x - z.y * z.y + c.x,
                        z.y * z.x + z.x * z.y + c.y
                    );

                    if (length(z) > 4.0) {
                        break;
                    }
                }

                return i;
            }

            void main() {
                highp float i = 0.0;
                float num_samples = 2;

                // for (int s = 0; s < num_samples; ++s) {
                    highp vec2 jitter = vec2(1, 1);
                    highp vec2 samplePos = pos.xy * pc.zoom + jitter / pc.ires - pc.cpos;
                    samplePos.y *= 1.0 / (pc.ires.x / pc.ires.y);

                    i += mandelbrot(samplePos);
                // }
                
                highp float avgI = i;

                f_color = vec4(vec2(avgI), sin(pc.time), 1.0);
            }
        ",
    }
}

use once_cell::sync::Lazy;

pub static FRAGMENT_PUSH_CONSTANTS: Lazy<Mutex<fs::PushConstantData>> = Lazy::new(|| {
    Mutex::new(
        fs::PushConstantData {
            time: 0.0.into(),
            cpos: [0.0, 0.0],
            ires: [800.0, 800.0],
            zoom: 1.0,
        }
    )
});

pub struct VkPresenter {
    pub viewport: vulkano::pipeline::graphics::viewport::Viewport,
    pub shader_mods: Vec<Arc<vulkano::shader::ShaderModule>>,
    pub vert_buffers: Vec<Subbuffer<[FVertex3d]>>,
    pub surface: Arc<Surface>,
    pub framebuffers : Vec<Arc<Framebuffer>>,
    pub render_pass: Arc<vulkano::render_pass::RenderPass>,

    pub pipeline: Arc<GraphicsPipeline>,
    pub layout: Arc<vulkano::pipeline::layout::PipelineLayout>,

    pub command_buffers: Vec<Arc<PrimaryAutoCommandBuffer<StandardCommandBufferAllocator>>>,


    pub window_resized: bool, pub recreate_swapchain: bool, 
    pub frames_in_flight: usize,
    pub previous_fence_i: u32, 
    pub fences: Vec<Option<Arc<FenceSignalFuture<PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>>>>>,
}

use crate::vk_pipeline::vert;
impl VkPresenter {
    pub fn new(vk: &mut Vk, window: Arc<winit::window::Window>) -> Self {
        let surface = Surface::from_window(vk.instance.clone(), window.clone()).unwrap();
        let viewport = vulkano::pipeline::graphics::viewport::Viewport {
            offset: [0.0, 0.0],
            extent: window.inner_size().into(),
            depth_range: 0.0..=1.0,
        };
        let vert_buffers = vec![
            vk.vertex_buffer(
                vec![
                    vert(0.0, 0.0, 0.0), 
                    vert(0.0, 0.0, 0.0),
                    vert(0.0, 0.0, 0.0),
                ],
            ),
        ];
        let vs = vs::load(vk.device.clone()).unwrap();
        let fs = fs::load(vk.device.clone()).unwrap();

        vk.set_swapchain(surface.clone(), &window);
        let images = vk.images.clone().unwrap();
        let render_pass = vk.get_render_pass();
        let framebuffers = vk.get_framebuffers(&render_pass);
        let (pipeline, layout) = vk.get_pipeline(
            vs.clone(), 
            fs.clone(), 
            render_pass.clone(), 
            viewport.clone()
        );

        let command_buffers = vk.get_command_buffers(
            &pipeline, 
            &framebuffers, 
            &vert_buffers[0], 
            layout.clone(), 
            *FRAGMENT_PUSH_CONSTANTS.lock().unwrap()
        );

        let window_resized = false;
        let recreate_swapchain = false;

        let frames_in_flight = images.len();
        let fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];
        let previous_fence_i = 0;

        Self {
            surface,
            render_pass,
            viewport,
            vert_buffers,
            shader_mods: vec![vs, fs],
            framebuffers,
            pipeline,
            layout, 
            command_buffers,
            window_resized, recreate_swapchain,
            frames_in_flight, fences, previous_fence_i,
        }
    }

    pub fn if_recreate_swapchain(&mut self, window: Arc<winit::window::Window>, vk: &mut Vk) {
        if self.window_resized || self.recreate_swapchain {
            self.recreate_swapchain = false;
            let new_dim = window.inner_size();

            FRAGMENT_PUSH_CONSTANTS.lock().unwrap().ires = new_dim.into(); 

            let (new_swpchain, new_imgs) = vk.swapchain.clone().unwrap()
                .recreate(vulkano::swapchain::SwapchainCreateInfo {
                    image_extent: new_dim.into(),
                    ..vk.swapchain.clone().unwrap().create_info()
                })
            .expect("failed to recreate swpchain");

            vk.swapchain = Some(new_swpchain);
            vk.images = Some(new_imgs);
            self.framebuffers = vk.get_framebuffers(&self.render_pass);

            if self.window_resized {
                self.window_resized = false;

                self.viewport.extent = new_dim.into();
                (self.pipeline, self.layout) = vk.get_pipeline(
                    self.shader_mods[0].clone(), 
                    self.shader_mods[1].clone(), 
                    self.render_pass.clone(), 
                    self.viewport.clone()
                );
            }
        }
    }

    pub fn update(&mut self, vk: &mut Vk) {
        self.command_buffers = vk.get_command_buffers(
            &self.pipeline.clone(), 
            &self.framebuffers, 
            &self.vert_buffers[0], 
            self.layout.clone(),
            *FRAGMENT_PUSH_CONSTANTS.lock().unwrap(),
        );

    }

    pub fn present(&mut self, vk: &mut Vk) {
        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(vk.swapchain.clone().unwrap(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return;
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None).unwrap();
        }

        let previous_future = match self.fences[self.previous_fence_i as usize].clone() {
            None => {
                let mut now = sync::now(vk.device.clone());
                now.cleanup_finished();
                now.boxed()
            }
            Some(fence) => fence.boxed(),
        };

        let future = previous_future
            .join(acquire_future)
            .then_execute(vk.queue.clone(), self.command_buffers[image_i as usize].clone())
            .unwrap()
            .then_swapchain_present(
                vk.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(vk.swapchain.clone().unwrap(), image_i),
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future.map_err(Validated::unwrap) {
            Ok(value) => Some(Arc::new(value)),
            Err(VulkanError::OutOfDate) => {
                self.recreate_swapchain = true;
                None
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };

        self.previous_fence_i = image_i;
        
    }
}
