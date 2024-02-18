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


use crate::vk_pipeline::vert;
use crate::vk_present::VkPresenter;
pub fn run() {
    let event_loop = EventLoop::new();
    let mut vk = crate::vk_utils::Vk::new(&event_loop);

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap()); 
    window.set_title("VULKAN");

    let mut pr = VkPresenter::new(&mut vk, window.clone());

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
                pr.window_resized = true;
            },

            Event::MainEventsCleared => {
                let then = std::time::Instant::now();
                
                if pr.window_resized || pr.recreate_swapchain {
                    pr.recreate_swapchain = false;
                    let new_dim = window.inner_size();

                    let (new_swpchain, new_imgs) = vk.swapchain.clone().unwrap()
                        .recreate(vulkano::swapchain::SwapchainCreateInfo {
                            image_extent: new_dim.into(),
                            ..vk.swapchain.clone().unwrap().create_info()
                        })
                    .expect("failed to recreate swpchain");

                    vk.swapchain = Some(new_swpchain);
                    vk.images = Some(new_imgs);
                    pr.framebuffers = vk.get_framebuffers(&pr.render_pass);

                    if pr.window_resized {
                        pr.window_resized = false;

                        pr.viewport.extent = new_dim.into();
                        (pr.pipeline, pr.layout) = vk.get_pipeline(
                            pr.shader_mods[0].clone(), 
                            pr.shader_mods[1].clone(), 
                            pr.render_pass.clone(), 
                            pr.viewport.clone()
                        );
                    }
                }

                pr.command_buffers = vk.get_command_buffers(
                    &pr.pipeline.clone(), 
                    &pr.framebuffers, 
                    &pr.vert_buffers[0], 
                    pr.layout.clone(),
                    crate::vk_present::fs::PushConstantData {time: 0.0}
                );

                let (image_i, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(vk.swapchain.clone().unwrap(), None)
                        .map_err(Validated::unwrap)
                    {
                        Ok(r) => r,
                        Err(VulkanError::OutOfDate) => {
                            pr.recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("failed to acquire next image: {e}"),
                    };

                if suboptimal {
                    pr.recreate_swapchain = true;
                }

                if let Some(image_fence) = &pr.fences[image_i as usize] {
                    image_fence.wait(None).unwrap();
                }

                let previous_future = match pr.fences[pr.previous_fence_i as usize].clone() {
                    None => {
                        let mut now = sync::now(vk.device.clone());
                        now.cleanup_finished();
                        now.boxed()
                    }
                    Some(fence) => fence.boxed(),
                };

                let future = previous_future
                    .join(acquire_future)
                    .then_execute(vk.queue.clone(), pr.command_buffers[image_i as usize].clone())
                    .unwrap()
                    .then_swapchain_present(
                        vk.queue.clone(),
                        SwapchainPresentInfo::swapchain_image_index(vk.swapchain.clone().unwrap(), image_i),
                    )
                    .then_signal_fence_and_flush();

                pr.fences[image_i as usize] = match future.map_err(Validated::unwrap) {
                    Ok(value) => Some(Arc::new(value)),
                    Err(VulkanError::OutOfDate) => {
                        pr.recreate_swapchain = true;
                        None
                    }
                    Err(e) => {
                        println!("failed to flush future: {e}");
                        None
                    }
                };

                pr.previous_fence_i = image_i;
                
                println!("MAIN: vk_present @ MainEventsCleared cleared within {:?}", then.elapsed());
            },

            _ => () 
        }
    });
}
