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


pub struct Painter{
    pub allocation_callbacks:Option<vk::AllocationCallbacks>,

    #[allow(dead_code)]
    pub instance:Instance,
    pub device:Device,

    pub swapchain_surface_format:vk::SurfaceFormatKHR,

    pub sampler:vk::Sampler,

    pub descriptor_set_layout:vk::DescriptorSetLayout,//list of descriptor types ("descriptorSetLayoutBindings"), the shader stages they are used in and their type
    pub descriptor_set:vk::DescriptorSet,//contains handles to descriptors of types specified in layout
    pub descriptor_pool:vk::DescriptorPool,//allocate descriptors

    pub render_pass:vk::RenderPass,

    pub graphics_pipeline_layout:vk::PipelineLayout,
    pub graphics_pipeline:vk::Pipeline,

    pub rendering_done:vk::Semaphore,

    pub graphics_queue:vk::Queue,
    pub graphics_queue_family_index:u32,
    pub graphics_queue_command_pool:vk::CommandPool,
    pub graphics_queue_command_buffers:Vec<vk::CommandBuffer>,
}
impl Drop for Painter{
    fn drop(&mut self){
        unsafe{
            self.device.destroy_sampler(self.sampler,self.get_allocation_callbacks());

            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, self.get_allocation_callbacks());

            self.device.destroy_descriptor_pool(self.descriptor_pool, self.get_allocation_callbacks());

            self.device.destroy_pipeline(self.graphics_pipeline, self.get_allocation_callbacks());
            
            self.device.destroy_pipeline_layout(self.graphics_pipeline_layout, self.get_allocation_callbacks());

            self.device.destroy_render_pass(self.render_pass, self.get_allocation_callbacks());

            self.device.destroy_semaphore(self.rendering_done,self.get_allocation_callbacks());

            self.device.destroy_command_pool(self.graphics_queue_command_pool, self.get_allocation_callbacks());
        }
    }
}
impl Painter{
    fn get_allocation_callbacks(&self)->Option<&vk::AllocationCallbacks>{
        self.allocation_callbacks.as_ref()
    }
}