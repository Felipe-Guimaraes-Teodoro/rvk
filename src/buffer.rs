use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter};

use vulkano::image::{Image, ImageCreateInfo, ImageUsage};
use vulkano::format::Format;
use vulkano::command_buffer::ClearColorImageInfo;
use vulkano::format::ClearColorValue;

use crate::vk_utils;


impl vk_utils::Vk {
    pub fn buf_iter
        <T: Sync + Send + Sized + ExactSizeIterator>
        (&self, iter: T) -> vulkano::buffer::Subbuffer<[<T as Iterator>::Item]> 
        where 
            <T as Iterator>::Item: bytemuck::Pod, 
            <T as Iterator>::Item: Sync, 
            <T as Iterator>::Item: Send  
    {
        Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST 
                    | MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            iter,
        ).expect("failed to create buffer")
    }

    pub fn vertex_buffer(&self, vec: Vec<crate::vk_pipeline::FVertex3d>)
    -> vulkano::buffer::Subbuffer<[crate::vk_pipeline::FVertex3d]> {
        Buffer::from_iter(
            self.memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec,
        )
        .expect("failed to create buffer")
    }

    pub fn image(&self, dim: [u32; 3]) -> std::sync::Arc<Image>  {
        Image::new(
            self.memory_allocator.clone(),
            ImageCreateInfo {
                image_type: vulkano::image::ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: dim,
                usage: ImageUsage::TRANSFER_DST 
                    | ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::TRANSFER_SRC
                    | ImageUsage::STORAGE,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
                ..Default::default()
            },
        )
        .unwrap()
    }
}
