#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(target_os="windows")]
extern crate winapi;

#[cfg(target_os="linux")]
extern crate xcb;

#[macro_use]
extern crate memoffset;
extern crate libc;

extern crate image;
extern crate ash;
extern crate nalgebra_glm as glm;
extern crate wavefront_obj as obj;

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


pub mod event;
pub use event::{Event};

pub mod control_flow;
pub use control_flow::{ControlFlow};

pub mod test_window;
pub use test_window::{TestWindow,TestWindowHandle};

pub mod window;
pub use window::{Window,WindowHandle};

pub mod decoder;
pub use decoder::{Decoder,Vertex,IntegratedBuffer,Mesh,Image};

pub mod painter;
pub use painter::{Painter};

pub struct Object{
    pub mesh:std::sync::Arc<Mesh>,
    pub texture:std::sync::Arc<Image>,
}
pub struct GraphicsPipeline{
    layout:vk::PipelineLayout,
    pipeline:vk::Pipeline,
    vertex:vk::ShaderModule,
    fragment:vk::ShaderModule,
}

#[cfg(target_os="windows")]
static mut WINDOWS_WINDOW_EVENTS:Vec<Event>=Vec::new();
#[cfg(target_os="windows")]
unsafe extern "system" fn windowproc(window:HWND,umsg:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
    match umsg{
        WM_CLOSE=>{
            WINDOWS_WINDOW_EVENTS.push(Event::WindowCloseRequested);
            return 0;
        },
        _=>DefWindowProcA(window,umsg,wparam,lparam)
    }
}

pub enum WindowManagerHandle{
    #[cfg(target_os="windows")]
    Windows{
        hinstance:HINSTANCE,
        class_name:String,
    },
    #[cfg(target_os="linux")]
    Xcb{
        connection:*mut base::xcb_connection_t,
    },
    #[allow(dead_code)]
    NeverMatch
}
impl WindowManagerHandle{
    pub fn new()->Self{
        #[cfg(target_os="windows")]
        {
            let hinstance=unsafe{
                GetModuleHandleA(std::ptr::null())
            } as HINSTANCE;
            if hinstance==std::ptr::null_mut(){
                panic!("hInstance")
            }
            let mut class:WNDCLASSA=unsafe{
                std::mem::zeroed()
            };
            class.lpfnWndProc=Some(windowproc);
            class.hInstance=hinstance;
            let class_name=String::from("mywindowclass");
            class.lpszClassName=class_name.as_str().as_ptr() as LPCSTR;//needs to be same address as the one used for CreateWindowEx
            unsafe{
                RegisterClassA(&class)
            };

            Self::Windows{
                hinstance,
                class_name
            }
        }
        #[cfg(not(target_os="windows"))]
        {
            let connection=unsafe{
                xcb_connect(std::ptr::null(),std::ptr::null_mut())
            };
            if connection==std::ptr::null_mut(){
                panic!("xcb_connect");
            }

            WindowManagerHandle::Xcb{
                connection,
            }
        }
    }
    pub fn destroy(&mut self){
        match self{
            #[cfg(target_os="windows")]
            WindowManagerHandle::Windows{..}=>{
                //class does not need to be deleted
            },
            #[cfg(target_os="linux")]
            WindowManagerHandle::Xcb{connection}=>{
                //connection implements drop
            },
            _=>unreachable!()
        }
    }

    #[cfg(target_os="linux")]
    fn xcb_check_cookie(&self,cookie:&base::xcb_void_cookie_t,message:&'static str){
        match self{
            WindowManagerHandle::Xcb{connection}=>{
                let error=unsafe{
                    base::xcb_request_check(*connection,*cookie)
                };
                if error!=std::ptr::null_mut(){
                    panic!("cookie failed on '{}', with minor error code {}, major error code {}",
                        message,
                        unsafe{(*error)}.minor_code,
                        unsafe{(*error)}.major_code,
                        //unsafe{(*error)}.error_code,
                    );
                }
            },
            _=>unreachable!()
        }
    }

    #[cfg(target_os="linux")]
    fn get_intern_atom(&self,name:&str)->xcb_atom_t{
        match self{
            WindowManagerHandle::Xcb{connection,..}=>{
                let name_cstr=name.as_ptr() as *const i8;
                let cookie = unsafe{
                    xcb_intern_atom(*connection, 0, libc::strlen (name_cstr) as u16, name_cstr )
                };
                let mut generic_error:*mut xcb_generic_error_t=std::ptr::null_mut();
                let reply = unsafe{
                    xcb_intern_atom_reply ( *connection, cookie, &mut generic_error as *mut *mut xcb_generic_error_t )
                };
                if generic_error!=std::ptr::null_mut(){
                    panic!("intern atom reply retrieve failed with error code {}",unsafe{*generic_error}.error_code)
                };
                let atom=unsafe{*reply}.atom;
                unsafe{
                    libc::free(reply as *mut libc::c_void);
                }
                return atom;
            },
            _=>unreachable!()
        }
    }
}
struct Manager{
    window_manager_handle:WindowManagerHandle,
    open_windows:Vec<Window>,
    entry:Entry,
    allocation_callbacks:Option<AllocationCallbacks>,
    instance:Instance,
    physical_device:PhysicalDevice,
    device:Device,

    surface:extensions::khr::Surface,
    present_queue:vk::Queue,
    present_queue_family_index:u32,
    present_queue_command_pool:vk::CommandPool,
    present_queue_command_buffers:Vec<vk::CommandBuffer>,
    frame_sync_fence:vk::Fence,

    swapchain_surface_format:vk::SurfaceFormatKHR,

    painter:std::mem::ManuallyDrop<Painter>,
    decoder:std::mem::ManuallyDrop<Decoder>,
}
impl Manager{
    fn get_allocation_callbacks(&self)->Option<&vk::AllocationCallbacks>{
        self.allocation_callbacks.as_ref()
    }

