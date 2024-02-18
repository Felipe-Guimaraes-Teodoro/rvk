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

            void main() {
                gl_Position = vec4(position, 1.0);
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

            layout(push_constant) uniform PushConstantData {
                float time;
            } pc;

            void main() {
                f_color = vec4(sin(pc.time), 0.0, 0.0, 1.0);
            }
        ",
    }
}

static FRAGMENT_PUSH_CONSTANTS: Mutex<fs::PushConstantData> = Mutex::new(
    fs::PushConstantData {
        time: 0.0,
    }
);

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
                vec![vert(0.0, 0.0, 0.0), vert(1.0, 1.0, 0.0), vert(-1.0, 0.0, 0.0)],
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

    pub fn present(&mut self) {}

    pub fn on_window_resized(&mut self) {}
}
