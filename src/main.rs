#[cfg(target_os="windows")]
extern crate winapi;

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
extern crate xcb;

#[cfg(target_os="linux")]
use xcb::{
    ffi::*,
};

extern crate libc;
extern crate ash;
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
        InstanceV1_0,
        DeviceV1_0,
    },
    Instance,
    Device,
    extensions,
};

#[derive(PartialEq,Debug,Clone,Copy)]
enum ControlFlow{
    Continue,
    Stop,
}
#[derive(PartialEq,Debug,Clone,Copy)]
enum ButtonKeyState{
    Pressed,
    Released,
}
#[derive(PartialEq,Debug,Clone,Copy)]
enum EnterLeave{
    Enter,
    Leave,
}
#[derive(PartialEq,Debug,Clone,Copy)]
enum FocusChange{
    Gained,
    Lost,
}
#[derive(PartialEq,Debug,Clone,Copy)]
enum Event{
    FirstEvent,
    LastEvent,
    ButtonEvent{
        button:u32,
        button_state:ButtonKeyState,
        x:u16,
        y:u16,
        enter_leave:Option<EnterLeave>,

    },
    KeyEvent{
        key:u32,
        key_state:ButtonKeyState,
        x:u16,
        y:u16,
    },
    FocusEvent{
        focus_change:FocusChange,
    },
    ResizeRequestEvent,
    WindowCloseRequested,
    #[allow(dead_code)]
    None,
}

