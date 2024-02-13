use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex, MutexGuard};

use winit::event_loop::EventLoop;
use winit::event::*;
use winit::window::WindowBuilder;

use vulkano::swapchain::Surface;

use vulkano::swapchain;
use vulkano::{Validated, VulkanError};
use vulkano::swapchain::SwapchainPresentInfo;

use vulkano::sync::{self, GpuFuture};

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

use crate::vk_pipeline::vert;
pub fn run() {
    let event_loop = EventLoop::new();
    let mut vk = crate::vk_utils::Vk::new(&event_loop);

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap()); 
    window.set_title("VULKAN");
    let surface = Surface::from_window(vk.instance.clone(), window.clone()).unwrap();

    let mut viewport = vulkano::pipeline::graphics::viewport::Viewport {
        offset: [0.0, 0.0],
        extent: window.inner_size().into(),
        depth_range: 0.0..=1.0,
    };
    let vs = vs::load(vk.device.clone()).expect("failed"); 
    let fs = fs::load(vk.device.clone()).expect("failed"); 

    let vertex_buffer = vk.vertex_buffer(
        vec![ vert(0.0, 0.0, 0.0), vert(1.0, 0.0, 0.0), vert(0.0, -0.5, 0.0) ],
    ); 

    vk.set_swapchain(surface, &window);
    let swapchain = vk.swapchain.clone().unwrap();
    let render_pass = vk.get_render_pass();
    let framebuffers = vk.get_framebuffers(&render_pass);
    let pipeline = vk.get_pipeline(vs.clone(), fs.clone(), render_pass.clone(), viewport.clone());
    let mut command_buffers = vk.get_command_buffers(&pipeline, &framebuffers, &vertex_buffer);

    let mut window_resized = false; 
    let mut recreate_swapchain = false;

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            },

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                window_resized = true;
            },

            Event::MainEventsCleared => {
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;
                    let new_dim = window.inner_size();

                    let (new_swpchain, new_imgs) = swapchain 
                        .recreate(vulkano::swapchain::SwapchainCreateInfo {
                            image_extent: new_dim.into(),
                            ..swapchain.create_info()
                        })
                    .expect("failed to recreate swpchain");

                    vk.swapchain = Some(new_swpchain);
                    vk.images = Some(new_imgs);
                    let new_framebuffers = vk.get_framebuffers(&render_pass);

                    if window_resized {
                        window_resized = false;

                        viewport.extent = new_dim.into();
                        let new_pipeline = vk.get_pipeline(vs.clone(), fs.clone(), render_pass.clone(), viewport.clone());
                        command_buffers = vk.get_command_buffers(&new_pipeline, &new_framebuffers, &vertex_buffer);
                    }


                    let (image_i, suboptimal, acquire_future) = 
                        match swapchain::acquire_next_image(swapchain.clone(), None)
                            .map_err(Validated::unwrap) 
                        {
                            Ok(r) => r,
                            Err(VulkanError::OutOfDate) => {
                                recreate_swapchain = true;
                                return;
                            }
                            Err(e) => panic!("failed to acquire next image: {e}"),
                        };

                    if suboptimal { recreate_swapchain = true; }

                    let execution = sync::now(vk.device.clone())
                        .join(acquire_future)
                        .then_execute(vk.queue.clone(), command_buffers[image_i as usize].clone())
                        .unwrap()
                        .then_swapchain_present(
                            vk.queue.clone(),
                            SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_i),
                        )
                        .then_signal_fence_and_flush();
                }
            },

            _ => () 
        }
    });
}
