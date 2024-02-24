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

pub static EVENT_LOOP: Lazy<Arc<Mutex<EventLoop<()>>>> = Lazy::new(|| {
       
});

use crate::vk_pipeline::vert;
use crate::vk_present::{VkPresenter, VkView};
use crate::vk_present::{FRAGMENT_PUSH_CONSTANTS, WINDOW_RESIZED};
pub fn run() {
    let event_loop = EventLoop::new();
    let mut vk = Arc::new(Mutex::new(crate::vk_utils::Vk::new(&event_loop)));

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap()); 
    window.set_title("VULKAN");

    let mut view = Arc::new(Mutex::new(VkView::new(&mut vk.clone().lock().unwrap(), window.clone())));
    let mut presenter = VkPresenter::new(&mut vk.clone().lock().unwrap());
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
                *WINDOW_RESIZED.lock().unwrap() = true;
                // view.clone().lock().unwrap().window_resized = true;
            },

            Event::MainEventsCleared => {
                let then = std::time::Instant::now();

                let view_c = view.clone();
                let vk_c = vk.clone();
                let window_c = window.clone();

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

                view_c.clone().lock().unwrap().if_recreate_swapchain(window_c.clone(), &mut vk_c.clone().lock().unwrap());
                view_c.clone().lock().unwrap().update(&mut vk_c.clone().lock().unwrap());
                *crate::vk_present::FRAGMENT_PUSH_CONSTANTS.lock().unwrap().time += 0.001;

                presenter.present(&mut vk.clone().lock().unwrap(), &view.clone().lock().unwrap());
                // pr.clone().lock().unwrap().present(&mut vk.clone().lock().unwrap());

                println!("MAIN: vk_present @ MainEventsCleared cleared within {:?}", then.elapsed());
                frame_id += 1;

            },

            _ => () 
        }
    });
}
