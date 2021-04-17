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

};

extern crate libc;
extern crate ash;
use ash::{
    vk,
    vk::{
        AllocationCallbacks,
        PhysicalDevice,
    },
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

#[derive(PartialEq,Debug)]
enum ControlFlow{
    Continue,
    Stop,
}
#[derive(PartialEq,Debug)]
enum Event{
    WindowCloseRequested,
    #[allow(dead_code)]
    None,
}

enum WindowHandle{
    #[cfg(target_os="windows")]
    Windows{
        hwnd:HWND,
        win32_surface:extensions::khr::Win32Surface,
    },
    #[cfg(target_os="linux")]
    Xcb{
        connection:something,
        windiw:i32
    },
    #[allow(dead_code)]
    NeverMatch
}
struct Window{
    handle:WindowHandle,
    surface:vk::SurfaceKHR,
}
struct TestWindow{
    handle:WindowHandle,
}
impl TestWindow{
    fn new(window_manager_handle:&WindowManagerHandle, entry: &Entry, instance:&Instance)->TestWindow{
        match &window_manager_handle{
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
                    handle:WindowHandle::Windows{
                        hwnd:window_hwnd,
                        win32_surface,
                    },
                }
            },
            /*
                let surface_create_info=vk::XcbSurfaceCreateInfoKHR{
                    connection,
                    window,
                    ..Default::default()
                };
                let surface=self.instance.create_xcb_surface(&surface_create_info,self.allocation_callbacks).unwrap();
            */
            _=>unimplemented!()
        }
    }
}
impl Drop for TestWindow{
    fn drop(&mut self){//,window_manager_handle:&WindowManagerHandle, entry: &mut Entry, instance:&mut Instance){
        //let surface=ash::extensions::khr::Surface::new(&self.entry,&self.instance);
        match self.handle{
            WindowHandle::Windows{hwnd,..}=>{
                unsafe{
                    DestroyWindow(hwnd);
                }
            },
            _=>unreachable!()
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

enum WindowManagerHandle{
    #[cfg(target_os="windows")]
    Windows{
        hinstance:HINSTANCE,
        class_name:String,
    },
    #[cfg(target_os="linux")]
    Xcb{

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
            unimplemented!()
        }
    }
    pub fn destroy(&mut self){

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
                        WindowHandle::Windows{hwnd:_,win32_surface}=>{
                            unsafe{
                                win32_surface.get_physical_device_win32_presentation_support(**pd,i as u32)
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

        Self{
            handle,
            open_windows,
            entry,
            allocation_callbacks,
            instance,
            physical_device,
            device,
            surface,
        }
    }

    pub fn new_window(&mut self,width:u16,height:u16){
        let surface;
        let handle={
            match &self.handle{
                WindowManagerHandle::Windows{hinstance,class_name}=>{
                    let window_hwnd:HWND=unsafe{
                        CreateWindowExA(
                            0,
                            class_name.as_str().as_ptr() as LPCSTR,
                            "my window".as_ptr() as LPCSTR,
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
                _=>unimplemented!()
            }
        };
        let window=Window{
            handle,
            surface,
        };
        self.open_windows.push(window);
    }
    fn destroy_window(&mut self,open_window_index:usize){
        unsafe{
            self.surface.destroy_surface(self.open_windows[open_window_index].surface,self.allocation_callbacks)
        };
        match self.open_windows[open_window_index].handle{
            WindowHandle::Windows{hwnd,..}=>{
                unsafe{
                    DestroyWindow(hwnd);
                }
            },
            _=>unreachable!()
        }
    }

    pub fn step(&mut self)->ControlFlow{
        match self.handle{
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
            _=>panic!("unsupported")
        }
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

            //cap framerate at 30fps
            std::thread::sleep(std::time::Duration::from_millis(1000/30));

            println!("step done");
        }
    }
}

fn main() {
    let mut manager=Manager::new();
    manager.window_manager.new_window(600,400);
    manager.run();
}
