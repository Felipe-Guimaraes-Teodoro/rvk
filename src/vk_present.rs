use std::sync::Arc;

use winit::event_loop::EventLoop;
use winit::event::*;
use winit::window::WindowBuilder;

use vulkano::swapchain::Surface;

use vulkano::swapchain;
use vulkano::{Validated, VulkanError};
use vulkano::swapchain::SwapchainPresentInfo;

use vulkano::sync::{self, GpuFuture};
use vulkano::sync::future::FenceSignalFuture;

use vulkano::buffer::Subbuffer;
use vulkano::command_buffer::{
    PrimaryAutoCommandBuffer, 
    allocator::StandardCommandBufferAllocator
};

use crate::vk_pipeline::FVertex3d;
use crate::vk_utils::Vk;

pub struct VkPresenter {
    viewport: vulkano::pipeline::graphics::viewport::Viewport,
    shader_mods: Vec<Arc<vulkano::shader::ShaderModule>>,
    vert_buffers: Vec<Subbuffer<[FVertex3d]>>,
    
    vk: Vk,

    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer<StandardCommandBufferAllocator>>>,


    window_resized: bool, recreate_swapchain: bool, 
    frames_in_flight: usize,
    previous_fence_i: u32, 
}

impl VkPresenter {
    pub fn new() -> Self {
        todo!()
    }

    pub fn present(&mut self) {}

    pub fn on_window_resized(&mut self) {}
}
