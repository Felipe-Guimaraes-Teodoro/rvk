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
    let mut frame_id = 0;

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

                pr.if_recreate_swapchain(window.clone(), &mut vk);
                pr.update(&mut vk);
                *crate::vk_present::FRAGMENT_PUSH_CONSTANTS.lock().unwrap().time += 0.001;
                pr.present(&mut vk);

                println!("MAIN: vk_present @ MainEventsCleared cleared within {:?}", then.elapsed());
                frame_id += 1;
            },

            _ => () 
        }
    });
}
