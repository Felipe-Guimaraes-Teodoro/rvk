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
use crate::vk_present::FRAGMENT_PUSH_CONSTANTS;
pub fn run() {
    let event_loop = EventLoop::new();
    let mut vk = Arc::new(Mutex::new(crate::vk_utils::Vk::new(&event_loop)));

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap()); 
    window.set_title("VULKAN");

    let mut pr = Arc::new(Mutex::new(VkPresenter::new(&mut vk.clone().lock().unwrap(), window.clone())));
    let mut frame_id = 0;

    let mut bool_key = [false; 6];

    let pool = threadpool::ThreadPool::new(12);

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { 
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            },

            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input: KeyboardInput { state, virtual_keycode: Some(virtual_keycode), .. }, .. },
                ..
            } => {
                match state {
                    ElementState::Pressed => {
                        if virtual_keycode == VirtualKeyCode::W {
                            bool_key[0] = true;
                        }
                        if virtual_keycode == VirtualKeyCode::A {
                            bool_key[1] = true;
                        }
                        if virtual_keycode == VirtualKeyCode::S {
                            bool_key[2] = true;
                        }
                        if virtual_keycode == VirtualKeyCode::D {
                            bool_key[3] = true;
                        }
                        if virtual_keycode == VirtualKeyCode::I {
                            bool_key[4] = true;
                        }
                        if virtual_keycode == VirtualKeyCode::K {
                            bool_key[5] = true;
                        }
                    },

                    ElementState::Released => {
                        if virtual_keycode == VirtualKeyCode::W {
                            bool_key[0] = false;
                        }
                        if virtual_keycode == VirtualKeyCode::A {
                            bool_key[1] = false;
                        }
                        if virtual_keycode == VirtualKeyCode::S {
                            bool_key[2] = false;
                        }
                        if virtual_keycode == VirtualKeyCode::D {
                            bool_key[3] = false;
                        }
                        if virtual_keycode == VirtualKeyCode::I {
                            bool_key[4] = false;
                        }
                        if virtual_keycode == VirtualKeyCode::K {
                            bool_key[5] = false;
                        }
                    }
                }
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                pr.clone().lock().unwrap().window_resized = true;
            },

            Event::MainEventsCleared => {
                let then = std::time::Instant::now();

                pool.execute(move || {
                let zoom = FRAGMENT_PUSH_CONSTANTS.lock().unwrap().zoom;

                if bool_key[0] {
                    FRAGMENT_PUSH_CONSTANTS.lock().unwrap().cpos[1] += 0.001 * zoom;
                }
                if bool_key[1] {
                    FRAGMENT_PUSH_CONSTANTS.lock().unwrap().cpos[0] += 0.001 * zoom;
                }
                if bool_key[2] {
                    FRAGMENT_PUSH_CONSTANTS.lock().unwrap().cpos[1] -= 0.001 * zoom;
                }
                if bool_key[3] {
                    FRAGMENT_PUSH_CONSTANTS.lock().unwrap().cpos[0] -= 0.001 * zoom;
                }
                if bool_key[4] {
                    FRAGMENT_PUSH_CONSTANTS.lock().unwrap().zoom /= 0.999;
                }
                if bool_key[5] {
                    FRAGMENT_PUSH_CONSTANTS.lock().unwrap().zoom /= 1.01;
                }
                });
                pr.clone().lock().unwrap().if_recreate_swapchain(window.clone(), &mut vk.clone().lock().unwrap());
                pr.clone().lock().unwrap().update(&mut vk.clone().lock().unwrap());
                *crate::vk_present::FRAGMENT_PUSH_CONSTANTS.lock().unwrap().time += 0.001;
                pr.clone().lock().unwrap().present(&mut vk.clone().lock().unwrap());

                println!("MAIN: vk_present @ MainEventsCleared cleared within {:?}", then.elapsed());
                frame_id += 1;
            },

            _ => () 
        }
    });
}