    pub fn new()->Self{
        let window_manager_handle=WindowManagerHandle::new();
        let open_windows=Vec::new();

        let entry=unsafe{
            Entry::new().unwrap()
        };
        let application_name="my application";
        let engine_name="my engine";

        let allocation_callbacks:Option<vk::AllocationCallbacks>=None;
        let temp_allocation_callbacks=allocation_callbacks.as_ref();

        let app_info=vk::ApplicationInfo{
            p_application_name:application_name.as_ptr() as *const i8,
            application_version:vk::make_version(0,1,0),
            p_engine_name:engine_name.as_ptr() as *const i8,
            engine_version:vk::make_version(0,1,0),
            api_version:vk::make_version(1,2,0),
            ..Default::default()
        };
        let instance_layers:Vec<&str>=vec![
            "VK_LAYER_KHRONOS_validation\0"//manual 0 termination because str.as_ptr() does not do that
        ];
        let instance_layer_names:Vec<*const i8>=instance_layers.iter().map(|l| l.as_ptr() as *const i8).collect();
        let instance_extensions=vec![
            "VK_KHR_surface\0",
            #[cfg(target_os="windows")]
            "VK_KHR_win32_surface\0",
            #[cfg(target_os="linux")]
            "VK_KHR_xcb_surface\0",
        ];
        let instance_extension_names:Vec<*const i8>=instance_extensions.iter().map(|e| e.as_ptr() as *const i8).collect();
        let instance_info=vk::InstanceCreateInfo{
            p_application_info:&app_info,
            enabled_layer_count:instance_layer_names.len() as u32,
            pp_enabled_layer_names:instance_layer_names.as_ptr(),
            enabled_extension_count:instance_extension_names.len() as u32,
            pp_enabled_extension_names:instance_extension_names.as_ptr(),
            ..Default::default()
        };

        let instance=unsafe{
            entry.create_instance(&instance_info,temp_allocation_callbacks).unwrap()
        };

        //create test window with surface that has identical properties to the surfaces used for regular windows later on
        //required to test which device has a queue family that can present to these surfaces
        //the window will destroy itself at the end of this function
        let test_window=TestWindow::new(&window_manager_handle,&entry,&instance,temp_allocation_callbacks);

        let device_layers:Vec<&str>=vec![
        ];
        let device_layer_names:Vec<*const i8>=device_layers.iter().map(|l| l.as_ptr() as *const i8).collect();

        let device_extensions:Vec<&str>=vec![
            "VK_KHR_swapchain\0",
        ];
        let device_extension_names:Vec<*const i8>=device_extensions.iter().map(|e| e.as_ptr() as *const i8).collect();

        let mut graphics_queue=vk::Queue::null();
        let mut present_queue=vk::Queue::null();

        //custom queue creation pipeline for more streamlined queue creation process (which does not take queue family max count into account...)
        struct CustomQueueCreateInfo<'a>{
            queue_family_index:u32,
            queues_data:Vec<PriorityAndReference<'a>>,
            flag_requirements:vk::QueueFlags,
            presentation_support:bool,
        }
        struct PriorityAndReference<'a>{
            priority:f32,
            reference:&'a mut vk::Queue,
        }
        let mut queue_create_infos=vec![
            CustomQueueCreateInfo{//graphics queue
                queue_family_index:u32::MAX,
                queues_data:vec![
                    PriorityAndReference{
                        priority:1.0f32,
                        reference:&mut graphics_queue
                    },
                ],
                flag_requirements:vk::QueueFlags::GRAPHICS,
                presentation_support: false,
            },
            CustomQueueCreateInfo{
                queue_family_index:u32::MAX,
                queues_data:vec![
                    PriorityAndReference{
                        priority:1.0f32,
                        reference:&mut present_queue
                    },
                ],
                flag_requirements:vk::QueueFlags::empty(),
                presentation_support: true,
            }
        ];

        //find fit physical device
        let physical_device:PhysicalDevice=*unsafe{
            instance.enumerate_physical_devices()
        }.unwrap().iter().find(|pd|{
            for qci in queue_create_infos.iter_mut(){
                qci.queue_family_index=u32::MAX;
            }

            let _properties=unsafe{
                instance.get_physical_device_properties(**pd)
            };
            let _features=unsafe{
                instance.get_physical_device_features(**pd)
            };

            //check if all extensions required are supported
            let extension_properties=unsafe{
                instance.enumerate_device_extension_properties(**pd)
            }.unwrap();
            let mut extensions_supported=device_extension_names.iter().enumerate().map(
                |(i,e)|
                match extension_properties.iter().find(
                    |p|
                    {
                        unsafe{
                            libc::strcmp(*e,p.extension_name.as_ptr())==0
                        }
                    }
                ){
                    Some(_)=>true,
                    None=>{
                        println!("extension {} unsupported",device_extensions[i]);
                        false
                    }
                }
            );
            if extensions_supported.find(|&e| !e).is_some(){
                return false;
            }

            for (i,queue_family_property) in unsafe{
                instance.get_physical_device_queue_family_properties(**pd)
            }.iter().enumerate(){
                for qci in queue_create_infos.iter_mut(){
                    if queue_family_property.queue_flags.contains(qci.flag_requirements) && !(qci.presentation_support && ! match &test_window.handle{
                        #[cfg(target_os="windows")]
                        TestWindowHandle::Windows{hwnd:_,win32_surface}=>{
                            unsafe{
                                win32_surface.get_physical_device_win32_presentation_support(**pd,i as u32)
                            }
                        },
                        #[cfg(target_os="linux")]
                        TestWindowHandle::Xcb{connection,window,xcb_surface,visual}=>{
                            unsafe{
                                xcb_surface.get_physical_device_xcb_presentation_support(**pd,i as u32,unsafe{
                                    std::mem::transmute::<*mut libc::c_void,&mut libc::c_void>((*connection) as *mut libc::c_void)
                                },*visual)
                            }
                        },
                        _=>unimplemented!()
                    }){
                        qci.queue_family_index=i as u32;
                    }
                }
                
                if queue_create_infos.iter().map(|qci| qci.queue_family_index==u32::MAX).find(|r| *r).is_some(){
                    return false;
                }else{
                    break; //all queue requirements fulfilled, stop queue fitness test
                }
            }

            true
        }).expect("no fit physical device found");

        let present_queue_family_index=queue_create_infos[0].queue_family_index;
        let graphics_queue_family_index=queue_create_infos[1].queue_family_index;

        //merge queues into data structure that has max 1 entry per queue family
        let mut merged_queue_map=std::collections::HashMap::<u32,Vec<usize>>::new();
        let mut merged_queue_create_infos:Vec<vk::DeviceQueueCreateInfo>=Vec::new();

        for (i,queue_create_info) in queue_create_infos.iter().enumerate(){
            let qfi=queue_create_info.queue_family_index;

            if let Some(qci)=merged_queue_map.get_mut(&qfi){
                qci.push(i);
            }else{
                merged_queue_map.insert(qfi,vec![i]);
            }
        }
        //preallocate storage for references to all vector containing the queue priorities
        //this vector will be deallocated _after_ device creation, so the priority vectors will live long enough
        let mut queue_priorities_storage=Vec::with_capacity(merged_queue_map.len());
        for (qfi,q) in merged_queue_map.iter(){
            let mut queue_priorities=Vec::new();
            for i in q.iter(){
                queue_priorities.append(
                    &mut queue_create_infos[*i]
                    .queues_data
                    .iter().map(|e|e.priority).collect()
                );
            }
            queue_priorities_storage.push(queue_priorities);
            let queue_priorities_ref=queue_priorities_storage.iter().next().unwrap();
            merged_queue_create_infos.push(
                vk::DeviceQueueCreateInfo{
                    queue_family_index:*qfi as u32,
                    queue_count:queue_priorities_ref.len() as u32,
                    p_queue_priorities:queue_priorities_ref.as_ptr() as *const f32,
                    ..Default::default()
                }
            );
        }

        let device_create_info=vk::DeviceCreateInfo{
            queue_create_info_count:merged_queue_create_infos.len() as u32,
            p_queue_create_infos:merged_queue_create_infos.as_ptr(),
            enabled_layer_count:device_layer_names.len() as u32,
            pp_enabled_layer_names:device_layer_names.as_ptr(),
            enabled_extension_count:device_extension_names.len() as u32,
            pp_enabled_extension_names:device_extension_names.as_ptr(),
            ..Default::default()
        };
        let device=unsafe{
            instance.create_device(physical_device,&device_create_info,temp_allocation_callbacks)
        }.unwrap();

        queue_priorities_storage.clear();

        //retrieve queues into original structure
        for (qfi,q) in merged_queue_map.iter(){
            let mut index=0;
            for i in q.iter(){
                let qci=&mut queue_create_infos[*i];
                for j in index..index+qci.queues_data.len(){
                    unsafe{
                        *(qci.queues_data[(j-index) as usize].reference)=device.get_device_queue(*qfi,j as u32);
                    }
                    //println!("got queue {} {}",*i,j-index);
                }
                index+=qci.queues_data.len();
            }
        }

        let surface=extensions::khr::Surface::new(&entry,&instance);
        
        //create command pools for each queue
        let present_queue_command_pool_create_info=vk::CommandPoolCreateInfo{
            flags:vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index:present_queue_family_index,
            ..Default::default()
        };
        let graphics_queue_command_pool_create_info=vk::CommandPoolCreateInfo{
            flags:vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index:graphics_queue_family_index,
            ..Default::default()
        };

        let present_queue_command_pool=unsafe{
            device.create_command_pool(&present_queue_command_pool_create_info,temp_allocation_callbacks)
        }.unwrap();
        let graphics_queue_command_pool=unsafe{
            device.create_command_pool(&graphics_queue_command_pool_create_info,temp_allocation_callbacks)
        }.unwrap();

        //create command buffers for each command pool
        let graphics_queue_command_buffers_create_info=vk::CommandBufferAllocateInfo{
            command_pool:graphics_queue_command_pool,
            level:vk::CommandBufferLevel::PRIMARY,
            command_buffer_count:1,
            ..Default::default()
        };
        let graphics_queue_command_buffers=unsafe{
            device.allocate_command_buffers(&graphics_queue_command_buffers_create_info)
        }.unwrap();

        let present_queue_command_buffers_create_info=vk::CommandBufferAllocateInfo{
            command_pool:present_queue_command_pool,
            level:vk::CommandBufferLevel::PRIMARY,
            command_buffer_count:1,
            ..Default::default()
        };
        let present_queue_command_buffers=unsafe{
            device.allocate_command_buffers(&present_queue_command_buffers_create_info)
        }.unwrap();

        let semaphore_create_info=vk::SemaphoreCreateInfo{
            ..Default::default()
        };
        let rendering_done=unsafe{
            device.create_semaphore(&semaphore_create_info,temp_allocation_callbacks)
        }.unwrap();

        //used to wait for last frame to be finished (and synchronized with max framerate) before new frame starts
        //must be signaled to simulate last frame being finished on first frame
        let fence_create_info=vk::FenceCreateInfo{
            flags:vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };
        let frame_sync_fence=unsafe{
            device.create_fence(&fence_create_info,temp_allocation_callbacks)
        }.unwrap();
        
        //create render pass for simple rendering operations
        let surface_formats=unsafe{
            surface.get_physical_device_surface_formats(physical_device, test_window.platform_surface)
        }.unwrap();
        //use first available format, but check for two 'better' alternatives
        let mut swapchain_surface_format=surface_formats[0];
        //if the only supported format is 'undefined', there is no preferred format for the surface
        //then use 'most widely used' format
        if surface_formats.len()==1 && swapchain_surface_format.format==vk::Format::UNDEFINED{
            swapchain_surface_format=vk::SurfaceFormatKHR{
                format:vk::Format::R8G8B8A8_UNORM,
                color_space:vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };
        }else{
            for format in surface_formats.iter(){
                if format.format==vk::Format::R8G8B8A8_UNORM{
                    swapchain_surface_format=*format;
                }
            }
        }

        //Painter related stuff
        let painter;
        {
            let render_pass={
                let render_pass_attachment_descriptions=vec![
                    vk::AttachmentDescription{
                        format:swapchain_surface_format.format,
                        samples:vk::SampleCountFlags::TYPE_1,
                        load_op:vk::AttachmentLoadOp::CLEAR,
                        store_op:vk::AttachmentStoreOp::STORE,
                        initial_layout:vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        final_layout:vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        ..Default::default()
                    }
                ];
                let render_pass_subpass_color_attachment_references=vec![
                    vk::AttachmentReference{
                        attachment:0,
                        layout:vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }
                ];
                let render_pass_subpass_descriptions=vec![
                    vk::SubpassDescription{
                        pipeline_bind_point:vk::PipelineBindPoint::GRAPHICS,
                        color_attachment_count:render_pass_subpass_color_attachment_references.len() as u32,
                        p_color_attachments:render_pass_subpass_color_attachment_references.as_ptr(),
                        ..Default::default()
                    }
                ];
                let render_pass_subpass_dependencies=vec![

                ];
                let render_pass_create_info=vk::RenderPassCreateInfo{
                    attachment_count:render_pass_attachment_descriptions.len() as u32,
                    p_attachments:render_pass_attachment_descriptions.as_ptr(),
                    subpass_count:render_pass_subpass_descriptions.len() as u32,
                    p_subpasses:render_pass_subpass_descriptions.as_ptr(),
                    dependency_count:render_pass_subpass_dependencies.len() as u32,
                    p_dependencies:render_pass_subpass_dependencies.as_ptr(),
                    ..Default::default()
                };

                unsafe{
                    device.create_render_pass(&render_pass_create_info,temp_allocation_callbacks)
                }.unwrap()
            };

            let sampler={
                let sampler_create_info=vk::SamplerCreateInfo{
                    mag_filter:vk::Filter::LINEAR,
                    min_filter:vk::Filter::LINEAR,
                    mipmap_mode:vk::SamplerMipmapMode::NEAREST,
                    address_mode_u:vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    address_mode_v:vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    address_mode_w:vk::SamplerAddressMode::CLAMP_TO_EDGE,
                    mip_lod_bias:0.0,
                    anisotropy_enable:false as u32,
                    compare_enable:false as u32,
                    min_lod:0.0,
                    max_lod:0.0,
                    border_color:vk::BorderColor::FLOAT_TRANSPARENT_BLACK,
                    unnormalized_coordinates:false as u32,
                    ..Default::default()
                };

                unsafe{
                    device.create_sampler(&sampler_create_info,temp_allocation_callbacks)
                }.unwrap()
            };

            let descriptor_set_layout={
                let descriptor_set_layout_bindings=vec![
                    vk::DescriptorSetLayoutBinding{
                        binding:0,
                        descriptor_type:vk::DescriptorType::COMBINED_IMAGE_SAMPLER,//combine image and sampler into a single descriptor
                        descriptor_count:1,
                        stage_flags:vk::ShaderStageFlags::FRAGMENT,
                        p_immutable_samplers:std::ptr::null(),
                    }
                ];
                let descriptor_set_layout_create_info=vk::DescriptorSetLayoutCreateInfo{
                    binding_count:descriptor_set_layout_bindings.len() as u32,
                    p_bindings:descriptor_set_layout_bindings.as_ptr(),
                    ..Default::default()
                };

                unsafe{
                    device.create_descriptor_set_layout(&descriptor_set_layout_create_info,temp_allocation_callbacks)
                }.unwrap()
            };

            let descriptor_pool={
                let descriptor_pool_sizes=vec![
                    vk::DescriptorPoolSize{
                        ty:vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                        descriptor_count:1,
                    }
                ];
                let descriptor_pool_create_info=vk::DescriptorPoolCreateInfo{
                    max_sets:1,
                    pool_size_count:descriptor_pool_sizes.len() as u32,
                    p_pool_sizes:descriptor_pool_sizes.as_ptr(),
                    ..Default::default()
                };

                unsafe{
                    device.create_descriptor_pool(&descriptor_pool_create_info,temp_allocation_callbacks)
                }.unwrap()
            };

            let descriptor_set_layouts=vec![
                descriptor_set_layout
            ];
            let descriptor_sets={
                let descriptor_set_allocate_info=vk::DescriptorSetAllocateInfo{
                    descriptor_pool,
                    descriptor_set_count:descriptor_set_layouts.len() as u32,
                    p_set_layouts:descriptor_set_layouts.as_ptr(),
                    ..Default::default()
                };

                unsafe{
                    device.allocate_descriptor_sets(&descriptor_set_allocate_info)
                }.unwrap()
            };
            let descriptor_set=descriptor_sets[0];

            let graphics_pipeline_layout_2d={
                let graphics_pipeline_layout_create_info=vk::PipelineLayoutCreateInfo{
                    //descriptor set layouts
                    set_layout_count:descriptor_set_layouts.len() as u32,
                    p_set_layouts:descriptor_set_layouts.as_ptr(),
                    //push constant ranges
                    ..Default::default()
                };

                unsafe{
                    device.create_pipeline_layout(&graphics_pipeline_layout_create_info,temp_allocation_callbacks)
                }.unwrap()
            };

            let (vertex_2d,fragment_2d)={
                let vertex_shader_code=std::fs::read("textured_polygon_2d.vert.spv").unwrap();
                let vertex_shader_create_info=vk::ShaderModuleCreateInfo{
                    code_size:vertex_shader_code.len(), //size in bytes
                    p_code:vertex_shader_code.as_ptr() as *const u32,//but pointer to 4byte unsigned integers
                    ..Default::default()
                };

                let fragment_shader_code=std::fs::read("textured_polygon_2d.frag.spv").unwrap();
                let fragment_shader_create_info=vk::ShaderModuleCreateInfo{
                    code_size:fragment_shader_code.len(),
                    p_code:fragment_shader_code.as_ptr() as *const u32,
                    ..Default::default()
                };
                
                unsafe{
                    (
                        device.create_shader_module(&vertex_shader_create_info,temp_allocation_callbacks).unwrap(),
                        device.create_shader_module(&fragment_shader_create_info,temp_allocation_callbacks).unwrap(),
                    )
                }
            };
            
            let graphics_pipeline_layout_3d={
                let push_constants=vec![
                    vk::PushConstantRange{
                        stage_flags:vk::ShaderStageFlags::VERTEX,
                        offset:0,
                        size:(std::mem::size_of::<glm::Mat4>()*3) as u32,
                    },
                ];
                let graphics_pipeline_layout_create_info=vk::PipelineLayoutCreateInfo{
                    //descriptor set layouts
                    set_layout_count:descriptor_set_layouts.len() as u32,
                    p_set_layouts:descriptor_set_layouts.as_ptr(),
                    //push constant ranges
                    push_constant_range_count:push_constants.len() as u32,
                    p_push_constant_ranges:push_constants.as_ptr(),
                    ..Default::default()
                };

                unsafe{
                    device.create_pipeline_layout(&graphics_pipeline_layout_create_info,temp_allocation_callbacks)
                }.unwrap()
            };

            let (vertex_3d,fragment_3d)={
                let vertex_shader_code=std::fs::read("textured_polygon_3d.vert.spv").unwrap();
                let vertex_shader_create_info=vk::ShaderModuleCreateInfo{
                    code_size:vertex_shader_code.len(), //size in bytes
                    p_code:vertex_shader_code.as_ptr() as *const u32,//but pointer to 4byte unsigned integers
                    ..Default::default()
                };

                let fragment_shader_code=std::fs::read("textured_polygon_3d.frag.spv").unwrap();
                let fragment_shader_create_info=vk::ShaderModuleCreateInfo{
                    code_size:fragment_shader_code.len(),
                    p_code:fragment_shader_code.as_ptr() as *const u32,
                    ..Default::default()
                };
                
                unsafe{
                    (
                        device.create_shader_module(&vertex_shader_create_info,temp_allocation_callbacks).unwrap(),
                        device.create_shader_module(&fragment_shader_create_info,temp_allocation_callbacks).unwrap(),
                    )
                }
            };

            let graphics_pipelines={
                let shader_entry_fn_name="main\0".as_ptr() as *const i8;

                let shader_stage_create_infos_2d=vec![
                    vk::PipelineShaderStageCreateInfo{
                        stage:vk::ShaderStageFlags::VERTEX,
                        module:vertex_2d,
                        p_name:shader_entry_fn_name,
                        ..Default::default()
                    },
                    vk::PipelineShaderStageCreateInfo{
                        stage:vk::ShaderStageFlags::FRAGMENT,
                        module:fragment_2d,
                        p_name:shader_entry_fn_name,
                        ..Default::default()
                    }
                ];
                let vertex_binding_descriptions=vec![
                    vk::VertexInputBindingDescription{
                        binding: 0,
                        stride: std::mem::size_of::<Vertex>() as u32,
                        input_rate:vk::VertexInputRate::VERTEX,
                    },
                ];
                let vertex_attribute_descriptions=vec![
                    vk::VertexInputAttributeDescription{
                        location:0,
                        binding:vertex_binding_descriptions[0].binding,
                        format:vk::Format::R32G32B32A32_SFLOAT,
                        offset:offset_of!(Vertex,x) as u32,
                    },
                    vk::VertexInputAttributeDescription{
                        location:1,
                        binding:vertex_binding_descriptions[0].binding,
                        format:vk::Format::R32G32_SFLOAT,
                        offset:offset_of!(Vertex,u) as u32,
                    },
                ];
                let vertex_input_state_create_info=vk::PipelineVertexInputStateCreateInfo{
                    vertex_binding_description_count:vertex_binding_descriptions.len() as u32,
                    p_vertex_binding_descriptions:vertex_binding_descriptions.as_ptr(),
                    vertex_attribute_description_count:vertex_attribute_descriptions.len() as u32,
                    p_vertex_attribute_descriptions:vertex_attribute_descriptions.as_ptr(),
                    ..Default::default()
                };
                let input_assembly_state_create_info=vk::PipelineInputAssemblyStateCreateInfo{
                    //topology:vk::PrimitiveTopology::TRIANGLE_LIST,
                    topology:vk::PrimitiveTopology::TRIANGLE_LIST,
                    primitive_restart_enable:false as u32,
                    ..Default::default()
                };
                let viewport_state_create_info=vk::PipelineViewportStateCreateInfo{
                    viewport_count:1,
                    //p_viewports:&viewport,//this is ignored if viewports are dynamic
                    scissor_count:1,//must match viewport count
                    //p_scissors:&scissor,//ignored if scissors are dynamic
                    ..Default::default()
                };
                let rasterization_state_create_info=vk::PipelineRasterizationStateCreateInfo{
                    depth_clamp_enable:false as u32,
                    rasterizer_discard_enable:false as u32,
                    polygon_mode:vk::PolygonMode::FILL,
                    cull_mode: vk::CullModeFlags::BACK,
                    front_face: vk::FrontFace::COUNTER_CLOCKWISE,
                    depth_bias_enable:false as u32,
                    line_width:1.0,//specs state this must be 1.0 if wide lines feature is not enabled
                    ..Default::default()
                };
                let multisample_state_create_info=vk::PipelineMultisampleStateCreateInfo{
                    rasterization_samples:vk::SampleCountFlags::TYPE_1,
                    sample_shading_enable:false as u32,
                    alpha_to_coverage_enable:false as u32,
                    alpha_to_one_enable:false as u32,
                    ..Default::default()
                };
                let color_blend_attachment_state=vk::PipelineColorBlendAttachmentState{
                    blend_enable:false as u32,
                    //src_color_blend_factor:vk::BlendFactor::ONE,
                    //dst_color_blend_factor:vk::BlendFactor::ZERO,
                    //color_blend_op:vk::BlendOp::ADD,
                    //src_alpha_blend_factor:vk::BlendFactor::ONE,
                    //dst_alpha_blend_factor:vk::BlendFactor::ZERO,
                    //alpha_blend_op:vk::BlendOp::ADD,
                    color_write_mask:vk::ColorComponentFlags::all(),//disable blending, but which color channels are forwarded still needs to be specified
                    ..Default::default()
                };
                let color_blend_state=vk::PipelineColorBlendStateCreateInfo{
                    logic_op_enable:false as u32,
                    logic_op:vk::LogicOp::COPY,
                    attachment_count:1,
                    p_attachments:&color_blend_attachment_state,
                    blend_constants:[0.0,0.0,0.0,0.0,],
                    ..Default::default()
                };
                let dynamic_states=vec![
                    vk::DynamicState::VIEWPORT,
                    vk::DynamicState::SCISSOR,
                ];
                let dynamic_state_create_info=vk::PipelineDynamicStateCreateInfo{
                    dynamic_state_count:dynamic_states.len() as u32,
                    p_dynamic_states:dynamic_states.as_ptr(),
                    ..Default::default()
                };
                let graphics_pipeline_create_info_2d=vk::GraphicsPipelineCreateInfo{
                    stage_count:shader_stage_create_infos_2d.len() as u32,
                    p_stages:shader_stage_create_infos_2d.as_ptr(),
                    p_vertex_input_state:&vertex_input_state_create_info,
                    p_input_assembly_state:&input_assembly_state_create_info,
                    p_viewport_state:&viewport_state_create_info,
                    p_rasterization_state:&rasterization_state_create_info,
                    p_multisample_state:&multisample_state_create_info,
                    p_color_blend_state:&color_blend_state,
                    p_dynamic_state:&dynamic_state_create_info,
                    layout:graphics_pipeline_layout_2d,
                    render_pass,
                    subpass:0,
                    base_pipeline_index:-1,
                    base_pipeline_handle:vk::Pipeline::null(),
                    ..Default::default()
                };
                
                let shader_stage_create_infos_3d=vec![
                    vk::PipelineShaderStageCreateInfo{
                        stage:vk::ShaderStageFlags::VERTEX,
                        module:vertex_3d,
                        p_name:shader_entry_fn_name,
                        ..Default::default()
                    },
                    vk::PipelineShaderStageCreateInfo{
                        stage:vk::ShaderStageFlags::FRAGMENT,
                        module:fragment_3d,
                        p_name:shader_entry_fn_name,
                        ..Default::default()
                    }
                ];
                let graphics_pipeline_create_info_3d=vk::GraphicsPipelineCreateInfo{
                    stage_count:shader_stage_create_infos_3d.len() as u32,
                    p_stages:shader_stage_create_infos_3d.as_ptr(),
                    p_vertex_input_state:&vertex_input_state_create_info,
                    p_input_assembly_state:&input_assembly_state_create_info,
                    p_viewport_state:&viewport_state_create_info,
                    p_rasterization_state:&rasterization_state_create_info,
                    p_multisample_state:&multisample_state_create_info,
                    p_color_blend_state:&color_blend_state,
                    p_dynamic_state:&dynamic_state_create_info,
                    layout:graphics_pipeline_layout_3d,
                    render_pass,
                    subpass:0,
                    base_pipeline_index:-1,
                    base_pipeline_handle:vk::Pipeline::null(),
                    ..Default::default()
                };

                unsafe{
                    device.create_graphics_pipelines(vk::PipelineCache::null(), &[graphics_pipeline_create_info_2d,graphics_pipeline_create_info_3d],temp_allocation_callbacks)
                }.unwrap()
            };
            let graphics_pipeline_2d=GraphicsPipeline{
                layout:graphics_pipeline_layout_2d,
                pipeline:graphics_pipelines[0],
                vertex:vertex_2d,
                fragment:fragment_2d
            };
            let graphics_pipeline_3d=GraphicsPipeline{
                layout:graphics_pipeline_layout_3d,
                pipeline:graphics_pipelines[1],
                vertex:vertex_3d,
                fragment:fragment_3d
            };
            
            painter=std::mem::ManuallyDrop::new(Painter{
                allocation_callbacks,

                device:device.clone(),

                swapchain_surface_format,

                sampler,

                descriptor_pool,
                descriptor_set_layout,
                descriptor_set,

                render_pass,

                graphics_pipeline_2d,
                graphics_pipeline_3d,

                rendering_done,

                graphics_queue,
                graphics_queue_family_index,
                graphics_queue_command_pool,
                graphics_queue_command_buffers,
            });
        }

        let decoder={
            //create staging buffer for resource upload with fixed size (should be enough for this exercise)
            let buffer_size=10*1024*1024;
            let buffer_create_info=vk::BufferCreateInfo{
                size:buffer_size,
                usage:vk::BufferUsageFlags::TRANSFER_SRC,
                sharing_mode:vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let buffer=unsafe{
                device.create_buffer(&buffer_create_info,temp_allocation_callbacks)
            }.unwrap();

            let mut memory=vk::DeviceMemory::null();

            let buffer_memory_requirements=unsafe{
                device.get_buffer_memory_requirements(buffer)
            };

            let device_memory_properties=unsafe{
                instance.get_physical_device_memory_properties(physical_device)
            };

            for memory_type_index in 0..device_memory_properties.memory_type_count{
                if (buffer_memory_requirements.memory_type_bits & (1<<memory_type_index))>0 
                && device_memory_properties.memory_types[memory_type_index as usize].property_flags.intersects(vk::MemoryPropertyFlags::HOST_VISIBLE){
                    //allocate
                    let memory_allocate_info=vk::MemoryAllocateInfo{
                        allocation_size:buffer_memory_requirements.size,
                        memory_type_index,
                        ..Default::default()
                    };
                    memory=unsafe{
                        device.allocate_memory(&memory_allocate_info,temp_allocation_callbacks)
                    }.unwrap();
                    //bind
                    let memory_offset=0;
                    unsafe{
                        device.bind_buffer_memory(buffer,memory,memory_offset)
                    }.unwrap();

                    break;
                }
            }
            if memory==vk::DeviceMemory::null(){
                panic!("staging buffer has no memory")
            }

            std::mem::ManuallyDrop::new(Decoder{
                allocation_callbacks,

                device:device.clone(),

                device_memory_properties,

                staging_buffer:IntegratedBuffer{
                    buffer_size,
                    item_count:buffer_size,//story abitrary bytes
                    buffer,
                    memory,
                },
                staging_buffer_in_use_size:0,

                meshes:std::collections::HashMap::new(),
                textures:std::collections::HashMap::new(),
            })
        };

        Self{
            window_manager_handle,
            open_windows,

            entry,

            allocation_callbacks,

            instance:instance.clone(),
            physical_device,
            device:device.clone(),

            surface,

            present_queue,
            present_queue_family_index,
            present_queue_command_pool,
            present_queue_command_buffers,

            frame_sync_fence,

            swapchain_surface_format,

            painter,

            decoder,
        }
    }

    pub fn create_semaphore(&self)->VkResult<vk::Semaphore>{
        let semaphore_create_info=vk::SemaphoreCreateInfo{
            ..Default::default()
        };
        unsafe{
            self.device.create_semaphore(&semaphore_create_info,self.get_allocation_callbacks())
        }
    }

    pub fn create_fence(&self,signaled:bool)->VkResult<vk::Fence>{
        let fence_create_info=vk::FenceCreateInfo{
            flags:if signaled{
                vk::FenceCreateFlags::SIGNALED
            }else{
                vk::FenceCreateFlags::empty()
            },
            ..Default::default()
        };
        unsafe{
            self.device.create_fence(&fence_create_info,self.get_allocation_callbacks())
        }
    }

    pub fn new_window(&mut self,width:u16,height:u16,title:&str){
        let surface;
        let handle={
            match &self.window_manager_handle{
                #[cfg(target_os="windows")]
                WindowManagerHandle::Windows{hinstance,class_name}=>{
                    let window_hwnd:HWND=unsafe{
                        CreateWindowExA(
                            0,
                            class_name.as_str().as_ptr() as LPCSTR,
                            title.as_ptr() as LPCSTR,
                            WS_OVERLAPPEDWINDOW,
                            100,
                            100,
                            width as i32,
                            height as i32,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            *hinstance,
                            std::ptr::null_mut(),
                        )
                    };
                    if window_hwnd==std::ptr::null_mut(){
                        panic!("CreateWindowExA")
                    }

                    unsafe{
                        ShowWindow(window_hwnd,SW_SHOWDEFAULT);
                        UpdateWindow(window_hwnd);
                    }

                    let win32_surface=ash::extensions::khr::Win32Surface::new(&self.entry,&self.instance);
                    
                    let surface_create_info=vk::Win32SurfaceCreateInfoKHR{
                        hinstance:*hinstance as *const libc::c_void,
                        hwnd:window_hwnd as *const libc::c_void,
                        ..Default::default()
                    };
                    surface=unsafe{
                        win32_surface.create_win32_surface(&surface_create_info,self.get_allocation_callbacks())
                    }.unwrap();

                    WindowHandle::Windows{
                        hwnd:window_hwnd,
                        win32_surface,
                    }
                },
                #[cfg(target_os="linux")]
                WindowManagerHandle::Xcb{connection}=>{
                    let window=unsafe{
                        xcb_generate_id(*connection)
                    };

                    let setup=unsafe{
                        xcb_get_setup(*connection)
                    };
                    let roots_iterator=unsafe{
                        xproto::xcb_setup_roots_iterator(setup)
                    };
                    let screen=roots_iterator.data;
                    let visual=unsafe{
                        (*screen).root_visual
                    };

                    let mask=xproto::XCB_CW_EVENT_MASK;
                    let values=vec![
                        xproto::XCB_EVENT_MASK_KEY_PRESS
                        | xproto::XCB_EVENT_MASK_KEY_RELEASE
                        | xproto::XCB_EVENT_MASK_BUTTON_PRESS
                        | xproto::XCB_EVENT_MASK_BUTTON_RELEASE
                        | xproto::XCB_EVENT_MASK_ENTER_WINDOW
                        | xproto::XCB_EVENT_MASK_LEAVE_WINDOW
                        | xproto::XCB_EVENT_MASK_POINTER_MOTION
                        | xproto::XCB_EVENT_MASK_POINTER_MOTION_HINT
                        | xproto::XCB_EVENT_MASK_BUTTON_1_MOTION
                        | xproto::XCB_EVENT_MASK_BUTTON_2_MOTION
                        | xproto::XCB_EVENT_MASK_BUTTON_3_MOTION
                        | xproto::XCB_EVENT_MASK_BUTTON_4_MOTION
                        | xproto::XCB_EVENT_MASK_BUTTON_5_MOTION
                        | xproto::XCB_EVENT_MASK_BUTTON_MOTION
                        | xproto::XCB_EVENT_MASK_KEYMAP_STATE
                        //| xproto::XCB_EVENT_MASK_EXPOSURE
                        //| xproto::XCB_EVENT_MASK_VISIBILITY_CHANGE
                        | xproto::XCB_EVENT_MASK_STRUCTURE_NOTIFY
                        | xproto::XCB_EVENT_MASK_RESIZE_REDIRECT
                        | xproto::XCB_EVENT_MASK_FOCUS_CHANGE
                        | xproto::XCB_EVENT_MASK_PROPERTY_CHANGE
                    ];

                    let create_window_cookie=unsafe{
                        xproto::xcb_create_window_checked(
                            *connection,
                            base::XCB_COPY_FROM_PARENT as u8,
                            window,
                            (*screen).root,
                            100,
                            100,
                            width,
                            height,
                            10,
                            xproto::XCB_WINDOW_CLASS_INPUT_OUTPUT as u16,
                            (*screen).root_visual,
                            mask,
                            values.as_ptr()
                        )
                    };

                    self.window_manager_handle.xcb_check_cookie(&create_window_cookie,"create window");

                    unsafe{
                        base::xcb_flush(*connection)
                    };

                    //set window decorations (?)
                    let window_type_atom=self.window_manager_handle.get_intern_atom("_NET_WM_WINDOW_TYPE\0");
                    let window_type_normal_atom=self.window_manager_handle.get_intern_atom("_NET_WM_WINDOW_TYPE_NORMAL\0");

                    let window_type_cookie=unsafe{
                        xcb_change_property_checked ( 
                            *connection,
                            XCB_PROP_MODE_REPLACE as u8,
                            window,
                            window_type_atom, //property
                            xproto::XCB_ATOM_ATOM, //type
                            32,//format (8,16 or 32 bits per entry in value list)
                            1, //length of value list
                            &window_type_normal_atom as *const xcb_atom_t as *const libc::c_void
                        )
                    };
                    self.window_manager_handle.xcb_check_cookie(&window_type_cookie,"change property");

                    //set window title
                    for prop_name in vec![
                        "WM_NAME\0",
                        "_NET_WM_NAME\0",
                        "_NET_WM_VISIBLE_NAME\0",
                        "WM_ICON_NAME\0",
                        "_NET_WM_ICON_NAME\0",
                        "_NET_WM_VISIBLE_ICON_NAME\0"
                    ].iter(){
                        let cookie=unsafe{
                            let atom=self.window_manager_handle.get_intern_atom(prop_name);
                            xcb_change_property_checked ( 
                                *connection,
                                XCB_PROP_MODE_REPLACE as u8,
                                window,
                                atom, //property
                                xproto::XCB_ATOM_STRING, //type
                                8,//format (8,16 or 32 bits per entry in value list)
                                libc::strlen(title.as_ptr() as *const i8) as u32, //length of value list
                                title.as_ptr() as *const i8 as *const libc::c_void // is this is a motif hints struct
                            )
                        };
                        self.window_manager_handle.xcb_check_cookie(&window_type_cookie,"change property");
                    }

                    let close=self.window_manager_handle.get_intern_atom("WM_DELETE_WINDOW\0");//_NET_CLOSE_WINDOW
                    let hidden=self.window_manager_handle.get_intern_atom("_NET_WM_STATE_HIDDEN\0");
                    let maximized_vertical=self.window_manager_handle.get_intern_atom("_NET_WM_STATE_MAXIMIZED_VERT\0");
                    let maximized_horizontal=self.window_manager_handle.get_intern_atom("_NET_WM_STATE_MAXIMIZED_HORZ\0");

                    let cookie=unsafe{
                        xcb_change_property_checked ( 
                            *connection,
                            XCB_PROP_MODE_REPLACE as u8,
                            window,
                            self.window_manager_handle.get_intern_atom("WM_PROTOCOLS\0"), //property
                            xproto::XCB_ATOM_ATOM, //type
                            32,//format (8,16 or 32 bits per entry in value list)
                            1, //length of value list
                            &close as *const xcb_atom_t as *const libc::c_void // is this is a motif hints struct
                        )
                    };
                    self.window_manager_handle.xcb_check_cookie(&window_type_cookie,"change property");

                    let map_window_cookie=unsafe{
                        xcb_map_window(*connection,window)
                    };
                    self.window_manager_handle.xcb_check_cookie(&map_window_cookie,"map window");

                    unsafe{
                        base::xcb_flush(*connection)
                    };

                    let xcb_surface=ash::extensions::khr::XcbSurface::new(&self.entry,&self.instance);

                    let surface_create_info=vk::XcbSurfaceCreateInfoKHR{
                        connection:*connection as *mut libc::c_void,
                        window:window,
                        ..Default::default()
                    };
                    surface=unsafe{
                        xcb_surface.create_xcb_surface(&surface_create_info,self.get_allocation_callbacks())
                    }.unwrap();

                    WindowHandle::Xcb{
                        connection:*connection,
                        visual,
                        window,
                        xcb_surface,
                        close,
                        hidden,
                        maximized_horizontal,
                        maximized_vertical,
                    }
                },
                _=>unimplemented!()
            }
        };
                  
        //vulkan spec states this must be done
        if unsafe{
            !self.surface.get_physical_device_surface_support(self.physical_device, self.present_queue_family_index, surface).unwrap()
        }{
            panic!("new surface does not support presentation like the temporary ones");
        }

        //create swapchain
        let image_available=self.create_semaphore().unwrap();
        let image_transferable=self.create_semaphore().unwrap();
        let image_presentable=self.create_semaphore().unwrap();

        let surface_capabilities=unsafe{
            self.surface.get_physical_device_surface_capabilities(self.physical_device, surface)
        }.unwrap();

        let mut image_count:u32=surface_capabilities.min_image_count+1;
        if surface_capabilities.max_image_count>0 && image_count>surface_capabilities.max_image_count{
            image_count=surface_capabilities.max_image_count;
        }

        let surface_formats=unsafe{
            self.surface.get_physical_device_surface_formats(self.physical_device, surface)
        }.unwrap();
        //use first available format, but check for two 'better' alternatives
        let mut surface_format=surface_formats[0];
        //if the only supported format is 'undefined', there is no preferred format for the surface
        //then use 'most widely used' format
        if surface_formats.len()==1 && surface_format.format==vk::Format::UNDEFINED{
            surface_format=vk::SurfaceFormatKHR{
                format:vk::Format::R8G8B8A8_UNORM,
                color_space:vk::ColorSpaceKHR::SRGB_NONLINEAR,
            };
        }else{
            for format in surface_formats.iter(){
                if format.format==vk::Format::R8G8B8A8_UNORM{
                    surface_format=*format;
                }
            }
        }

        //set extent to current extent, according to surface
        //if that is not available (indicated by special values of 'current extent')
        //extent will be specified by swapchain specs, which are those used for the 
        //creation of this window
        let mut swapchain_extent=surface_capabilities.current_extent;
        if swapchain_extent.width==u32::MAX || swapchain_extent.height==u32::MAX{
            swapchain_extent.width=width as u32;
            swapchain_extent.height=height as u32;
            if swapchain_extent.width<surface_capabilities.min_image_extent.width{
                swapchain_extent.width=surface_capabilities.min_image_extent.width;
            }
            if swapchain_extent.height<surface_capabilities.min_image_extent.height{
                swapchain_extent.height=surface_capabilities.min_image_extent.height;
            }
            if swapchain_extent.width>surface_capabilities.max_image_extent.width{
                swapchain_extent.width=surface_capabilities.max_image_extent.width;
            }
            if swapchain_extent.height>surface_capabilities.max_image_extent.height{
                swapchain_extent.height=surface_capabilities.max_image_extent.height;
            }
        }

        let swapchain_surface_usage_flags=vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST;
        if !surface_capabilities.supported_usage_flags.contains(swapchain_surface_usage_flags){
            panic!("surface capabilities");
        }

        let swapchain_surface_transform=if surface_capabilities.supported_transforms.contains(vk::SurfaceTransformFlagsKHR::IDENTITY){
            vk::SurfaceTransformFlagsKHR::IDENTITY
        }else{
            surface_capabilities.current_transform
        };

        let surface_present_modes=unsafe{
            self.surface.get_physical_device_surface_present_modes(self.physical_device, surface)
        }.unwrap();
        let swapchain_surface_present_mode=if surface_present_modes.contains(&vk::PresentModeKHR::MAILBOX){
            vk::PresentModeKHR::MAILBOX
        }else{
            vk::PresentModeKHR::FIFO
        };

        //queue family indices accessing the swapchain (e.g. presenting to it), for which we have a dedicated queue
        let queue_family_indices=vec![
            self.present_queue_family_index,
        ];
        let swapchain_create_info=vk::SwapchainCreateInfoKHR{
            surface,
            min_image_count:image_count,
            image_format:surface_format.format,
            image_color_space:surface_format.color_space,
            image_extent:swapchain_extent,
            image_array_layers:1,
            image_usage:swapchain_surface_usage_flags,
            image_sharing_mode:vk::SharingMode::EXCLUSIVE,
            queue_family_index_count:queue_family_indices.len() as u32,
            p_queue_family_indices:queue_family_indices.as_ptr(),
            pre_transform:swapchain_surface_transform,
            composite_alpha:vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode:swapchain_surface_present_mode,
            clipped:false as u32,
            ..Default::default()
        };
        let swapchain=extensions::khr::Swapchain::new(&self.instance,&self.device);
        let swapchain_handle=unsafe{
            swapchain.create_swapchain(&swapchain_create_info, self.get_allocation_callbacks())
        }.unwrap();

        //images may only be created when this function is valled, so presenting an image index before
        //this function is called violate the specs (the image may not exist yet)
        let swapchain_images=unsafe{
            swapchain.get_swapchain_images(swapchain_handle)
        }.unwrap();

        let subresource_range=vk::ImageSubresourceRange{
            aspect_mask:vk::ImageAspectFlags::COLOR,
            base_mip_level:0,
            level_count:1,
            base_array_layer:0,
            layer_count:1,
            ..Default::default()
        };     

        let swapchain_image_views:Vec<vk::ImageView>=swapchain_images.iter().map(|image|{
            let image_view_create_info=vk::ImageViewCreateInfo{
                image:*image,
                view_type:vk::ImageViewType::TYPE_2D,
                format:self.swapchain_surface_format.format,
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
                self.device.create_image_view(&image_view_create_info, self.get_allocation_callbacks())
            }.unwrap()
        }).collect();

        let swapchain_image_framebuffers:Vec<vk::Framebuffer>=swapchain_image_views.iter().map(|view|{
            let framebuffer_create_info=vk::FramebufferCreateInfo{
                render_pass:self.painter.render_pass,
                attachment_count:1,
                p_attachments:view,
                width:swapchain_extent.width,
                height:swapchain_extent.height,
                layers:1,
                ..Default::default()
            };
            unsafe{
                self.device.create_framebuffer(&framebuffer_create_info, self.get_allocation_callbacks())
            }.unwrap()
        }).collect();

        let window=Window{
            extent:swapchain_extent,
            handle,
            surface,
            image_available,
            image_transferable,
            image_presentable,
            swapchain,
            swapchain_handle,
            swapchain_images,
            swapchain_image_views,
            swapchain_image_framebuffers,
        };
        self.open_windows.push(window);
    }
    fn destroy_window(&mut self,open_window_index:usize){
        for framebuffer in self.open_windows[open_window_index].swapchain_image_framebuffers.iter(){
            unsafe{
                self.device.destroy_framebuffer(*framebuffer, self.get_allocation_callbacks());
            }
        }
        for image_view in self.open_windows[open_window_index].swapchain_image_views.iter(){
            unsafe{
                self.device.destroy_image_view(*image_view, self.get_allocation_callbacks());
            }
        }
        unsafe{
            self.device.destroy_semaphore(self.open_windows[open_window_index].image_available, self.get_allocation_callbacks());
            self.device.destroy_semaphore(self.open_windows[open_window_index].image_transferable, self.get_allocation_callbacks());
            self.device.destroy_semaphore(self.open_windows[open_window_index].image_presentable, self.get_allocation_callbacks());
            self.open_windows[open_window_index].swapchain.destroy_swapchain(self.open_windows[open_window_index].swapchain_handle,self.get_allocation_callbacks());
            self.surface.destroy_surface(self.open_windows[open_window_index].surface,self.get_allocation_callbacks())
        };
        match self.open_windows[open_window_index].handle{
            #[cfg(target_os="windows")]
            WindowHandle::Windows{hwnd,..}=>{
                unsafe{
                    DestroyWindow(hwnd);
                }
            },
            #[cfg(target_os="linux")]
            WindowHandle::Xcb{connection,window,..}=>{
                unsafe{
                    xcb_destroy_window(connection,window)
                };
            },
            _=>unreachable!()
        }
    }

    pub fn step(&mut self)->ControlFlow{
        //handle window i/o
        match self.window_manager_handle{
            #[cfg(target_os="windows")]
            WindowManagerHandle::Windows{..}=>{
                let mut msg:MSG=unsafe{
                    std::mem::zeroed()
                };
                while unsafe{
                    PeekMessageA(&mut msg,std::ptr::null_mut(),0,0,PM_REMOVE)>0
                }{
                    unsafe{
                        TranslateMessage(&mut msg);
                        DispatchMessageA(&mut msg);
                    }
                }
        
                #[cfg(target_os="windows")]
                {
                    for ev in unsafe{
                        WINDOWS_WINDOW_EVENTS.iter()
                    }{
                        match ev{
                            Event::WindowCloseRequested=>{
                                return ControlFlow::Stop;
                            }
                            _=>{}
                        }
                    }
                    unsafe{
                        WINDOWS_WINDOW_EVENTS.clear();
                    }
                }
            },
            #[cfg(target_os="linux")]
            WindowManagerHandle::Xcb{connection}=>{
                unsafe{
                    xcb_flush(connection);
                }

                let mut generic_event;

                while{
                    generic_event=unsafe{
                        xcb_poll_for_event(connection)
                    };
                    generic_event!=std::ptr::null_mut()
                }{
                    let response_type=unsafe{*generic_event}.response_type & 0x7f;
                    match response_type{
                        xproto::XCB_KEY_PRESS=>{
                            //println!("key pressed");
                        },
                        xproto::XCB_KEY_RELEASE=>{
                            //println!("key pressed");
                        },
                        xproto::XCB_CLIENT_MESSAGE=>{
                            let event=generic_event as *const xproto::xcb_client_message_event_t;
                            match self.open_windows[0].handle{
                                WindowHandle::Xcb{close,..}=>{
                                    if unsafe{*event}.data.data32()[0]==close{
                                        return ControlFlow::Stop;
                                    }
                                },
                                _=>unreachable!()
                            }
                        },
                        _=>{}
                    }
                }
            },
            _=>panic!("unsupported")
        }

        //render below
        
        //wait for last frame to finish
        unsafe{
            self.device.wait_for_fences(&[self.frame_sync_fence], true, u64::MAX).unwrap();
            self.device.reset_fences(&[self.frame_sync_fence]).unwrap();
        }

        //acquire next swapchain image for drawing and presenting
        let (image_index,suboptimal)=unsafe{
            self.open_windows[0].swapchain.acquire_next_image(self.open_windows[0].swapchain_handle, u64::MAX, self.open_windows[0].image_available, vk::Fence::null())
        }.unwrap();

        //this means the swapchain should be recreated, but we dont care much right now
        if suboptimal{
            println!("swapchain image acquired is suboptimal");
            return ControlFlow::Stop;
        }

        let swapchain_image=self.open_windows[0].swapchain_images[image_index as usize];

        //transfer swapchain image ownership to graphics queue for drawing
        {
            //begin transition command buffer 1
            let present_queue_command_buffer_begin_info=vk::CommandBufferBeginInfo{
                flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };
            unsafe{
                self.device.begin_command_buffer(self.present_queue_command_buffers[0],&present_queue_command_buffer_begin_info)
            }.unwrap();
            let subresource_range=vk::ImageSubresourceRange{
                aspect_mask:vk::ImageAspectFlags::COLOR,
                base_mip_level:0,
                level_count:1,
                base_array_layer:0,
                layer_count:1,
            };
            let image_memory_barrier=vk::ImageMemoryBarrier{
                src_access_mask:vk::AccessFlags::MEMORY_READ,
                dst_access_mask:vk::AccessFlags::MEMORY_READ,
                old_layout:vk::ImageLayout::UNDEFINED,
                new_layout:vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                src_queue_family_index:self.present_queue_family_index,
                dst_queue_family_index:self.present_queue_family_index,
                image:swapchain_image,
                subresource_range,
                ..Default::default()
            };
            unsafe{
                self.device.cmd_pipeline_barrier(
                    self.present_queue_command_buffers[0], 
                    vk::PipelineStageFlags::TOP_OF_PIPE, 
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, 
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[image_memory_barrier]
                )
            };
            //end transition command buffer 1
            unsafe{
                self.device.end_command_buffer(self.present_queue_command_buffers[0])
            }.unwrap();
            //submit transition command buffer 1
            let wait_semaphores_1=vec![
                self.open_windows[0].image_available,
            ];
            let dst_stage_masks_1=vec![
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ];
            let command_buffers_1=vec![
                self.present_queue_command_buffers[0]
            ];
            let signal_semaphores_1=vec![
                self.open_windows[0].image_transferable
            ];
            let submit_info_1=vk::SubmitInfo{
                wait_semaphore_count:wait_semaphores_1.len() as u32,
                p_wait_semaphores:wait_semaphores_1.as_ptr(),
                p_wait_dst_stage_mask:dst_stage_masks_1.as_ptr(),
                command_buffer_count:command_buffers_1.len() as u32,
                p_command_buffers:command_buffers_1.as_ptr(),
                signal_semaphore_count:signal_semaphores_1.len() as u32,
                p_signal_semaphores:signal_semaphores_1.as_ptr(),
                ..Default::default()
            };
            unsafe{
                self.device.queue_submit(self.present_queue,&[submit_info_1],vk::Fence::null())
            }.unwrap();
        }

        //upload resources if required, and draw them
        {
            let graphics_queue_command_buffer_begin_info=vk::CommandBufferBeginInfo{
                flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };
            unsafe{
                self.device.begin_command_buffer(self.painter.graphics_queue_command_buffers[0], &graphics_queue_command_buffer_begin_info)
            }.unwrap();
    
            //barrier from top to transfer
            unsafe{
                self.device.cmd_pipeline_barrier(self.painter.graphics_queue_command_buffers[0],vk::PipelineStageFlags::TOP_OF_PIPE,vk::PipelineStageFlags::TRANSFER,vk::DependencyFlags::empty(),&[],&[],&[])
            };

            //record mesh upload
            let quad_data=self.decoder.get_mesh("quad.obj",self.painter.graphics_queue_command_buffers[0]);

            //record texture upload (use staging buffer range outside of potential mesh upload range)
            let intel_truck=self.decoder.get_texture("inteltruck.png", self.painter.graphics_queue_command_buffers[0]);
            
            //set descriptor set data here for now (only needs to be done once, ever, but i dont know where)
            {
                let descriptor_image_info=vk::DescriptorImageInfo{
                    sampler:self.painter.sampler,
                    image_view:intel_truck.image_view,
                    image_layout:vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                };
                let write_descriptor_set=vk::WriteDescriptorSet{
                    dst_set:self.painter.descriptor_set,
                    dst_binding:0,
                    dst_array_element:0,
                    descriptor_count:1,
                    descriptor_type:vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    p_image_info:&descriptor_image_info,
                    ..Default::default()
                };
                unsafe{
                    self.device.update_descriptor_sets(&[write_descriptor_set],&[])
                };
                unsafe{
                    self.device.device_wait_idle()
                }.unwrap();
            }

            self.painter.draw(
                self.open_windows[0].swapchain_image_framebuffers[image_index as usize],
                self.open_windows[0].extent,
                &vec![Object{
                    mesh:quad_data.clone(),
                    texture:intel_truck
                }],
                self.open_windows[0].image_transferable
            );
        }
        self.decoder.staging_buffer_in_use_size=0;

        //retrieve swapchain image from graphics queue for presentation
        {
            let present_queue_command_buffer_begin_info=vk::CommandBufferBeginInfo{
                flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };
            //begin transition command buffer 2
            unsafe{
                self.device.begin_command_buffer(self.present_queue_command_buffers[0],&present_queue_command_buffer_begin_info)
            }.unwrap();
            let subresource_range=vk::ImageSubresourceRange{
                aspect_mask:vk::ImageAspectFlags::COLOR,
                base_mip_level:0,
                level_count:1,
                base_array_layer:0,
                layer_count:1,
            };
            let image_memory_barrier=vk::ImageMemoryBarrier{
                src_access_mask:vk::AccessFlags::MEMORY_READ,
                dst_access_mask:vk::AccessFlags::MEMORY_READ,
                old_layout:vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                new_layout:vk::ImageLayout::PRESENT_SRC_KHR,
                src_queue_family_index:self.present_queue_family_index,
                dst_queue_family_index:self.present_queue_family_index,
                image:swapchain_image,
                subresource_range,
                ..Default::default()
            };
            unsafe{
                self.device.cmd_pipeline_barrier(
                    self.present_queue_command_buffers[0], 
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, 
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[image_memory_barrier]
                )
            };
            //end transition command buffer 2
            unsafe{
                self.device.end_command_buffer(self.present_queue_command_buffers[0])
            }.unwrap();
            //submit transition command buffer 2
            let wait_semaphores_2=vec![
                self.painter.rendering_done,
            ];
            let dst_stage_masks_2=vec![
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ];
            let command_buffers_2=vec![
                self.present_queue_command_buffers[0]
            ];
            let signal_semaphores_2=vec![
                self.open_windows[0].image_presentable
            ];
            let submit_info_2=vk::SubmitInfo{
                wait_semaphore_count:wait_semaphores_2.len() as u32,
                p_wait_semaphores:wait_semaphores_2.as_ptr(),
                p_wait_dst_stage_mask:dst_stage_masks_2.as_ptr(),
                command_buffer_count:command_buffers_2.len() as u32,
                p_command_buffers:command_buffers_2.as_ptr(),
                signal_semaphore_count:signal_semaphores_2.len() as u32,
                p_signal_semaphores:signal_semaphores_2.as_ptr(),
                ..Default::default()
            };
            unsafe{
                self.device.queue_submit(self.present_queue,&[submit_info_2],self.frame_sync_fence)
            }.unwrap();
        }

        //present swapchain image
        {
            let present_wait_semaphores=vec![
                self.open_windows[0].image_presentable,
            ];
            let mut present_results=vec![
                vk::Result::SUCCESS,
            ];
            let present_info=vk::PresentInfoKHR{
                wait_semaphore_count:present_wait_semaphores.len() as u32,
                p_wait_semaphores:present_wait_semaphores.as_ptr(),
                swapchain_count:1,
                p_swapchains:&self.open_windows[0].swapchain_handle,
                p_image_indices:&image_index,
                p_results:present_results.as_mut_ptr(),
                ..Default::default()
            };
            unsafe{
                self.open_windows[0].swapchain.queue_present(self.present_queue,&present_info)
            }.unwrap();
        }

        ControlFlow::Continue
    }

    pub fn run(&mut self){
        loop{
            if self.step()!=ControlFlow::Continue{
                break;
            }

            //cap framerate
            let max_fps=5;
            std::thread::sleep(std::time::Duration::from_millis(1000/max_fps));
        }
    }
}
impl Drop for Manager{
    fn drop(&mut self){
        //finish all gpu interaction, which may include window system interaction before window and vulkan resourse destruction
        unsafe{
            self.device.device_wait_idle()
        }.unwrap();

        unsafe{
            std::mem::ManuallyDrop::drop(&mut self.painter);
            std::mem::ManuallyDrop::drop(&mut self.decoder);
        }

        for open_window_index in 0..self.open_windows.len(){
            self.destroy_window(open_window_index);
        }

        unsafe{
            self.device.destroy_fence(self.frame_sync_fence, self.get_allocation_callbacks());

            self.device.destroy_command_pool(self.present_queue_command_pool, self.get_allocation_callbacks());

            self.device.destroy_device(self.get_allocation_callbacks());

            self.instance.destroy_instance(self.get_allocation_callbacks())
        };

        self.window_manager_handle.destroy();
    }
}

fn main() {
    let mut manager=Manager::new();
    manager.new_window(600,400,"hello milena\0");
    manager.run();
}
