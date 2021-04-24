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

use crate::WindowManagerHandle;

pub enum TestWindowHandle{
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
pub struct TestWindow<'a>{
    pub handle:TestWindowHandle,
    pub surface:extensions::khr::Surface,
    pub platform_surface:vk::SurfaceKHR,
    pub allocation_callbacks:Option<&'a vk::AllocationCallbacks>,
}
impl TestWindow<'_>{
    pub fn new<'a>(window_manager_handle:&WindowManagerHandle, entry: &Entry, instance:&Instance, allocation_callbacks:Option<&'a vk::AllocationCallbacks>)->TestWindow<'a>{
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
                
                let surface_create_info=vk::Win32SurfaceCreateInfoKHR{
                    hinstance:*hinstance as *const libc::c_void,
                    hwnd:window_hwnd as *const libc::c_void,
                    ..Default::default()
                };
                let platform_surface=unsafe{
                    win32_surface.create_win32_surface(&surface_create_info,allocation_callbacks)
                }.unwrap();

                let surface=extensions::khr::Surface::new(entry,instance);

                TestWindow::<'_>{
                    handle:TestWindowHandle::Windows{
                        hwnd:window_hwnd,
                        win32_surface,
                    },
                    surface,
                    platform_surface,
                    allocation_callbacks
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

                let surface_create_info=vk::XcbSurfaceCreateInfoKHR{
                    connection:*connection as *mut libc::c_void,
                    window:window,
                    ..Default::default()
                };
                let platform_surface=unsafe{
                    xcb_surface.create_xcb_surface(&surface_create_info,allocation_callbacks)
                }.unwrap();

                let surface=extensions::khr::Surface::new(entry,instance);

                TestWindow{
                    handle:TestWindowHandle::Xcb{
                        connection:*connection,
                        visual,
                        window,
                        xcb_surface,
                    },
                    surface,
                    platform_surface,
                    allocation_callbacks
                }
            },
            _=>unimplemented!()
        }
    }
}
impl Drop for TestWindow<'_>{
    fn drop(&mut self){
        unsafe{
            self.surface.destroy_surface(self.platform_surface,self.allocation_callbacks)
        }
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