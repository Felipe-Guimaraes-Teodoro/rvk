use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::QueueFlags;
use vulkano::device::{Device, physical::PhysicalDevice, DeviceCreateInfo, QueueCreateInfo, Queue};

use vulkano::memory::allocator::{
    StandardMemoryAllocator, FreeListAllocator, GenericMemoryAllocator
};

use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract};
use vulkano::sync::{self, GpuFuture};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::swapchain::Surface;
use vulkano::image::ImageUsage;
use vulkano::swapchain::{Swapchain, SwapchainCreateInfo};

use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

// todo: make vk global
// issues: vk requires event loopto be initialized; so either make it so tgat vk doesnt need event
// loop or make event_loop global aswell, which also comes with it s own problems...
//

// pub static VK: Lazy<Arc<Mutex<Vk>>> = Lazy::new( || {
//     Vk::new().into();
// });

pub struct VkMemAllocators {
    pub memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    pub command_buffer_allocator: StandardCommandBufferAllocator,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

pub struct Vk {
    pub library: Arc<VulkanLibrary>,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>, 
    pub queue: Arc<Queue>,
    pub instance: Arc<Instance>,

    pub mem_allocators: Arc<VkMemAllocators>,

    pub swapchain: Option<Arc<vulkano::swapchain::Swapchain>>,
    pub images: Option<Vec<Arc<vulkano::image::Image>>>,

    pub resolution: [f32; 2],
}

impl Vk {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        // Initialization // 
        let library = VulkanLibrary::new().expect("no local Vulkan library/DLL");
        let req_extensions = Surface::required_extensions(&event_loop);
        let instance = Instance::new(
            library.clone(), 
            InstanceCreateInfo {
                enabled_extensions: req_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        let physical_device = instance
            .enumerate_physical_devices()
            .expect("could not enumerate devices")
            .next()
            .expect("no devices available");

        let queue_family_index = physical_device
            .queue_family_properties()
            .iter()
            .enumerate()
            .position(|(_queue_family_index, queue_family_properties)| {
                queue_family_properties.queue_flags.contains(QueueFlags::GRAPHICS)
            })
            .expect("couldn't find a graphical queue family") as u32;

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                // here we pass the desired queue family to use by index
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: vulkano::device::DeviceExtensions {
                    khr_swapchain: true,
                    ..vulkano::device::DeviceExtensions::empty()
                },
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let command_buffer_allocator = StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        );

        let descriptor_set_allocator =
            Arc::new(StandardDescriptorSetAllocator::new(device.clone(), Default::default()));


        let mem_allocators= Arc::new(VkMemAllocators {
            command_buffer_allocator,
            memory_allocator,
            descriptor_set_allocator,
        });

        Self {
            library,
            device,
            physical_device,
            queue,
            instance,
            mem_allocators,
            resolution: [1024.0, 1024.0],

            swapchain: None, // will be initialized later on
            images: None,
        }
    }

    pub fn set_swapchain(&mut self, surface: Arc<Surface>, window: &winit::window::Window) {
        let caps = self.physical_device
            .surface_capabilities(&surface, Default::default())
            .expect("failed to get surface capabilities");
        let dimensions = window.inner_size();
        let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
        let image_format = Some(
            self.physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        )
        .unwrap();

        let (mut swapchain, images) = Swapchain::new(
            self.device.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: caps.min_image_count + 1,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT, 
                present_mode: vulkano::swapchain::PresentMode::Mailbox,
                composite_alpha,
                ..Default::default()
            },
        )
        .unwrap();

        self.swapchain = Some(swapchain.clone());
        self.images = Some(images.clone());
    }

    pub fn sync(&self, command: Arc<impl PrimaryCommandBufferAbstract + 'static>) {
        let future = sync::now(self.device.clone())
            .then_execute(self.queue.clone(), command.clone())
            .unwrap()
            .then_signal_fence_and_flush()
            .unwrap();

        future.wait(None).unwrap();
    }
}


