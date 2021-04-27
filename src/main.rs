#![allow(dead_code)]
#![allow(unused_imports)]

#[cfg(target_os="windows")]
extern crate winapi;

#[cfg(target_os="linux")]
extern crate xcb;

#[macro_use]
extern crate memoffset;
extern crate libc;

extern crate ash;
extern crate image;
extern crate nalgebra_glm as glm;

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
pub use painter::{Painter,RenderPass};

pub struct VulkanBase{
    entry:Entry,
    allocation_callbacks:Option<AllocationCallbacks>,
    instance:Instance,
    physical_device:PhysicalDevice,
    device:Device,
}

unsafe impl Send for VulkanBase{}
unsafe impl Sync for VulkanBase{}

impl VulkanBase{
    fn get_allocation_callbacks(&self)->Option<&vk::AllocationCallbacks>{
        self.allocation_callbacks.as_ref()
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
}
impl Drop for VulkanBase{
    fn drop(&mut self){
        unsafe{
            self.device.destroy_device(self.get_allocation_callbacks());

            self.instance.destroy_instance(self.get_allocation_callbacks())
        }
    }
}

pub struct Object{
    pub mesh:std::sync::Arc<Mesh>,
    pub texture:std::sync::Arc<Image>,
}

pub struct Queue{
    vulkan:std::sync::Arc<VulkanBase>,
    queue:vk::Queue,
    family_index:u32,
    command_pool:std::rc::Rc<vk::CommandPool>,
    command_buffers:Vec<vk::CommandBuffer>,
}
impl Queue{
    pub fn create_command_buffers(&mut self,count:u32)->std::ops::Range<usize>{
       let command_buffers_create_info=vk::CommandBufferAllocateInfo{
            command_pool:*self.command_pool,
            level:vk::CommandBufferLevel::PRIMARY,
            command_buffer_count:count,
            ..Default::default()
        };

        let mut command_buffers=unsafe{
            self.vulkan.device.allocate_command_buffers(&command_buffers_create_info)
        }.unwrap();

        let start=self.command_buffers.len();
        self.command_buffers.append(&mut command_buffers);
        let end=start+count as usize;

        start..end
    }
}
impl Clone for Queue{
    fn clone(&self)->Self{
        Queue{
            vulkan:self.vulkan.clone(),
            queue:self.queue,
            family_index:self.family_index,
            command_pool:self.command_pool.clone(),
            command_buffers:Vec::new(),
        }
    }
}
impl Drop for Queue{
    fn drop(&mut self){
        if std::rc::Rc::strong_count(&self.command_pool)==1{
            unsafe{
                self.vulkan.device.destroy_command_pool(*self.command_pool, self.vulkan.get_allocation_callbacks());
            }
        }
    }
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
    open_windows:std::collections::HashMap<u32,Window>,

    next_window_id:u32,

    vulkan:std::sync::Arc<VulkanBase>,

