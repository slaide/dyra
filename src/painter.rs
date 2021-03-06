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

use crate::{Object,GraphicsPipeline};

pub struct Painter{
    pub allocation_callbacks:Option<vk::AllocationCallbacks>,

    pub device:Device,

    pub swapchain_surface_format:vk::SurfaceFormatKHR,

    pub sampler:vk::Sampler,

    pub descriptor_set_layout:vk::DescriptorSetLayout,//list of descriptor types ("descriptorSetLayoutBindings"), the shader stages they are used in and their type
    pub descriptor_set:vk::DescriptorSet,//contains handles to descriptors of types specified in layout
    pub descriptor_pool:vk::DescriptorPool,//allocate descriptors

    pub render_pass:vk::RenderPass,

    pub graphics_pipeline_2d:GraphicsPipeline,
    pub graphics_pipeline_3d:GraphicsPipeline,

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

            for pipeline in &[&self.graphics_pipeline_2d,&self.graphics_pipeline_3d]{
                self.device.destroy_pipeline(pipeline.pipeline, self.get_allocation_callbacks());
                self.device.destroy_pipeline_layout(pipeline.layout, self.get_allocation_callbacks());
                
                self.device.destroy_shader_module(pipeline.vertex,self.get_allocation_callbacks());
                self.device.destroy_shader_module(pipeline.fragment,self.get_allocation_callbacks());
            }

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
    pub fn draw(&mut self,framebuffer:vk::Framebuffer,window_extent:vk::Extent2D,objects:&Vec<Object>,done:vk::Semaphore){
        //record graphics command buffer
        //begin
        /*
        //for now, begin outside because the manager handles resource upload using this queue, which needs to happen before the actual drawing, but after the 'begin' command
        let graphics_queue_command_buffer_begin_info=vk::CommandBufferBeginInfo{
            flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        unsafe{
            self.device.begin_command_buffer(self.graphics_queue_command_buffers[0], &graphics_queue_command_buffer_begin_info)
        }.unwrap();

        //barrier from top to transfer
        unsafe{
            self.device.cmd_pipeline_barrier(self.graphics_queue_command_buffers[0],vk::PipelineStageFlags::TOP_OF_PIPE,vk::PipelineStageFlags::TRANSFER,vk::DependencyFlags::empty(),&[],&[],&[])
        };
        */

        //render quad
        //begin render pass
        let clear_value=vk::ClearValue{
            color:vk::ClearColorValue{
                float32:[0.9,0.5,0.2,1.0],
            },
        };
        let render_pass_begin_info=vk::RenderPassBeginInfo{
            render_pass:self.render_pass,
            framebuffer:framebuffer,
            render_area:vk::Rect2D{
                offset:vk::Offset2D{
                    x:0,
                    y:0,
                },
                extent:window_extent
            },
            clear_value_count:1,
            p_clear_values:&clear_value,
            ..Default::default()
        };
        unsafe{
            self.device.cmd_begin_render_pass(self.graphics_queue_command_buffers[0], &render_pass_begin_info, vk::SubpassContents::INLINE)
        };
        //bind pipeline 2d
        unsafe{
            self.device.cmd_bind_pipeline(self.graphics_queue_command_buffers[0],vk::PipelineBindPoint::GRAPHICS,self.graphics_pipeline_2d.pipeline);
        }
        let viewport=vk::Viewport{
            x:0.0,
            y:0.0,
            width:window_extent.width as f32,
            height:window_extent.height as f32,
            min_depth:0.0,
            max_depth:1.0,
        };
        let scissor=vk::Rect2D{
            offset:vk::Offset2D{
                x:0,
                y:0,  
            },
            extent:vk::Extent2D{
                width:window_extent.width,
                height:window_extent.height,
            }
        };
        unsafe{
            self.device.cmd_set_viewport(self.graphics_queue_command_buffers[0],0,&[viewport]);
            self.device.cmd_set_scissor(self.graphics_queue_command_buffers[0],0,&[scissor]);
        }
        //bind descriptor set for fragment shader
        unsafe{
            self.device.cmd_bind_descriptor_sets(self.graphics_queue_command_buffers[0], vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline_2d.layout, 0, &[self.descriptor_set], &[]);
        }
        //draw
        for obj in objects.iter(){
            unsafe{
                self.device.cmd_bind_vertex_buffers(self.graphics_queue_command_buffers[0],0,&[obj.mesh.vertices.buffer],&[0]);
                self.device.cmd_bind_index_buffer(self.graphics_queue_command_buffers[0],obj.mesh.vertex_indices.buffer,0,vk::IndexType::UINT16);
                //self.device.cmd_draw_indexed(self.graphics_queue_command_buffers[0],obj.mesh.vertex_indices.item_count as u32,1,0,0,0);
            }
        }

        //bind pipeline 3d
        unsafe{
            self.device.cmd_bind_pipeline(self.graphics_queue_command_buffers[0],vk::PipelineBindPoint::GRAPHICS,self.graphics_pipeline_3d.pipeline);
        }
        unsafe{
            self.device.cmd_set_viewport(self.graphics_queue_command_buffers[0],0,&[viewport]);
            self.device.cmd_set_scissor(self.graphics_queue_command_buffers[0],0,&[scissor]);
        }
        //push constants
        let eye=glm::vec3(2.0,2.0,2.0);
        let target=glm::vec3(0.0,0.0,0.0);
        let view=glm::look_at(&eye,&target,&glm::vec3(0.0,1.0,0.0));

        let model=glm::rotate_x(&glm::identity::<f32,4>(),0.4);

        let projection=glm::perspective_fov(glm::radians(&glm::vec1(80.0)).x,2.0,2.0,0.001,1000.0);

        //bind descriptor set for fragment shader
        unsafe{
            self.device.cmd_bind_descriptor_sets(self.graphics_queue_command_buffers[0], vk::PipelineBindPoint::GRAPHICS, self.graphics_pipeline_3d.layout, 0, &[self.descriptor_set], &[]);
        }
        unsafe{
            self.device.cmd_push_constants(
                self.graphics_queue_command_buffers[0],
                self.graphics_pipeline_3d.layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                std::mem::transmute::<&[glm::Mat4;3], &[u8;16*4*3]>(&[model,view,projection]),
            );
        }
        //draw
        for obj in objects.iter(){
            unsafe{
                self.device.cmd_bind_vertex_buffers(self.graphics_queue_command_buffers[0],0,&[obj.mesh.vertices.buffer],&[0]);
                self.device.cmd_bind_index_buffer(self.graphics_queue_command_buffers[0],obj.mesh.vertex_indices.buffer,0,vk::IndexType::UINT16);
                self.device.cmd_draw_indexed(self.graphics_queue_command_buffers[0],obj.mesh.vertex_indices.item_count as u32,1,0,0,0);
            }
        }
        //end render pass
        unsafe{
            self.device.cmd_end_render_pass(self.graphics_queue_command_buffers[0])
        };
        //end
        unsafe{
            self.device.end_command_buffer(self.graphics_queue_command_buffers[0])
        }.unwrap();
        //submit
        let wait_semaphores_graphics=vec![
            done,
        ];
        let dst_stage_masks_graphics=vec![
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ];
        let command_buffers_graphics=vec![
            self.graphics_queue_command_buffers[0]
        ];
        let signal_semaphores=vec![
            self.rendering_done
        ];
        let submit_info_graphics=vk::SubmitInfo{
            wait_semaphore_count:wait_semaphores_graphics.len() as u32,
            p_wait_semaphores:wait_semaphores_graphics.as_ptr(),
            p_wait_dst_stage_mask:dst_stage_masks_graphics.as_ptr(),
            command_buffer_count:command_buffers_graphics.len() as u32,
            p_command_buffers:command_buffers_graphics.as_ptr(),
            signal_semaphore_count:signal_semaphores.len() as u32,
            p_signal_semaphores:signal_semaphores.as_ptr(),
            ..Default::default()
        };
        unsafe{
            self.device.queue_submit(self.graphics_queue,&[submit_info_graphics],vk::Fence::null())
        }.unwrap();
    }
}