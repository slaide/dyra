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

use std::str::FromStr;

#[derive(Debug,Clone)]
#[repr(C)]
pub struct VertexCoordinates{
    pub x:f32,
    pub y:f32,
    pub z:f32,
    pub w:f32,
}
#[derive(Debug,Clone)]
#[repr(C)]
pub struct VertexTextureCoordinates{
    pub u:f32,
    pub v:f32,
    pub w:f32,
}
#[derive(Debug,Clone)]
#[repr(C)]
pub struct TexturedVertex{
    pub vertex_coordinates:VertexCoordinates,
    pub vertex_texture_coordinates:VertexTextureCoordinates,
}
#[derive(Debug,Clone)]
#[repr(C)]
pub struct Vertex{
    pub vertex_coordinates:VertexCoordinates,
}

//polygon face with indices of vertices in related vertex list
#[derive(Debug,Clone,Copy)]
#[repr(C)]
pub struct Face{
    a:u16,
    b:u16,
    c:u16,
}

#[derive(Debug,Clone)]
#[repr(C)]
pub enum VertexData{
    Textured(Vec<TexturedVertex>),
    Plain(Vec<Vertex>)
}
impl VertexData{
    pub fn len(&self)->usize{
        match self{
            VertexData::Textured(v)=>v.len(),
            VertexData::Plain(v)=>v.len(),
        }
    }
    pub fn mem_size(&self)->usize{
        match self{
            VertexData::Textured(v)=>v.len()*std::mem::size_of::<TexturedVertex>(),
            VertexData::Plain(v)=>v.len()*std::mem::size_of::<Vertex>(),
        }
    }
    pub fn as_ptr(&self)->*const libc::c_void{
        match self{
            VertexData::Textured(v)=>v.as_ptr() as *const libc::c_void,
            VertexData::Plain(v)=>v.as_ptr() as *const libc::c_void,
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
pub struct StagingBuffer{
    pub buffer:IntegratedBuffer,
    pub buffer_in_use_size:u64,
}
impl StagingBuffer{
    pub fn size_left(&self)->u64{
        self.buffer.buffer_size-self.buffer_in_use_size
    }
}

#[derive(Debug,Clone,Copy)]
pub struct Image{
    pub width:u32,
    pub height:u32,
    pub format:vk::Format,
    pub memory:vk::DeviceMemory,
    pub image:vk::Image,
    pub image_view:vk::ImageView,
    pub layout:vk::ImageLayout,
}
pub struct Decoder{
    pub vulkan:std::sync::Arc<crate::VulkanBase>,

    pub device_memory_properties:vk::PhysicalDeviceMemoryProperties,

    pub transfer_queue:crate::Queue,

    pub staging_buffers:Vec<StagingBuffer>,

    pub meshes:std::collections::HashMap<&'static str,std::sync::Arc<Mesh>>,

    pub textures:std::collections::HashMap<String,std::sync::Arc<Image>>,
}
impl Decoder{
    pub fn new(vulkan:&std::sync::Arc<crate::VulkanBase>,transfer_queue:crate::Queue)->Self{
        let device_memory_properties=unsafe{
            vulkan.instance.get_physical_device_memory_properties(vulkan.physical_device)
        };

        let mut transfer_queue=transfer_queue;
        let _=transfer_queue.create_command_buffers(8);
        
        Self{
            vulkan:vulkan.clone(),

            device_memory_properties,

            transfer_queue,

            staging_buffers:Vec::new(),

            meshes:std::collections::HashMap::new(),
            textures:std::collections::HashMap::new(),
        }
    }

    pub fn get_mesh(&mut self,name:&'static str)->std::sync::Arc<Mesh>{
        if let Some(mesh)=self.meshes.get(name){
            return mesh.clone();
        }

        let (vertices,vertex_indices)={
            use std::io::BufRead;
            let file=std::fs::File::open(name).unwrap();
            let file_content=std::io::BufReader::new(file);

            let mut lines=file_content.lines();
            let attribute=lines.next().unwrap();
            assert!(attribute.unwrap()=="#version");

            let version=lines.next().unwrap().unwrap().parse::<u32>().unwrap();
            println!("parsing mesh of version {}",version);

            assert!(lines.next().unwrap().unwrap()=="#settings");

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="vertex_coordinate_count");
            let vertex_coordinate_count=line.next().unwrap().parse::<u32>().unwrap();

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="vertex_texture_coordinates");
            let vertex_texture_coordinates=line.next().unwrap()=="true";

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="vertex_texture_coordinate_count");
            let vertex_texture_coordinate_count=line.next().unwrap().parse::<u32>().unwrap();

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="vertex_count");
            let vertex_count=line.next().unwrap().parse::<u32>().unwrap();

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="face_count");
            let face_count=line.next().unwrap().parse::<u32>().unwrap();

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="vertex_data_interleaved_input");
            let vertex_data_interleaved_input=line.next().unwrap()=="true";

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="index_start");
            let index_start=line.next().unwrap().parse::<u16>().unwrap();

            assert!(lines.next().unwrap().unwrap()=="#defaults");

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="v.z");
            let default_v_z=line.next().unwrap().parse::<f32>().unwrap();

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="v.w");
            let default_v_w=line.next().unwrap().parse::<f32>().unwrap();

            let line=lines.next().unwrap().unwrap();
            let mut line=line.split('=');
            let attribute=line.next().unwrap();
            assert!(attribute=="vt.w");
            let default_vt_w=line.next().unwrap().parse::<f32>().unwrap();
            
            assert!(lines.next().unwrap().unwrap()=="#vertexdata");

            let vertex_data_line_count=if vertex_texture_coordinates{
                vertex_count*2
            }else{
                vertex_count
            };

            let mut vertices={
                if vertex_texture_coordinates{
                    let vertex_data_count=(vertex_count*2) as usize;
                    let mut vertices=Vec::with_capacity(vertex_data_count);
                    use itertools::Itertools;
                    if vertex_data_interleaved_input{
                        vertices.extend(lines.by_ref().take(vertex_data_count).tuples().map(|(line1,line2)|{
                            let vertex_data=line1.unwrap();
                            let vertex_texture_data=line2.unwrap();

                            let mut vertex_data=vertex_data.split(' ');
                            let mut vertex_texture_data=vertex_texture_data.split(' ');

                            assert!(vertex_data.next().unwrap()=="v");
                            let vertex_coordinates=VertexCoordinates{
                                x:vertex_data.next().unwrap().parse::<f32>().unwrap(),
                                y:vertex_data.next().unwrap().parse::<f32>().unwrap(),
                                z:match vertex_coordinate_count{
                                    2=>default_v_z,
                                    3=>vertex_data.next().unwrap().parse::<f32>().unwrap(),
                                    4=>vertex_data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                },
                                w:match vertex_coordinate_count{
                                    3=>default_v_w,
                                    4=>vertex_data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                }
                            };
                            assert!(vertex_data.next().is_none());

                            assert!(vertex_texture_data.next().unwrap()=="vt");
                            let vertex_texture_coordinates=VertexTextureCoordinates{
                                u:vertex_texture_data.next().unwrap().parse::<f32>().unwrap(),
                                v:vertex_texture_data.next().unwrap().parse::<f32>().unwrap(),
                                w:match vertex_texture_coordinate_count{
                                    2=>default_vt_w,
                                    3=>vertex_texture_data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                }
                            };
                            let vertex=TexturedVertex{
                                vertex_coordinates,
                                vertex_texture_coordinates
                            };
                            assert!(vertex_texture_data.next().is_none());
                            vertex
                        }));
                    }else{
                        let mut vertex_coordinates=Vec::with_capacity(vertex_count as usize);
                        vertex_coordinates.extend(lines.by_ref().take(vertex_count as usize).map(|line|{
                            let line=line.unwrap();
                            let mut data=line.split(' ');
                            assert!(data.next().unwrap()=="v");
                            let vertex_coordinates=VertexCoordinates{
                                x:data.next().unwrap().parse::<f32>().unwrap(),
                                y:data.next().unwrap().parse::<f32>().unwrap(),
                                z:match vertex_coordinate_count{
                                    2=>default_v_z,
                                    3=>data.next().unwrap().parse::<f32>().unwrap(),
                                    4=>data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                },
                                w:match vertex_coordinate_count{
                                    3=>default_v_w,
                                    4=>data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                }
                            };
                            assert!(data.next().is_none());

                            vertex_coordinates
                        }));
                        let mut vertex_texture_coordinates=Vec::with_capacity(vertex_count as usize);
                        vertex_texture_coordinates.extend(lines.by_ref().take(vertex_count as usize).map(|line|{
                            let line=line.unwrap();
                            let mut data=line.split(' ');
                            assert!(data.next().unwrap()=="vt");
                            VertexTextureCoordinates{
                                u:data.next().unwrap().parse::<f32>().unwrap(),
                                v:data.next().unwrap().parse::<f32>().unwrap(),
                                w:match vertex_texture_coordinate_count{
                                    2=>default_vt_w,
                                    3=>data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                },
                            }
                        }));
                        vertices.extend(vertex_coordinates.iter().zip(vertex_texture_coordinates.iter()).map(|(vertex_coordinates,vertex_texture_coordinates)|{
                            TexturedVertex{
                                vertex_coordinates:vertex_coordinates.clone(),
                                vertex_texture_coordinates:vertex_texture_coordinates.clone()
                            }
                        }));
                    }
                    VertexData::Textured(vertices)
                }else{
                    let vertex_data_count=vertex_count as usize;
                    let mut vertices=Vec::with_capacity(vertex_data_count);
                    vertices.extend(lines.by_ref().take(vertex_data_count).map(|line|{
                        let data=line.unwrap();
                        let mut data=data.split(' ');
                        assert!(data.next().unwrap()=="v");
                        let vertex=Vertex{
                            vertex_coordinates:VertexCoordinates{
                                x:data.next().unwrap().parse::<f32>().unwrap(),
                                y:data.next().unwrap().parse::<f32>().unwrap(),
                                z:match vertex_coordinate_count{
                                    2=>default_v_z,
                                    3=>data.next().unwrap().parse::<f32>().unwrap(),
                                    4=>data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                },
                                w:match vertex_coordinate_count{
                                    3=>default_v_w,
                                    4=>data.next().unwrap().parse::<f32>().unwrap(),
                                    _=>unreachable!(),
                                }
                            }
                        };
                        assert!(data.next().is_none());
                        vertex
                    }));
                    VertexData::Plain(vertices)
                }
            };
            
            assert!(lines.next().unwrap().unwrap()=="#facedata");

            let mut vertex_indices:Vec::<Face>=Vec::with_capacity(face_count as usize);
            vertex_indices.extend(lines.by_ref().take(face_count as usize).map(|line|{
                let data=line.unwrap();
                let mut data=data.split(' ');
                assert!(data.next().unwrap()=="f");
                let face=Face{
                    a:data.next().unwrap().parse::<u16>().unwrap()-index_start,
                    b:data.next().unwrap().parse::<u16>().unwrap()-index_start,
                    c:data.next().unwrap().parse::<u16>().unwrap()-index_start,
                };
                assert!(data.next().is_none());
                face
            }));
            //println!("{:?}",&vertex_indices);

            (vertices,vertex_indices)
        };

        let (vertices_size,vertices_buffer,vertices_memory)={
            let size=vertices.mem_size() as u64;
            let buffer_create_info=vk::BufferCreateInfo{
                size:size,
                usage:vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                sharing_mode:vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let buffer=unsafe{
                self.vulkan.device.create_buffer(&buffer_create_info,self.vulkan.get_allocation_callbacks())
            }.unwrap();

            let mut memory=vk::DeviceMemory::null();

            let buffer_memory_requirements=unsafe{
                self.vulkan.device.get_buffer_memory_requirements(buffer)
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
                        self.vulkan.device.allocate_memory(&memory_allocate_info,self.vulkan.get_allocation_callbacks())
                    }.unwrap();

                    //bind
                    let buffer_memory_offset=0;
                    unsafe{
                        self.vulkan.device.bind_buffer_memory(buffer,memory,buffer_memory_offset)
                    }.unwrap();

                    let mut staging_buffer=if let Some(sb)=self.staging_buffers.iter_mut().find(|sb|sb.size_left()>=size){
                        sb
                    }else{
                        self.new_staging(size.next_power_of_two());
                        self.staging_buffers.last_mut().unwrap()
                    };
                    let offset=staging_buffer.buffer_in_use_size;
                    staging_buffer.buffer_in_use_size+=buffer_memory_requirements.size;

                    //map staging (!)
                    let memory_pointer=unsafe{
                        self.vulkan.device.map_memory(staging_buffer.buffer.memory,offset,size,vk::MemoryMapFlags::empty())
                    }.unwrap();

                    //memcpy
                    unsafe{
                        libc::memcpy(memory_pointer,vertices.as_ptr(),size as usize);
                    }

                    //flush
                    let flush_range=vk::MappedMemoryRange{
                        memory:staging_buffer.buffer.memory,
                        offset,
                        size:vk::WHOLE_SIZE,
                        ..Default::default()
                    };
                    unsafe{
                        self.vulkan.device.flush_mapped_memory_ranges(&[flush_range])
                    }.unwrap();

                    //unmap
                    unsafe{
                        self.vulkan.device.unmap_memory(staging_buffer.buffer.memory);
                    }

                    let command_buffer=self.transfer_queue.command_buffers[0];

                    let begin_info=vk::CommandBufferBeginInfo{

                        ..Default::default()
                    };
                    
                    let wait_semaphores=vec![];
                    let command_buffers=vec![
                        command_buffer
                    ];
                    let signal_semaphores=vec![];
                    let submit_info=vk::SubmitInfo{
                        wait_semaphore_count:wait_semaphores.len() as u32,
                        p_wait_semaphores:wait_semaphores.as_ptr(),
                        p_wait_dst_stage_mask:&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        command_buffer_count:command_buffers.len() as u32,
                        p_command_buffers:command_buffers.as_ptr(),
                        signal_semaphore_count:signal_semaphores.len() as u32,
                        p_signal_semaphores:signal_semaphores.as_ptr(),
                        ..Default::default()
                    };

                    unsafe{
                        self.vulkan.device.begin_command_buffer(command_buffer,&begin_info).unwrap();
                        self.vulkan.device.cmd_copy_buffer(command_buffer,staging_buffer.buffer.buffer,buffer,&[
                            vk::BufferCopy{
                                src_offset:offset,
                                dst_offset:0,
                                size:size,
                            }
                        ]);
                        self.vulkan.device.end_command_buffer(command_buffer).unwrap();

                        self.vulkan.device.queue_submit(self.transfer_queue.queue,&[submit_info],vk::Fence::null()).unwrap();
                    }

                    break;
                }
            }
            if memory==vk::DeviceMemory::null(){
                panic!("staging buffer has no memory")
            }

            (size,buffer,memory)
        };

        let (vertex_indices_size,vertex_indices_buffer,vertex_indices_memory)={
            let size=(vertex_indices.len() * std::mem::size_of::<Face>()) as u64;
            let buffer_create_info=vk::BufferCreateInfo{
                size:size,
                usage:vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                sharing_mode:vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let buffer=unsafe{
                self.vulkan.device.create_buffer(&buffer_create_info,self.vulkan.get_allocation_callbacks())
            }.unwrap();

            let mut memory=vk::DeviceMemory::null();

            let buffer_memory_requirements=unsafe{
                self.vulkan.device.get_buffer_memory_requirements(buffer)
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
                        self.vulkan.device.allocate_memory(&memory_allocate_info,self.vulkan.get_allocation_callbacks())
                    }.unwrap();

                    //bind
                    let buffer_memory_offset=0;
                    unsafe{
                        self.vulkan.device.bind_buffer_memory(buffer,memory,buffer_memory_offset)
                    }.unwrap();

                    let mut staging_buffer=if let Some(sb)=self.staging_buffers.iter_mut().find(|sb|sb.size_left()>=size){
                        sb
                    }else{
                        self.new_staging(size.next_power_of_two());
                        self.staging_buffers.last_mut().unwrap()
                    };
                    let offset=staging_buffer.buffer_in_use_size;
                    staging_buffer.buffer_in_use_size+=buffer_memory_requirements.size;

                    //map staging (!)
                    let memory_pointer=unsafe{
                        self.vulkan.device.map_memory(staging_buffer.buffer.memory,offset,size,vk::MemoryMapFlags::empty())
                    }.unwrap();

                    //memcpy
                    unsafe{
                        libc::memcpy(memory_pointer,vertex_indices.as_ptr() as *const libc::c_void,size as usize);
                    }

                    //flush
                    let flush_range=vk::MappedMemoryRange{
                        memory:staging_buffer.buffer.memory,
                        offset,
                        //size,
                        size:vk::WHOLE_SIZE,
                        ..Default::default()
                    };
                    unsafe{
                        self.vulkan.device.flush_mapped_memory_ranges(&[flush_range])
                    }.unwrap();

                    //unmap
                    unsafe{
                        self.vulkan.device.unmap_memory(staging_buffer.buffer.memory);
                    }

                    let command_buffer=self.transfer_queue.command_buffers[0];

                    let begin_info=vk::CommandBufferBeginInfo{

                        ..Default::default()
                    };
                    
                    let wait_semaphores=vec![];
                    let command_buffers=vec![
                        command_buffer
                    ];
                    let signal_semaphores=vec![];
                    let submit_info=vk::SubmitInfo{
                        wait_semaphore_count:wait_semaphores.len() as u32,
                        p_wait_semaphores:wait_semaphores.as_ptr(),
                        p_wait_dst_stage_mask:&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        command_buffer_count:command_buffers.len() as u32,
                        p_command_buffers:command_buffers.as_ptr(),
                        signal_semaphore_count:signal_semaphores.len() as u32,
                        p_signal_semaphores:signal_semaphores.as_ptr(),
                        ..Default::default()
                    };

                    unsafe{
                        self.vulkan.device.device_wait_idle().unwrap();
                        self.vulkan.device.begin_command_buffer(command_buffer,&begin_info).unwrap();
                        self.vulkan.device.cmd_copy_buffer(command_buffer,staging_buffer.buffer.buffer,buffer,&[
                            vk::BufferCopy{
                                src_offset:offset,
                                dst_offset:0,
                                size:size,
                            }
                        ]);
                        self.vulkan.device.end_command_buffer(command_buffer).unwrap();
                        
                        self.vulkan.device.queue_submit(self.transfer_queue.queue,&[submit_info],vk::Fence::null()).unwrap();
                    }

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
            //self.vulkan.device.cmd_pipeline_barrier(self.transfer_queue.command_buffers[0], vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::VERTEX_INPUT, vk::DependencyFlags::empty(), &[], &buffer_memory_barriers[..], &[]);
        }

        let mesh=std::sync::Arc::new(Mesh{
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

        self.meshes.insert(name,mesh.clone());

        mesh
    }

    pub fn new_staging(&mut self,size:u64){
        let buffer_create_info=vk::BufferCreateInfo{
            size:size,
            usage:vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode:vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        let buffer=unsafe{
            self.vulkan.device.create_buffer(&buffer_create_info,self.vulkan.get_allocation_callbacks())
        }.unwrap();

        let mut memory=vk::DeviceMemory::null();

        let buffer_memory_requirements=unsafe{
            self.vulkan.device.get_buffer_memory_requirements(buffer)
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
                    self.vulkan.device.allocate_memory(&memory_allocate_info,self.vulkan.get_allocation_callbacks())
                }.unwrap();
                //bind
                let memory_offset=0;
                unsafe{
                    self.vulkan.device.bind_buffer_memory(buffer,memory,memory_offset)
                }.unwrap();

                break;
            }
        }
        if memory==vk::DeviceMemory::null(){
            panic!("staging buffer has no memory")
        }

        self.staging_buffers.push(StagingBuffer{
            buffer:IntegratedBuffer{
                buffer_size:size,
                item_count:size,
                buffer,
                memory,
            },
            buffer_in_use_size:0,
        });
    }

    pub fn get_texture(&mut self,filename:&String,command_buffer:vk::CommandBuffer)->std::sync::Arc<Image>{
        //return cached texture if present
        if let Some(texture)=self.textures.get(filename){
            return texture.clone();
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
                self.vulkan.device.create_image(&image_create_info,self.vulkan.get_allocation_callbacks()).unwrap()
            }
        };

        //allocate image memory and upload data into staging buffer
        //then schedule commands to copy image data from staging into image memory
        let mut memory=vk::DeviceMemory::null();
        {
            let image_memory_reqirements=unsafe{
                self.vulkan.device.get_image_memory_requirements(image)
            };
            if self.staging_buffers[0].buffer.buffer_size<image_memory_reqirements.size{
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
                        memory=self.vulkan.device.allocate_memory(&memory_allocate_info, self.vulkan.get_allocation_callbacks()).unwrap()
                    }
                    
                    //bind memory to image handle
                    unsafe{
                        self.vulkan.device.bind_image_memory(image, memory, 0)
                    }.unwrap();

                    let offset=self.staging_buffers[0].buffer_in_use_size;//offset mesh data because staging buffer is used mesh and texture upload, with no synchronization against each other (could do that, somehow?)
                    self.staging_buffers[0].buffer_in_use_size+=image_memory_reqirements.size;

                    //map staging memory
                    let memory_pointer=unsafe{
                        self.vulkan.device.map_memory(self.staging_buffers[0].buffer.memory, offset, image_memory_reqirements.size, vk::MemoryMapFlags::empty())
                    }.unwrap();

                    //copy image data to staging
                    unsafe{
                        libc::memcpy(memory_pointer,native_image.into_raw().as_mut_ptr() as *mut libc::c_void,image_memory_reqirements.size as usize);
                    }

                    //flush staging and unmap after
                    let flush_range=vk::MappedMemoryRange{
                        memory:self.staging_buffers[0].buffer.memory,
                        offset,
                        size:image_memory_reqirements.size,
                        ..Default::default()
                    };
                    unsafe{
                        self.vulkan.device.flush_mapped_memory_ranges(&[flush_range]).unwrap();
                        self.vulkan.device.unmap_memory(self.staging_buffers[0].buffer.memory);
                    }

                    //schedule image data transfer from staging to final

                    let command_buffer=self.transfer_queue.command_buffers[0];

                    let begin_info=vk::CommandBufferBeginInfo{

                        ..Default::default()
                    };
                    
                    let wait_semaphores=vec![];
                    let command_buffers=vec![
                        command_buffer
                    ];
                    let signal_semaphores=vec![];
                    let submit_info=vk::SubmitInfo{
                        wait_semaphore_count:wait_semaphores.len() as u32,
                        p_wait_semaphores:wait_semaphores.as_ptr(),
                        p_wait_dst_stage_mask:&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                        command_buffer_count:command_buffers.len() as u32,
                        p_command_buffers:command_buffers.as_ptr(),
                        signal_semaphore_count:signal_semaphores.len() as u32,
                        p_signal_semaphores:signal_semaphores.as_ptr(),
                        ..Default::default()
                    };
                    
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
                        self.vulkan.device.device_wait_idle().unwrap();

                        self.vulkan.device.begin_command_buffer(command_buffer,&begin_info).unwrap();

                        //perform buffer layout transition from copy target to vertex data source
                        self.vulkan.device.cmd_pipeline_barrier(command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[image_memory_barrier_none_to_transfer]);
                        
                        self.vulkan.device.cmd_copy_buffer_to_image(command_buffer, self.staging_buffers[0].buffer.buffer, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[buffer_image_copy_info]);

                        self.vulkan.device.cmd_pipeline_barrier( command_buffer, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::FRAGMENT_SHADER, vk::DependencyFlags::empty(), &[], &[], &[image_memory_barrier_transfer_to_shader_read]);

                        self.vulkan.device.end_command_buffer(command_buffer).unwrap();

                        self.vulkan.device.queue_submit(self.transfer_queue.queue,&[submit_info],vk::Fence::null()).unwrap();
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
                self.vulkan.device.create_image_view(&image_view_create_info,self.vulkan.get_allocation_callbacks()).unwrap()
            }
        };

        let image=std::sync::Arc::new(Image{
            width,
            height,
            format:vk::Format::R8G8B8A8_UNORM,
            memory,
            image,
            image_view,
            layout:vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        });

        self.textures.insert(String::from(filename),image.clone());

        image
    }
}

#[cfg(disabled)]
impl Drop for Decoder{
    fn drop(&mut self){
        for(_name,texture) in self.textures.iter(){
            unsafe{
                self.vulkan.device.destroy_image_view(texture.image_view,self.get_allocation_callbacks());
                self.vulkan.device.destroy_image(texture.image,self.get_allocation_callbacks());
                self.vulkan.device.free_memory(texture.memory,self.get_allocation_callbacks());
            }
        }
        for (_name,mesh) in self.meshes.iter(){
            unsafe{
                self.vulkan.device.free_memory(mesh.vertices.memory,self.get_allocation_callbacks());
                self.vulkan.device.destroy_buffer(mesh.vertices.buffer, self.get_allocation_callbacks());

                self.vulkan.device.free_memory(mesh.vertex_indices.memory,self.get_allocation_callbacks());
                self.vulkan.device.destroy_buffer(mesh.vertex_indices.buffer, self.get_allocation_callbacks());
            }
        }

        unsafe{
            self.vulkan.device.free_memory(self.staging_buffer.memory,self.get_allocation_callbacks());
            self.vulkan.device.destroy_buffer(self.staging_buffer.buffer, self.get_allocation_callbacks());
        }

    }
}