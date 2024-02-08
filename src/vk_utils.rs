use vulkano::VulkanLibrary;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::device::QueueFlags;
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo, Queue};

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

use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;

pub struct Vk {
    pub library: Arc<VulkanLibrary>,
    pub device: Arc<Device>, 
    pub queue: Arc<Queue>,
    pub instance: Arc<Instance>,
    pub memory_allocator: Arc<GenericMemoryAllocator<FreeListAllocator>>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,

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
            physical_device,
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

        // commands

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo::default(),
        ));

        let descriptor_set_allocator =
            Arc::new(StandardDescriptorSetAllocator::new(device.clone(), Default::default()));

        Self {
            library,
            device,
            queue,
            instance,
            memory_allocator,
            command_buffer_allocator,
            descriptor_set_allocator,
            resolution: [1024.0, 1024.0],
        }
    }

    pub fn builder(&self) 
        -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer<Arc<StandardCommandBufferAllocator>>, Arc<StandardCommandBufferAllocator>>
    {
        AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap()
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


