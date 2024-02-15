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
use vulkano::sync::future::FenceSignalFuture;

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

            layout(push_constant) uniform PushConstantData {
                float time;
            } pc;

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

    let push_constants = fs::PushConstantData {
        time: 0.0, 
    };

    let vertex_buffer = vk.vertex_buffer(
        vec![ vert(0.0, 0.0, 0.0), vert(1.0, 0.0, 0.0), vert(0.0, -0.5, 0.0) ],
    ); 
    vert

    vk.set_swapchain(surface, &window);
    let images = vk.images.clone().unwrap();
    let render_pass = vk.get_render_pass();
    let framebuffers = vk.get_framebuffers(&render_pass);
    let pipeline = vk.get_pipeline(vs.clone(), fs.clone(), render_pass.clone(), viewport.clone());
    let mut command_buffers = vk.get_command_buffers(&pipeline, &framebuffers, &vertex_buffer);

    let mut window_resized = false; 
    let mut recreate_swapchain = false;

    let frames_in_flight = images.len();
    let mut fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];
    let mut previous_fence_i = 0;


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
                let then = std::time::Instant::now();
                
                if window_resized || recreate_swapchain {
                    recreate_swapchain = false;
                    let new_dim = window.inner_size();

                    let (new_swpchain, new_imgs) = vk.swapchain.clone().unwrap()
                        .recreate(vulkano::swapchain::SwapchainCreateInfo {
                            image_extent: new_dim.into(),
                            ..vk.swapchain.clone().unwrap().create_info()
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
                }

                let (image_i, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(vk.swapchain.clone().unwrap(), None)
                        .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                if suboptimal {
                    recreate_swapchain = true;
                }

                if let Some(image_fence) = &fences[image_i as usize] {
                    image_fence.wait(None).unwrap();
                }

                let previous_future = match fences[previous_fence_i as usize].clone() {
                    None => {
                        let mut now = sync::now(vk.device.clone());
                        now.cleanup_finished();
                        now.boxed()
                    }
                    Some(fence) => fence.boxed(),
                };

                let future = previous_future
                    .join(acquire_future)
                    .then_execute(vk.queue.clone(), command_buffers[image_i as usize].clone())
                    .unwrap()
                    .then_swapchain_present(
                        vk.queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(vk.swapchain.clone().unwrap(), image_i),
                    )
                    .then_signal_fence_and_flush();

                fences[image_i as usize] = match future.map_err(Validated::unwrap) {
                    Ok(value) => Some(Arc::new(value)),
                    Err(VulkanError::OutOfDate) => {
                        recreate_swapchain = true;
                        None
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        None
                    }
                };

                previous_fence_i = image_i;

                println!("MAIN: vk_present @ MainEventsCleared cleared within {:?}", then.elapsed());
            },

            _ => () 
        }
    });
}
