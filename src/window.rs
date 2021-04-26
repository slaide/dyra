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

pub enum WindowHandle{
    #[cfg(target_os="windows")]
    Windows{
        hwnd:HWND,
        #[allow(dead_code)]
        win32_surface:extensions::khr::Win32Surface,
    },
    #[cfg(target_os="linux")]
    Xcb{
        connection:*mut base::xcb_connection_t,
        visual:xproto::xcb_visualid_t,
        window:u32,
        #[allow(dead_code)]
        xcb_surface:ash::extensions::khr::XcbSurface,
        close:xcb_atom_t,
        maximized_horizontal:xcb_atom_t,
        maximized_vertical:xcb_atom_t,
        hidden:xcb_atom_t,
    },
    #[allow(dead_code)]
    NeverMatch
}
pub struct Window{
    pub id:u32,

    pub extent:vk::Extent2D,

    pub handle:WindowHandle,

    pub surface:vk::SurfaceKHR,
}