enum TestWindowHandle{
    #[cfg(target_os="windows")]
    Windows{
        hwnd:HWND,
        win32_surface:extensions::khr::Win32Surface,
    },
    #[cfg(target_os="linux")]
    Xcb{
        connection:*mut base::xcb_connection_t,
        visual:xproto::xcb_visualid_t,
        window:u32,
        xcb_surface:ash::extensions::khr::XcbSurface,
    },
    #[allow(dead_code)]
    NeverMatch
}
struct TestWindow{
    handle:TestWindowHandle,
}
impl TestWindow{
    fn new(window_manager_handle:&WindowManagerHandle, entry: &Entry, instance:&Instance)->TestWindow{
        match &window_manager_handle{
            #[cfg(target_os="windows")]
            WindowManagerHandle::Windows{hinstance,class_name}=>{
                let window_hwnd:HWND=unsafe{
                    CreateWindowExA(
                        0,
                        class_name.as_str().as_ptr() as LPCSTR,
                        "my window".as_ptr() as LPCSTR,
                        WS_OVERLAPPEDWINDOW,
                        0,
                        0,
                        150,
                        100,
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
                
                let win32_surface=ash::extensions::khr::Win32Surface::new(entry,instance);

                TestWindow{
                    handle:TestWindowHandle::Windows{
                        hwnd:window_hwnd,
                        win32_surface,
                    },
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

                let mask=0;
                let values=vec![
                ];

                let create_window_cookie=unsafe{
                    xproto::xcb_create_window_checked(
                        *connection,
                        base::XCB_COPY_FROM_PARENT as u8,
                        window,
                        (*screen).root,
                        0,
                        0,
                        150,
                        100,
                        10,
                        xproto::XCB_WINDOW_CLASS_INPUT_OUTPUT as u16,
                        visual,
                        mask,
                        values.as_ptr()
                    )
                };

                window_manager_handle.xcb_check_cookie(&create_window_cookie,"create window");

                let map_window_cookie=unsafe{
                    xcb_map_window(*connection,window)
                };
                window_manager_handle.xcb_check_cookie(&map_window_cookie,"map window");

                unsafe{
                    base::xcb_flush(*connection)
                };

                let xcb_surface=ash::extensions::khr::XcbSurface::new(entry,instance);

                TestWindow{
                    handle:TestWindowHandle::Xcb{
                        connection:*connection,
                        visual,
                        window,
                        xcb_surface,
                    }
                }
            },
            _=>unimplemented!()
        }
    }
}
impl Drop for TestWindow{
    fn drop(&mut self){//,window_manager_handle:&WindowManagerHandle, entry: &mut Entry, instance:&mut Instance){
        //let surface=ash::extensions::khr::Surface::new(&self.entry,&self.instance);
        match self.handle{
            #[cfg(target_os="windows")]
            TestWindowHandle::Windows{hwnd,..}=>{
                unsafe{
                    DestroyWindow(hwnd);
                }
            },
            #[cfg(target_os="linux")]
            TestWindowHandle::Xcb{connection,window,..}=>{
                unsafe{
                    xcb_destroy_window(connection,window)
                };
            },
            _=>unreachable!()
        }

    }
}

enum WindowHandle{
    #[cfg(target_os="windows")]
    Windows{
        hwnd:HWND,
        win32_surface:extensions::khr::Win32Surface,
    },
    #[cfg(target_os="linux")]
    Xcb{
        connection:*mut base::xcb_connection_t,
        visual:xproto::xcb_visualid_t,
        window:u32,
        xcb_surface:ash::extensions::khr::XcbSurface,
        close:xcb_atom_t,
        maximized_horizontal:xcb_atom_t,
        maximized_vertical:xcb_atom_t,
        hidden:xcb_atom_t,
    },
    #[allow(dead_code)]
    NeverMatch
}
struct Window{
    handle:WindowHandle,
    surface:vk::SurfaceKHR,
    image_available:vk::Semaphore,
    image_presentable:vk::Semaphore,
    swapchain:extensions::khr::Swapchain,
    swapchain_handle:vk::SwapchainKHR,
    swapchain_images:Vec<vk::Image>,
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

enum WindowManagerHandle{
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
struct WindowManager<'m>{
    handle:WindowManagerHandle,
    open_windows:Vec<Window>,
    entry:Entry,
    allocation_callbacks:Option<&'m AllocationCallbacks>,
    instance:Instance,
    physical_device:PhysicalDevice,
    device:Device,
    surface:extensions::khr::Surface,
    present_queue:vk::Queue,
    present_queue_family_index:u32,
    present_queue_command_pool:vk::CommandPool,
    graphics_queue:vk::Queue,
    graphics_queue_family_index:u32,
    graphics_queue_command_pool:vk::CommandPool,
}
impl WindowManager<'_>{
    pub fn new()->Self{
        let handle=WindowManagerHandle::new();
        let open_windows=Vec::new();

        let entry=unsafe{
            Entry::new().unwrap()
        };
        let application_name="my application";
        let engine_name="my engine";

        let allocation_callbacks=None;

        let app_info=vk::ApplicationInfo{
            p_application_name:application_name.as_ptr() as *const i8,
            application_version:vk::make_version(0,1,0),
            p_engine_name:engine_name.as_ptr() as *const i8,
            engine_version:vk::make_version(0,1,0),
            api_version:vk::make_version(1,2,0),
            ..Default::default()
        };
        let instance_layers:Vec<&str>=vec![
            //"VK_LAYER_KHRONOS_validation\0"//manual 0 termination because str.as_ptr() does not do that
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
            entry.create_instance(&instance_info,allocation_callbacks).unwrap()
        };

        let test_window=TestWindow::new(&handle,&entry,&instance);

        let device_layers:Vec<&str>=vec![
        ];
        let device_layer_names:Vec<*const i8>=device_layers.iter().map(|l| l.as_ptr() as *const i8).collect();

        let device_extensions:Vec<&str>=vec![
            "VK_KHR_swapchain\0",
        ];
        let device_extension_names:Vec<*const i8>=device_extensions.iter().map(|e| e.as_ptr() as *const i8).collect();

        let mut graphics_queue=vk::Queue::null();
        let mut present_queue=vk::Queue::null();

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
            instance.create_device(physical_device,&device_create_info,allocation_callbacks)
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
        
        let present_queue_command_pool_create_info=vk::CommandPoolCreateInfo{
            queue_family_index:present_queue_family_index,
            ..Default::default()
        };
        let graphics_queue_command_pool_create_info=vk::CommandPoolCreateInfo{
            queue_family_index:graphics_queue_family_index,
            ..Default::default()
        };

        let present_queue_command_pool=unsafe{
            device.create_command_pool(&present_queue_command_pool_create_info,allocation_callbacks)
        }.unwrap();
        let graphics_queue_command_pool=unsafe{
            device.create_command_pool(&graphics_queue_command_pool_create_info,allocation_callbacks)
        }.unwrap();

        Self{
            handle,
            open_windows,
            entry,
            allocation_callbacks,
            instance,
            physical_device,
            device,
            surface,
            present_queue,
            present_queue_family_index,
            present_queue_command_pool,
            graphics_queue,
            graphics_queue_family_index,
            graphics_queue_command_pool,
        }
    }

    pub fn create_semaphore(&self)->VkResult<vk::Semaphore>{
        let semaphore_create_info=vk::SemaphoreCreateInfo{
            ..Default::default()
        };
        unsafe{
            self.device.create_semaphore(&semaphore_create_info,self.allocation_callbacks)
        }
    }

    pub fn new_window(&mut self,width:u16,height:u16,title:&str){
        let surface;
        let handle={
            match &self.handle{
                #[cfg(target_os="windows")]
                WindowManagerHandle::Windows{hinstance,class_name}=>{
                    let window_hwnd:HWND=unsafe{
                        CreateWindowExA(
                            0,
                            class_name.as_str().as_ptr() as LPCSTR,
                            title.as_ptr() as LPCSTR,
                            WS_OVERLAPPEDWINDOW,
                            0,
                            0,
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
                        win32_surface.create_win32_surface(&surface_create_info,self.allocation_callbacks)
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
                            0,
                            0,
                            width,
                            height,
                            10,
                            xproto::XCB_WINDOW_CLASS_INPUT_OUTPUT as u16,
                            (*screen).root_visual,
                            mask,
                            values.as_ptr()
                        )
                    };

                    self.handle.xcb_check_cookie(&create_window_cookie,"create window");

                    unsafe{
                        base::xcb_flush(*connection)
                    };

                    //set window decorations (?)
                    let window_type_atom=self.handle.get_intern_atom("_NET_WM_WINDOW_TYPE\0");
                    let window_type_normal_atom=self.handle.get_intern_atom("_NET_WM_WINDOW_TYPE_NORMAL\0");

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
                    self.handle.xcb_check_cookie(&window_type_cookie,"change property");

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
                            let atom=self.handle.get_intern_atom(prop_name);
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
                        self.handle.xcb_check_cookie(&window_type_cookie,"change property");
                    }

                    let close=self.handle.get_intern_atom("WM_DELETE_WINDOW\0");//_NET_CLOSE_WINDOW
                    let hidden=self.handle.get_intern_atom("_NET_WM_STATE_HIDDEN\0");
                    let maximized_vertical=self.handle.get_intern_atom("_NET_WM_STATE_MAXIMIZED_VERT\0");
                    let maximized_horizontal=self.handle.get_intern_atom("_NET_WM_STATE_MAXIMIZED_HORZ\0");

                    let cookie=unsafe{
                        xcb_change_property_checked ( 
                            *connection,
                            XCB_PROP_MODE_REPLACE as u8,
                            window,
                            self.handle.get_intern_atom("WM_PROTOCOLS\0"), //property
                            xproto::XCB_ATOM_ATOM, //type
                            32,//format (8,16 or 32 bits per entry in value list)
                            1, //length of value list
                            &close as *const xcb_atom_t as *const libc::c_void // is this is a motif hints struct
                        )
                    };
                    self.handle.xcb_check_cookie(&window_type_cookie,"change property");

                    let map_window_cookie=unsafe{
                        xcb_map_window(*connection,window)
                    };
                    self.handle.xcb_check_cookie(&map_window_cookie,"map window");

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
                        xcb_surface.create_xcb_surface(&surface_create_info,self.allocation_callbacks)
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
            clipped:0u32,//false, but in c
            ..Default::default()
        };
        println!("min swapchain image count: {}",swapchain_create_info.min_image_count);
        let swapchain=extensions::khr::Swapchain::new(&self.instance,&self.device);
        let swapchain_handle=unsafe{
            swapchain.create_swapchain(&swapchain_create_info, self.allocation_callbacks)
        }.unwrap();

        //images may only be created when this function is valled, so presenting an image index before
        //this function is called violate the specs (the image may not exist yet)
        let swapchain_images=unsafe{
            swapchain.get_swapchain_images(swapchain_handle)
        }.unwrap();

        let window=Window{
            handle,
            surface,
            image_available,
            image_presentable,
            swapchain,
            swapchain_handle,
            swapchain_images,
        };
        self.open_windows.push(window);
    }
    fn destroy_window(&mut self,open_window_index:usize){
        let window=&mut self.open_windows[open_window_index];
        unsafe{
            self.device.destroy_semaphore(window.image_available, self.allocation_callbacks);
            self.device.destroy_semaphore(window.image_presentable, self.allocation_callbacks);
            window.swapchain.destroy_swapchain(window.swapchain_handle,self.allocation_callbacks);
            self.surface.destroy_surface(window.surface,self.allocation_callbacks)
        };
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
        match self.handle{
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
                    println!("handling event");
                    let response_type=unsafe{*generic_event}.response_type & 0x7f;
                    match response_type{
                        xproto::XCB_KEY_PRESS=>{
                            println!("key pressed");
                        },
                        xproto::XCB_KEY_RELEASE=>{
                            println!("key pressed");
                        },
                        xproto::XCB_CLIENT_MESSAGE=>{
                            println!("client message");
                            let event=generic_event as *const xproto::xcb_client_message_event_t;
                            match self.open_windows[0].handle{
                                WindowHandle::Xcb{close,..}=>{
                                    if unsafe{*event}.data.data32()[0]==close{
                                        println!("close window");
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

        //render here

        let window=&mut self.open_windows[0];

        let (image_index,suboptimal)=unsafe{
            window.swapchain.acquire_next_image(window.swapchain_handle, u64::MAX, window.image_available, vk::Fence::null())
        }.unwrap();

        //begin transition command buffer 1
        //cmd pipeline barrier 1
        //end transition command buffer 1
        //submit transition command buffer 1
        //begin transition command buffer 2
        //cmd pipeline barrier 2
        //end transition command buffer 2
        //submit transition command buffer 2

        let present_wait_semaphores=vec![
            window.image_available,
        ];
        let mut present_results=vec![
            vk::Result::SUCCESS,
        ];
        let present_info=vk::PresentInfoKHR{
            wait_semaphore_count:present_wait_semaphores.len() as u32,
            p_wait_semaphores:present_wait_semaphores.as_ptr(),
            swapchain_count:1,
            p_swapchains:&window.swapchain_handle,
            p_image_indices:&image_index,
            p_results:present_results.as_mut_ptr(),
            ..Default::default()
        };
        unsafe{
            window.swapchain.queue_present(self.present_queue,&present_info)
        }.unwrap();

        ControlFlow::Continue
    }
}
impl Drop for WindowManager<'_>{
    fn drop(&mut self){
        for open_window_index in 0..self.open_windows.len(){
            self.destroy_window(open_window_index);
        }

        unsafe{
            self.device.destroy_device(self.allocation_callbacks);
            self.instance.destroy_instance(self.allocation_callbacks)
        };

        self.handle.destroy();
    }
}

struct Manager<'m>{
    window_manager: WindowManager<'m>,
}
impl Manager<'_>{
    pub fn new()->Self{
        Self{
            window_manager:WindowManager::new()
        }
    }
    pub fn step(&mut self)->ControlFlow{
        self.window_manager.step()
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

fn main() {
    let mut manager=Manager::new();
    manager.window_manager.new_window(600,400,"hello milena");
    manager.run();
}