    painter:Painter,
    decoder:Decoder,
}
impl Manager{
    pub fn new()->Self{
        let window_manager_handle=WindowManagerHandle::new();

        let entry=unsafe{
            Entry::new().unwrap()
        };

        let allocation_callbacks:Option<vk::AllocationCallbacks>=None;
        let temp_allocation_callbacks=allocation_callbacks.as_ref();

        let instance={
            let application_name="my application";
            let engine_name="my engine";
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
    
            unsafe{
                entry.create_instance(&instance_info,temp_allocation_callbacks).unwrap()
            }
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

        let mut present_queue_family_index=0;
        let mut transfer_queue_family_index=0;
        let mut graphics_queue_family_index=0;
        
        let mut queue_create_infos=Vec::with_capacity(3);

        //find fit physical device
        let physical_device:PhysicalDevice=*unsafe{
            instance.enumerate_physical_devices()
        }.unwrap().iter().find(|pd|{
            present_queue_family_index=u32::MAX;
            transfer_queue_family_index=u32::MAX;
            graphics_queue_family_index=u32::MAX;

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

            let queue_family_properties=unsafe{
                instance.get_physical_device_queue_family_properties(**pd)
            };

            //if only one family is available, assume the worst: only a single queue that does everything
            if queue_family_properties.len()==1{
                present_queue_family_index=0;
                transfer_queue_family_index=0;
                graphics_queue_family_index=0;
                queue_create_infos.push(vk::DeviceQueueCreateInfo{
                    queue_family_index:0 as u32,
                    queue_count:1,
                    p_queue_priorities:std::ptr::null(),
                    ..Default::default()
                });
            }else{
                for (i,queue_family_property) in queue_family_properties.iter().enumerate(){
                    if match &test_window.handle{
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
                    }{
                        present_queue_family_index=i as u32;

                        if let Some(qci)=queue_create_infos.iter_mut().find(|qci| qci.queue_family_index==(i as u32)){
                            qci.queue_count+=1;
                        }else{
                            queue_create_infos.push(vk::DeviceQueueCreateInfo{
                                queue_family_index:i as u32,
                                queue_count:1,
                                p_queue_priorities:std::ptr::null(),
                                ..Default::default()
                            });
                        }
                    }
                    
                    if queue_family_property.queue_flags.contains(vk::QueueFlags::TRANSFER){
                        transfer_queue_family_index=i as u32;

                        if let Some(qci)=queue_create_infos.iter_mut().find(|qci| qci.queue_family_index==(i as u32)){
                            qci.queue_count+=1;
                        }else{
                            queue_create_infos.push(vk::DeviceQueueCreateInfo{
                                queue_family_index:i as u32,
                                queue_count:1,
                                p_queue_priorities:std::ptr::null(),
                                ..Default::default()
                            });
                        }
                    }
                    if queue_family_property.queue_flags.contains(vk::QueueFlags::GRAPHICS){
                        graphics_queue_family_index=i as u32;

                        if let Some(qci)=queue_create_infos.iter_mut().find(|qci| qci.queue_family_index==(i as u32)){
                            qci.queue_count+=1;
                        }else{
                            queue_create_infos.push(vk::DeviceQueueCreateInfo{
                                queue_family_index:i as u32,
                                queue_count:1,
                                p_queue_priorities:std::ptr::null(),
                                ..Default::default()
                            });
                        }
                    }
                }
            }

            //if one of the queue family indices is not set to something below the poison value, the physical device is unfit
            let queue_family_indices=&[present_queue_family_index,transfer_queue_family_index,graphics_queue_family_index];
            if queue_family_indices.iter().find(|qfi| (**qfi)==u32::MAX).is_some(){
                return false;
            }

            true
        }).expect("no fit physical device found");

        let queue_priorities:Vec<Vec<f32>>=queue_create_infos.iter().map(
            |qci| vec![1.0;qci.queue_count as usize]
        ).collect();
        for (index,item) in queue_create_infos.iter_mut().enumerate(){
            item.p_queue_priorities=queue_priorities[index].as_ptr();
        }

        let device_create_info=vk::DeviceCreateInfo{
            queue_create_info_count:queue_create_infos.len() as u32,
            p_queue_create_infos:queue_create_infos.as_ptr(),
            enabled_layer_count:device_layer_names.len() as u32,
            pp_enabled_layer_names:device_layer_names.as_ptr(),
            enabled_extension_count:device_extension_names.len() as u32,
            pp_enabled_extension_names:device_extension_names.as_ptr(),
            ..Default::default()
        };
        let device=unsafe{
            instance.create_device(physical_device,&device_create_info,temp_allocation_callbacks)
        }.unwrap();

        let vulkan=std::sync::Arc::new(VulkanBase{
            entry,

            allocation_callbacks,

            instance:instance.clone(),
            physical_device,
            device:device.clone(),
        });
        
        let mut queue_indices=std::collections::HashMap::new();
        for qci in queue_create_infos.iter(){
            queue_indices.insert(qci.queue_family_index,0);
        }
        
        let present_queue;
        let transfer_queue;
        let graphics_queue;
        if 
            present_queue_family_index==0 &&
            transfer_queue_family_index==0 &&
            graphics_queue_family_index==0 &&
            queue_create_infos[0].queue_count==1
        {
            let queue=match queue_indices.get_mut(&present_queue_family_index){
                Some(i)=>{
                    *i+=1;
                    unsafe{
                        device.get_device_queue(present_queue_family_index,*i-1)
                    }
                },
                None=>unreachable!()
            };

            let command_pool_create_info=vk::CommandPoolCreateInfo{
                flags:vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index:present_queue_family_index,
                ..Default::default()
            };
            let command_pool=unsafe{
                device.create_command_pool(&command_pool_create_info,temp_allocation_callbacks)
            }.unwrap();
            present_queue=crate::Queue{
                vulkan:vulkan.clone(),
                queue,
                family_index:present_queue_family_index,
                command_pool:std::rc::Rc::new(command_pool),
                command_buffers:Vec::new(),
            };
            
            let command_pool_create_info=vk::CommandPoolCreateInfo{
                flags:vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index:transfer_queue_family_index,
                ..Default::default()
            };
            let queue=match queue_indices.get_mut(&transfer_queue_family_index){
                Some(i)=>{
                    *i+=1;
                    unsafe{
                        device.get_device_queue(transfer_queue_family_index,*i-1)
                    }
                },
                None=>unreachable!()
            };
            let command_pool=unsafe{
                device.create_command_pool(&command_pool_create_info,temp_allocation_callbacks)
            }.unwrap();
            transfer_queue=crate::Queue{
                vulkan:vulkan.clone(),
                queue,
                family_index:transfer_queue_family_index,
                command_pool:std::rc::Rc::new(command_pool),
                command_buffers:Vec::new(),
            };
            
            let command_pool_create_info=vk::CommandPoolCreateInfo{
                flags:vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index:graphics_queue_family_index,
                ..Default::default()
            };
            let queue=match queue_indices.get_mut(&graphics_queue_family_index){
                Some(i)=>{
                    *i+=1;
                    unsafe{
                        device.get_device_queue(graphics_queue_family_index,*i-1)
                    }
                },
                None=>unreachable!()
            };
            let command_pool=unsafe{
                device.create_command_pool(&command_pool_create_info,temp_allocation_callbacks)
            }.unwrap();
            graphics_queue=crate::Queue{
                vulkan:vulkan.clone(),
                queue,
                family_index:graphics_queue_family_index,
                command_pool:std::rc::Rc::new(command_pool),
                command_buffers:Vec::new(),
            };
        }else{
            let queue=unsafe{
                device.get_device_queue(0,0)
            };

            let command_pool_create_info=vk::CommandPoolCreateInfo{
                flags:vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
                queue_family_index:0,
                ..Default::default()
            };
            let command_pool=unsafe{
                device.create_command_pool(&command_pool_create_info,temp_allocation_callbacks)
            }.unwrap();
            let queue=crate::Queue{
                vulkan:vulkan.clone(),
                queue,
                family_index:0,
                command_pool:std::rc::Rc::new(command_pool),
                command_buffers:Vec::new(),
            };

            transfer_queue=queue.clone();
            graphics_queue=queue.clone();
            present_queue=queue.clone();
        };

        Self{
            window_manager_handle,
            next_window_id:0,
            open_windows:std::collections::HashMap::new(),

            vulkan:vulkan.clone(),

            painter:Painter::new(&vulkan,&test_window,graphics_queue,present_queue),

            decoder:Decoder::new(&vulkan,transfer_queue),
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

                    let win32_surface=ash::extensions::khr::Win32Surface::new(&self.vulkan.entry,&self.vulkan.instance);
                    
                    let surface_create_info=vk::Win32SurfaceCreateInfoKHR{
                        hinstance:*hinstance as *const libc::c_void,
                        hwnd:window_hwnd as *const libc::c_void,
                        ..Default::default()
                    };
                    surface=unsafe{
                        win32_surface.create_win32_surface(&surface_create_info,self.vulkan.get_allocation_callbacks())
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

        self.next_window_id+=1;

        let window=Window{
            id:self.next_window_id-1,
            extent:vk::Extent2D{
                height:height as u32,
                width:width as u32,
            },
            handle,
            surface,
        };

        self.painter.add_window(&window);

        self.open_windows.insert(window.id,window);
    }

    fn destroy_window(&mut self,window_id:u32){
        let window=self.open_windows.remove(&window_id).unwrap();
        self.painter.remove_window(window.id);

        match window.handle{
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

        self.painter.draw();

        unsafe{
            self.vulkan.device.device_wait_idle()
        }.unwrap();

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
            self.vulkan.device.device_wait_idle()
        }.unwrap();

        for id in self.open_windows.iter().map(|(id,_)| *id).collect::<Vec<u32>>().iter(){
            self.destroy_window(*id);
        }

        self.window_manager_handle.destroy();
    }
}

fn main() {
    let mut manager=Manager::new();
    manager.new_window(600,400,"hello milena\0");
    manager.run();
}
