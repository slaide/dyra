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

use crate::{Object};

pub struct GraphicsPipeline{
    layout:vk::PipelineLayout,
    pipeline:vk::Pipeline,

    vertex:vk::ShaderModule,
    fragment:vk::ShaderModule,
    
    pub descriptor_pool:vk::DescriptorPool,
    pub descriptor_set_layout:vk::DescriptorSetLayout,
}
//data for a specific shader
pub struct ShaderData{
    pub descriptor_set:vk::DescriptorSet,
    
    pub sampler:vk::Sampler,
}
pub struct RenderPass{
    pub render_pass:vk::RenderPass,

    //color information
    pub color_image:vk::Image,
    pub color_image_view:vk::ImageView,
    //depth information
    pub depth_image:vk::Image,
    pub depth_image_view:vk::ImageView,

    //shader stuff
    pub pipelines:std::collections::HashMap<String,GraphicsPipeline>,

    //rendering from this pass done
    pub rendering_done:vk::Semaphore,
}
impl RenderPass{
    pub fn new()->Self{
        Self{
            render_pass:vk::RenderPass::null(),

            color_image:vk::Image::null(),
            color_image_view:vk::ImageView::null(),

            depth_image:vk::Image::null(),
            depth_image_view:vk::ImageView::null(),

            pipelines:std::collections::HashMap::new(),

            rendering_done:vk::Semaphore::null(),
        }
    }
}

pub struct WindowAttachments{
    pub surface:vk::SurfaceKHR,

    pub image_available:vk::Semaphore,
    pub image_transferable:vk::Semaphore,
    pub image_presentable:vk::Semaphore,
    
    pub swapchain:extensions::khr::Swapchain,
    pub swapchain_handle:vk::SwapchainKHR,
    pub swapchain_images:Vec<vk::Image>,
    pub swapchain_image_views:Vec<vk::ImageView>,
    pub swapchain_image_framebuffers:Vec<vk::Framebuffer>,
}

pub struct Painter{
    pub allocation_callbacks:Option<vk::AllocationCallbacks>,

    pub device:Device,

    pub present_queue:std::sync::Arc<crate::Queue>,

    pub swapchain_surface_format:vk::SurfaceFormatKHR,

    pub window_attachments:std::collections::HashMap<u32,WindowAttachments>,

    pub graphics_queue:std::sync::Arc<crate::Queue>,

    pub render_pass_2d:RenderPass,
    pub render_pass_3d:RenderPass,
}
/*
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
*/

impl Painter{
    fn get_allocation_callbacks(&self)->Option<&vk::AllocationCallbacks>{
        self.allocation_callbacks.as_ref()
    }
    #[cfg(disabled)]
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
        let clear_values=vec![
            vk::ClearValue{
                color:vk::ClearColorValue{
                    float32:[0.9,0.5,0.2,1.0],
                },
            },
            vk::ClearValue{
                depth_stencil:vk::ClearDepthStencilValue{
                    depth:1.0,
                    stencil:0,
                },
            },
        ];
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
            clear_value_count:clear_values.len() as u32,
            p_clear_values:clear_values.as_ptr(),
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
        let eye=glm::vec3(0.0,-1.0,2.0);
        let target=glm::vec3(0.0,-1.0,0.0);
        let view=glm::look_at(&eye,&target,&glm::vec3(0.0,1.0,0.0));

        let (scale_x,scale_y,scale_z)=(0.25,0.25,0.25);
        let (rotate_x,rotate_y,rotate_z)=(180.0,170.0,0.0);
        let (translate_x,translate_y,translate_z)=(0.0,0.0,0.0);

        let model=glm::translate(
            &glm::scale(
                &glm::rotate_x(
                    &glm::rotate_y(            
                        &glm::rotate_z(
                            &glm::identity::<f32,4>(),
                            glm::radians(&glm::vec1(rotate_z)).x,
                        ),
                        glm::radians(&glm::vec1(rotate_y)).x,
                    ),
                    glm::radians(&glm::vec1(rotate_x)).x,
                ),
                &glm::vec3(scale_x,scale_y,scale_z)
            ),
            &glm::vec3(translate_x,translate_y,translate_z)
        );

        let projection=glm::perspective_fov(
            glm::radians(&glm::vec1(80.0)).x,
            window_extent.width as f32,
            window_extent.height as f32,
            0.001,
            1000.0
        );

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

    #[cfg(disabled)]
    pub fn add_window(&mut self,window_id:u32){
        
                  
        //vulkan spec states this must be done
        if unsafe{
            !self.surface.get_physical_device_surface_support(self.physical_device, self.painter.present_queue.family_index, surface).unwrap()
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
            self.painter.present_queue.family_index,
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

        let swapchain_image_framebuffers:Vec<vk::Framebuffer>=Vec::new();/*swapchain_image_views.iter().map(|view|{
            let attachments=vec![
                *view,
            ];
            let framebuffer_create_info=vk::FramebufferCreateInfo{
                render_pass:self.painter.render_pass,
                attachment_count:attachments.len() as u32,
                p_attachments:attachments.as_ptr(),
                width:swapchain_extent.width,
                height:swapchain_extent.height,
                layers:1,
                ..Default::default()
            };
            unsafe{
                self.device.create_framebuffer(&framebuffer_create_info, self.get_allocation_callbacks())
            }.unwrap()
        }).collect();
        */
        
        let _=WindowAttachments{

        };
    }
}