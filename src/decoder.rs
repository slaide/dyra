#[cfg(target_os="windows")]
use winapi::{
    um::{
        libloaderapi::{
            GetModuleHandleA
        },
        winnt::{
            LPCSTR,
        },
        winuser::{
            WNDCLASSA,
            DefWindowProcA,
            RegisterClassA,
            MSG,
            WM_CLOSE,
            PeekMessageA,
            PM_REMOVE,
            TranslateMessage,
            DispatchMessageA,
            CreateWindowExA,
            WS_OVERLAPPEDWINDOW,
            DestroyWindow,
            ShowWindow,
            SW_SHOWDEFAULT,
            UpdateWindow,
        }
    },
    shared::{
        windef::{
            HWND
        },
        minwindef::{
            HINSTANCE,
            LPARAM,
            WPARAM,
            LRESULT,
        }
    }
};

#[cfg(target_os="linux")]
use xcb::{
    ffi::*,
};

use ash::{
    vk,
    vk::{
        AllocationCallbacks,
        PhysicalDevice,
    },
    prelude::VkResult,
    Entry,
    version::{
        EntryV1_0,
        //EntryV1_1,
        //EntryV1_2,
        InstanceV1_0,
        //InstanceV1_1,
        //InstanceV1_2,
        DeviceV1_0,
        //DeviceV1_1,
        //DeviceV1_2,
    },
    Instance,
    Device,
    extensions,
};

pub struct Vertex{
    //space coordinates
    pub x:f32,
    pub y:f32,
    pub z:f32,
    pub w:f32,
    //teture coordinates
    pub u:f32,
    pub v:f32,
}
impl Vertex{
    pub fn new(
        x:f32,
        y:f32,
        z:f32,
        w:f32,
        u:f32,
        v:f32,
    )->Self{
        Self{
            x,
            y,
            z,
            w,
            u,
            v,
        }
    }
}
pub struct VertexIndices{
    a:u16,
    b:u16,
    c:u16,
}
impl VertexIndices{
    pub fn new(a:u16,b:u16,c:u16)->Self{
        Self{
            a,
            b,
            c,
        }
    }
}

#[derive(Debug,Clone)]
pub struct Mesh{
    pub vertices:IntegratedBuffer,
    pub vertex_indices:IntegratedBuffer,
}

#[derive(Debug,Clone,Copy)]
pub struct IntegratedBuffer{
    pub buffer_size:u64,
    pub item_count:u64,
    pub buffer:vk::Buffer,
    pub memory:vk::DeviceMemory,
}

#[derive(Debug,Clone,Copy)]
pub struct Image{
    pub width:u32,
    pub height:u32,
    pub format:vk::Format,
    pub memory:vk::DeviceMemory,
    pub image:vk::Image,
    pub image_view:vk::ImageView,
}
pub struct Decoder{
    pub allocation_callbacks:Option<vk::AllocationCallbacks>,

    pub device:Device,

    pub device_memory_properties:vk::PhysicalDeviceMemoryProperties,

    pub staging_buffer:IntegratedBuffer,
    pub staging_buffer_in_use_size:u64,

    pub meshes:std::collections::HashMap<&'static str,Mesh>,

    pub textures:std::collections::HashMap<&'static str,Image>,
}
impl Decoder{
    pub fn get_allocation_callbacks(&self)->Option<&vk::AllocationCallbacks>{
        self.allocation_callbacks.as_ref()
    }

    pub fn get_quad(&mut self,command_buffer:vk::CommandBuffer)->Mesh{
        if let Some(quad)=self.meshes.get("quad"){
            return quad.clone();
        }
        
        let quad_vertices=vec![
            Vertex::new(
                -0.7, -0.7, 0.0, 1.0,
                0.0, 0.0,
            ),
            Vertex::new(
                -0.7, 0.7, 0.0, 1.0,
                0.0, 1.0,
            ),
            Vertex::new(
                0.7, -0.7, 0.0, 1.0,
                1.0, 0.0,
            ),

            Vertex::new(
                -0.7, 0.7, 0.0, 1.0,
                0.0, 1.0,
            ),
            Vertex::new(
                0.7, 0.7, 0.0, 1.0,
                1.0, 1.0,
            ),
            Vertex::new(
                0.7, -0.7, 0.0, 1.0,
                1.0, 0.0,
            ),
        ];

        let quad_vertex_indices=vec![
            VertexIndices::new(0,1,2),
            VertexIndices::new(3,4,5),
        ];

        let mesh=self.upload_vertex_data("quad",&quad_vertices,&quad_vertex_indices,command_buffer);
        mesh.clone()
    }

    pub fn upload_vertex_data(&mut self,name:&'static str, vertices:&Vec<Vertex>,vertex_indices:&Vec<VertexIndices>,command_buffer:vk::CommandBuffer)->&Mesh{
        let (vertices_size,vertices_buffer,vertices_memory)={
            let size=(vertices.len() * std::mem::size_of::<Vertex>()) as u64;
            let buffer_create_info=vk::BufferCreateInfo{
                size:size,
                usage:vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                sharing_mode:vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let buffer=unsafe{
                self.device.create_buffer(&buffer_create_info,self.get_allocation_callbacks())
            }.unwrap();

            let mut memory=vk::DeviceMemory::null();

            let buffer_memory_requirements=unsafe{
                self.device.get_buffer_memory_requirements(buffer)
            };

            for memory_type_index in 0..self.device_memory_properties.memory_type_count{
                if (buffer_memory_requirements.memory_type_bits & (1<<memory_type_index))>0 
                && self.device_memory_properties.memory_types[memory_type_index as usize].property_flags.intersects(vk::MemoryPropertyFlags::DEVICE_LOCAL){
                    //allocate
                    let memory_allocate_info=vk::MemoryAllocateInfo{
                        allocation_size:buffer_memory_requirements.size,
                        memory_type_index,
                        ..Default::default()
                    };
                    memory=unsafe{
                        self.device.allocate_memory(&memory_allocate_info,self.get_allocation_callbacks())
                    }.unwrap();

                    //bind
                    let buffer_memory_offset=0;
                    unsafe{
                        self.device.bind_buffer_memory(buffer,memory,buffer_memory_offset)
                    }.unwrap();

                    let offset=self.staging_buffer_in_use_size;
                    self.staging_buffer_in_use_size+=buffer_memory_requirements.size;

                    //map staging (!)
                    let memory_pointer=unsafe{
                        self.device.map_memory(self.staging_buffer.memory,offset,size,vk::MemoryMapFlags::empty())
                    }.unwrap();

                    //memcpy
                    unsafe{
                        libc::memcpy(memory_pointer,vertices.as_ptr() as *const libc::c_void,size as usize);
                    }

                    //flush
                    let flush_range=vk::MappedMemoryRange{
                        memory:self.staging_buffer.memory,
                        offset,
                        //size,
                        size:vk::WHOLE_SIZE,
                        ..Default::default()
                    };
                    unsafe{
                        self.device.flush_mapped_memory_ranges(&[flush_range])
                    }.unwrap();

                    //unmap
                    unsafe{
                        self.device.unmap_memory(self.staging_buffer.memory);
                    }

                    unsafe{
                        self.device.cmd_copy_buffer(command_buffer,self.staging_buffer.buffer,buffer,&[
                            vk::BufferCopy{
                                src_offset:offset,
                                dst_offset:0,
                                size:size,
                            }
                        ]);
                    };

                    break;
                }
            }
            if memory==vk::DeviceMemory::null(){
                panic!("staging buffer has no memory")
            }

            (size,buffer,memory)
        };

        let (vertex_indices_size,vertex_indices_buffer,vertex_indices_memory)={
            let size=(vertex_indices.len() * std::mem::size_of::<VertexIndices>()) as u64;
            let buffer_create_info=vk::BufferCreateInfo{
                size:size,
                usage:vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                sharing_mode:vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let buffer=unsafe{
                self.device.create_buffer(&buffer_create_info,self.get_allocation_callbacks())
            }.unwrap();

            let mut memory=vk::DeviceMemory::null();

            let buffer_memory_requirements=unsafe{
                self.device.get_buffer_memory_requirements(buffer)
            };

            for memory_type_index in 0..self.device_memory_properties.memory_type_count{
                if (buffer_memory_requirements.memory_type_bits & (1<<memory_type_index))>0 
                && self.device_memory_properties.memory_types[memory_type_index as usize].property_flags.intersects(vk::MemoryPropertyFlags::DEVICE_LOCAL){
                    //allocate
                    let memory_allocate_info=vk::MemoryAllocateInfo{
                        allocation_size:buffer_memory_requirements.size,
                        memory_type_index,
                        ..Default::default()
                    };
                    memory=unsafe{
                        self.device.allocate_memory(&memory_allocate_info,self.get_allocation_callbacks())
                    }.unwrap();

                    //bind
                    let buffer_memory_offset=0;
                    unsafe{
                        self.device.bind_buffer_memory(buffer,memory,buffer_memory_offset)
                    }.unwrap();

                    let offset=self.staging_buffer_in_use_size;
                    self.staging_buffer_in_use_size+=buffer_memory_requirements.size;

                    //map staging (!)
                    let memory_pointer=unsafe{
                        self.device.map_memory(self.staging_buffer.memory,offset,size,vk::MemoryMapFlags::empty())
                    }.unwrap();

                    //memcpy
                    unsafe{
                        libc::memcpy(memory_pointer,vertex_indices.as_ptr() as *const libc::c_void,size as usize);
                    }

                    //flush
                    let flush_range=vk::MappedMemoryRange{
                        memory:self.staging_buffer.memory,
                        offset,
                        //size,
                        size:vk::WHOLE_SIZE,
                        ..Default::default()
                    };
                    unsafe{
                        self.device.flush_mapped_memory_ranges(&[flush_range])
                    }.unwrap();

                    //unmap
                    unsafe{
                        self.device.unmap_memory(self.staging_buffer.memory);
                    }

                    unsafe{
                        self.device.cmd_copy_buffer(command_buffer,self.staging_buffer.buffer,buffer,&[
                            vk::BufferCopy{
                                src_offset:offset,
                                dst_offset:0,
                                size:size,
                            }
                        ]);
                    };

                    break;
                }
            }
            if memory==vk::DeviceMemory::null(){
                panic!("staging buffer has no memory")
            }

            (size,buffer,memory)
        };

        let buffer_memory_barriers = vec![
            vk::BufferMemoryBarrier{
                src_access_mask:vk::AccessFlags::MEMORY_WRITE,
                dst_access_mask:vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                buffer:vertices_buffer,
                offset: 0,
                size:vertices_size,
                ..Default::default()
            },
            vk::BufferMemoryBarrier{
                src_access_mask:vk::AccessFlags::MEMORY_WRITE,
                dst_access_mask:vk::AccessFlags::VERTEX_ATTRIBUTE_READ,
                src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                buffer:vertex_indices_buffer,
                offset: 0,
                size:vertex_indices_size,
                ..Default::default()
            },
        ];

        unsafe{
            self.device.cmd_pipeline_barrier( command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::VERTEX_INPUT, vk::DependencyFlags::empty(), &[], &buffer_memory_barriers[..], &[]);
        }

        self.meshes.insert(name,Mesh{
            vertices:IntegratedBuffer{
                buffer_size:vertices_size,
                item_count:vertices.len() as u64,
                buffer:vertices_buffer,
                memory:vertices_memory,
            },
            vertex_indices:IntegratedBuffer{
                buffer_size:vertex_indices_size,
                item_count:(vertex_indices.len()*3) as u64,
                buffer:vertex_indices_buffer,
                memory:vertex_indices_memory,
            }
        });

        self.meshes.get(name).unwrap()
    }

    /*
    pub fn new_staging(&mut self,size:u64)->IntegratedBuffer{
        let buffer_create_info=vk::BufferCreateInfo{
            size:size,
            usage:vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode:vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer=unsafe{
            self.device.create_buffer(&buffer_create_info,self.allocation_callbacks)
        }.unwrap();

        let mut memory=vk::DeviceMemory::null();

        let buffer_memory_requirements=unsafe{
            self.device.get_buffer_memory_requirements(buffer)
        };

        for memory_type_index in 0..self.device_memory_properties.memory_type_count{
            if (buffer_memory_requirements.memory_type_bits & (1<<memory_type_index))>0 
            && self.device_memory_properties.memory_types[memory_type_index as usize].property_flags.intersects(vk::MemoryPropertyFlags::HOST_VISIBLE){
                //allocate
                let memory_allocate_info=vk::MemoryAllocateInfo{
                    allocation_size:buffer_memory_requirements.size,
                    memory_type_index,
                    ..Default::default()
                };
                memory=unsafe{
                    self.device.allocate_memory(&memory_allocate_info,self.allocation_callbacks)
                }.unwrap();
                //bind
                let memory_offset=0;
                unsafe{
                    self.device.bind_buffer_memory(buffer,memory,memory_offset)
                }.unwrap();

                break;
            }
        }
        if memory==vk::DeviceMemory::null(){
            panic!("staging buffer has no memory")
        }

        IntegratedBuffer{
            size,
            buffer,
            memory,
        }
    }
    */

    pub fn get_texture(&mut self,filename:&'static str,command_buffer:vk::CommandBuffer)->Image{
        //return cached texture if present
        if let Some(texture)=self.textures.get(filename){
            return *texture;
        }

        //read file from disk and decode into b8g8r8a8 format
        let native_image=image::open(filename).unwrap().into_rgba8();
        let width=native_image.width();
        let height=native_image.height();

        //create image vulkan handle
        let image={
            let image_create_info=vk::ImageCreateInfo{
                image_type:vk::ImageType::TYPE_2D,
                format:vk::Format::R8G8B8A8_UNORM,
                extent:vk::Extent3D{
                    width,
                    height,
                    depth:1,
                },
                mip_levels:1,
                array_layers:1,
                samples:vk::SampleCountFlags::TYPE_1,
                tiling:vk::ImageTiling::OPTIMAL,
                usage:vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED, //is copied to, and then sampled from
                sharing_mode:vk::SharingMode::EXCLUSIVE,//is only accessed from queues from the same family at the same time (ownership transfer in between)
                initial_layout:vk::ImageLayout::UNDEFINED,//when ownership is acquired to copy data to this image, the layout is transitioned to a valid value
                ..Default::default()
            };
            unsafe{
                self.device.create_image(&image_create_info,self.get_allocation_callbacks()).unwrap()
            }
        };

        //allocate image memory and upload data into staging buffer
        //then schedule commands to copy image data from staging into image memory
        let mut memory=vk::DeviceMemory::null();
        {
            let image_memory_reqirements=unsafe{
                self.device.get_image_memory_requirements(image)
            };
            if self.staging_buffer.buffer_size<image_memory_reqirements.size{
                panic!("staging buffer not big enough");
            }

            for i in 0..self.device_memory_properties.memory_type_count{
                if (image_memory_reqirements.memory_type_bits & (1<<i))>0 &&
                    self.device_memory_properties.memory_types[i as usize].property_flags
                        .contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                {
                    let memory_allocate_info=vk::MemoryAllocateInfo{
                        allocation_size:image_memory_reqirements.size,
                        memory_type_index:i,
                        ..Default::default()
                    };
                    //allocate image memory
                    unsafe{
                        memory=self.device.allocate_memory(&memory_allocate_info, self.get_allocation_callbacks()).unwrap()
                    }
                    
                    //bind memory to image handle
                    unsafe{
                        self.device.bind_image_memory(image, memory, 0)
                    }.unwrap();

                    let offset=self.staging_buffer_in_use_size;//offset mesh data because staging buffer is used mesh and texture upload, with no synchronization against each other (could do that, somehow?)
                    self.staging_buffer_in_use_size+=image_memory_reqirements.size;

                    //map staging memory
                    let memory_pointer=unsafe{
                        self.device.map_memory(self.staging_buffer.memory, offset, image_memory_reqirements.size, vk::MemoryMapFlags::empty())
                    }.unwrap();

                    //copy image data to staging
                    unsafe{
                        libc::memcpy(memory_pointer,native_image.into_raw().as_mut_ptr() as *mut libc::c_void,image_memory_reqirements.size as usize);
                    }

                    //flush staging and unmap after
                    let flush_range=vk::MappedMemoryRange{
                        memory:self.staging_buffer.memory,
                        offset,
                        size:image_memory_reqirements.size,
                        ..Default::default()
                    };
                    unsafe{
                        self.device.flush_mapped_memory_ranges(&[flush_range]).unwrap();
                        self.device.unmap_memory(self.staging_buffer.memory);
                    }

                    //schedule image data transfer from staging to final
                    
                    let image_subresource_range=vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    };
                    let image_memory_barrier_none_to_transfer = vk::ImageMemoryBarrier{
                        src_access_mask:vk::AccessFlags::empty(),
                        dst_access_mask:vk::AccessFlags::TRANSFER_WRITE,
                        old_layout:vk::ImageLayout::UNDEFINED,
                        new_layout:vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image,
                        subresource_range:image_subresource_range,
                        ..Default::default()
                    };
                    unsafe{
                        //perform buffer layout transition from copy target to vertex data source
                        self.device.cmd_pipeline_barrier(command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[image_memory_barrier_none_to_transfer]);
                    }

                    let buffer_image_copy_info=vk::BufferImageCopy{
                        buffer_offset:offset,
                        buffer_row_length:0,
                        buffer_image_height:0,
                        image_subresource:vk::ImageSubresourceLayers{
                            aspect_mask:vk::ImageAspectFlags::COLOR,
                            mip_level:0,
                            base_array_layer:0,
                            layer_count:1,
                        },
                        image_offset:vk::Offset3D{
                            x:0,
                            y:0,
                            z:0,
                        },
                        image_extent:vk::Extent3D{
                            width,
                            height,
                            depth:1,
                        },
                        ..Default::default()
                    };
                    unsafe{
                        self.device.cmd_copy_buffer_to_image(command_buffer, self.staging_buffer.buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[buffer_image_copy_info]);
                    }
                    
                    let image_subresource_range=vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    };
                    let image_memory_barrier_transfer_to_shader_read = vk::ImageMemoryBarrier{
                        src_access_mask:vk::AccessFlags::TRANSFER_WRITE,
                        dst_access_mask:vk::AccessFlags::SHADER_READ,
                        old_layout:vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        new_layout:vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                        src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
                        image,
                        subresource_range:image_subresource_range,
                        ..Default::default()
                    };
                    unsafe{
                        self.device.cmd_pipeline_barrier( command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[image_memory_barrier_transfer_to_shader_read]);
                    }

                    break;
                }
            }
        };
        if memory==vk::DeviceMemory::null(){
            panic!("no fit memory found!");
        }
        
        //create image view to enable image access
        let image_view={
            let subresource_range=vk::ImageSubresourceRange{
                aspect_mask:vk::ImageAspectFlags::COLOR,
                base_mip_level:0,
                level_count:1,
                base_array_layer:0,
                layer_count:1,
            };
            let image_view_create_info=vk::ImageViewCreateInfo{
                image,
                view_type:vk::ImageViewType::TYPE_2D,
                format:vk::Format::R8G8B8A8_UNORM,
                components:vk::ComponentMapping{
                    r:vk::ComponentSwizzle::IDENTITY,
                    g:vk::ComponentSwizzle::IDENTITY,
                    b:vk::ComponentSwizzle::IDENTITY,
                    a:vk::ComponentSwizzle::IDENTITY,
                },
                subresource_range,
                ..Default::default()
            };
            unsafe{
                self.device.create_image_view(&image_view_create_info,self.get_allocation_callbacks()).unwrap()
            }
        };

        let image=Image{
            width,
            height,
            format:vk::Format::R8G8B8A8_UNORM,
            memory,
            image,
            image_view,
        };

        self.textures.insert(filename,image);

        return image;
    }
}
impl Drop for Decoder{
    fn drop(&mut self){
        for(_name,texture) in self.textures.iter(){
            unsafe{
                self.device.destroy_image_view(texture.image_view,self.get_allocation_callbacks());
                self.device.destroy_image(texture.image,self.get_allocation_callbacks());
                self.device.free_memory(texture.memory,self.get_allocation_callbacks());
            }
        }
        for (_name,mesh) in self.meshes.iter(){
            unsafe{
                self.device.free_memory(mesh.vertices.memory,self.get_allocation_callbacks());
                self.device.destroy_buffer(mesh.vertices.buffer, self.get_allocation_callbacks());

                self.device.free_memory(mesh.vertex_indices.memory,self.get_allocation_callbacks());
                self.device.destroy_buffer(mesh.vertex_indices.buffer, self.get_allocation_callbacks());
            }
        }

        unsafe{
            self.device.free_memory(self.staging_buffer.memory,self.get_allocation_callbacks());
            self.device.destroy_buffer(self.staging_buffer.buffer, self.get_allocation_callbacks());
        }

    }
}