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
    set_bindings:Vec<Vec<vk::DescriptorSetLayoutBinding>>,

    layout:vk::PipelineLayout,
    pipeline:vk::Pipeline,

    vertex:vk::ShaderModule,
    fragment:vk::ShaderModule,
    
    pub descriptor_pool:vk::DescriptorPool,
    pub descriptor_set_layouts:Vec<vk::DescriptorSetLayout>,
}
//data for a specific shader
pub struct ShaderData{
    pub descriptor_set:vk::DescriptorSet,
    
    pub sampler:vk::Sampler,
}
pub struct RenderPass{
    pub vulkan:std::sync::Arc<crate::VulkanBase>,

    pub render_pass:vk::RenderPass,

    pub extent:vk::Extent2D,

    //color information
    pub color_image_format:vk::Format,
    pub color_image_layout:vk::ImageLayout,
    pub color_image:vk::Image,
    pub color_image_memory:vk::DeviceMemory,
    pub color_image_view:vk::ImageView,
    //depth information
    pub depth_image_format:vk::Format,
    pub depth_image_layout:vk::ImageLayout,
    pub depth_image:vk::Image,
    pub depth_image_memory:vk::DeviceMemory,
    pub depth_image_view:vk::ImageView,

    pub framebuffer:vk::Framebuffer,

    //shader stuff
    pub pipelines:std::collections::HashMap<String,std::sync::Arc<GraphicsPipeline>>,

    //rendering from this pass done
    pub rendering_done:vk::Semaphore,
}
impl RenderPass{
    pub fn new(vulkan:&std::sync::Arc<crate::VulkanBase>,surface:&extensions::khr::Surface,window:&crate::Window)->Self{
        let surface_formats=unsafe{
            surface.get_physical_device_surface_formats(vulkan.physical_device,window.surface)
        }.unwrap();
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

        let depth_image_format=&[vk::Format::D32_SFLOAT,vk::Format::D32_SFLOAT_S8_UINT,vk::Format::D24_UNORM_S8_UINT].iter().find(|depth_format|{
            unsafe{
                vulkan.instance.get_physical_device_format_properties(vulkan.physical_device,**depth_format)
            }.optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        }).unwrap();
        let depth_image_format=**depth_image_format;

        let color_image_format=surface_format.format;
        
        let color_image_layout=vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
        let depth_image_layout=vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL;
        let depth_image_layout_has_stencil_component=depth_image_format!=vk::Format::D32_SFLOAT;

        let color_image_create_info=vk::ImageCreateInfo{
            image_type:vk::ImageType::TYPE_2D,
            format:color_image_format,
            extent:vk::Extent3D{
                height:window.extent.height,
                width:window.extent.width,
                depth:1
            },
            mip_levels:1,
            array_layers:1,
            samples:vk::SampleCountFlags::TYPE_1,
            tiling:if unsafe{
                vulkan.instance.get_physical_device_format_properties(
                    vulkan.physical_device,color_image_format
                ).optimal_tiling_features.contains(vk::FormatFeatureFlags::COLOR_ATTACHMENT)
            }{
                vk::ImageTiling::OPTIMAL
            }else{
                vk::ImageTiling::LINEAR
            },
            //render color to, and copy to swapchain
            usage:vk::ImageUsageFlags::COLOR_ATTACHMENT|vk::ImageUsageFlags::TRANSFER_SRC,
            sharing_mode:vk::SharingMode::EXCLUSIVE,
            initial_layout:vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };
        let color_image=unsafe{
            vulkan.device.create_image(&color_image_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();
        let color_image_memory={
            let color_image_memory_requirements=unsafe{
                vulkan.device.get_image_memory_requirements(color_image)
            };
            let memory_types=unsafe{
                vulkan.instance.get_physical_device_memory_properties(vulkan.physical_device).memory_types
            };
            let memory_allocate_info=vk::MemoryAllocateInfo{
                allocation_size:color_image_memory_requirements.size,
                memory_type_index:memory_types.iter().enumerate().find(|(index,memory_type)|{
                    ((1u32 << *index) & color_image_memory_requirements.memory_type_bits)>0 
                    && memory_type.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                }).unwrap().0 as u32,
                ..Default::default()
            };
            unsafe{
                vulkan.device.allocate_memory(&memory_allocate_info,vulkan.get_allocation_callbacks())
            }
        }.unwrap();
        unsafe{
            vulkan.device.bind_image_memory(color_image,color_image_memory,0)
        }.unwrap();
        let color_image_view_create_info=vk::ImageViewCreateInfo{
            image:color_image,
            view_type:vk::ImageViewType::TYPE_2D,
            format:color_image_format,
            components:vk::ComponentMapping{
                r:vk::ComponentSwizzle::IDENTITY,
                g:vk::ComponentSwizzle::IDENTITY,
                b:vk::ComponentSwizzle::IDENTITY,
                a:vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range:vk::ImageSubresourceRange{
                aspect_mask:vk::ImageAspectFlags::COLOR,
                base_mip_level:0,
                level_count:1,
                base_array_layer:0,
                layer_count:1,
            },
            ..Default::default()
        };
        let color_image_view=unsafe{
            vulkan.device.create_image_view(&color_image_view_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();

        let depth_image_create_info=vk::ImageCreateInfo{
            image_type:vk::ImageType::TYPE_2D,
            format:depth_image_format,
            extent:vk::Extent3D{
                height:window.extent.height,
                width:window.extent.width,
                depth:1
            },
            mip_levels:1,
            array_layers:1,
            samples:vk::SampleCountFlags::TYPE_1,
            tiling:if unsafe{
                vulkan.instance.get_physical_device_format_properties(
                    vulkan.physical_device,depth_image_format
                ).optimal_tiling_features.contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
            }{
                vk::ImageTiling::OPTIMAL
            }else{
                vk::ImageTiling::LINEAR
            },
            usage:vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            sharing_mode:vk::SharingMode::EXCLUSIVE,
            initial_layout:vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };
        let depth_image=unsafe{
            vulkan.device.create_image(&depth_image_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();
        let depth_image_memory={
            let depth_image_memory_requirements=unsafe{
                vulkan.device.get_image_memory_requirements(depth_image)
            };
            let memory_types=unsafe{
                vulkan.instance.get_physical_device_memory_properties(vulkan.physical_device).memory_types
            };
            let memory_allocate_info=vk::MemoryAllocateInfo{
                allocation_size:depth_image_memory_requirements.size,
                memory_type_index:memory_types.iter().enumerate().find(|(index,memory_type)|{
                    ((1u32 << *index) & depth_image_memory_requirements.memory_type_bits)>0 
                    && memory_type.property_flags.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL)
                }).unwrap().0 as u32,
                ..Default::default()
            };
            unsafe{
                vulkan.device.allocate_memory(&memory_allocate_info,vulkan.get_allocation_callbacks())
            }
        }.unwrap();
        unsafe{
            vulkan.device.bind_image_memory(depth_image,depth_image_memory,0)
        }.unwrap();
        let depth_image_view_create_info=vk::ImageViewCreateInfo{
            image:depth_image,
            view_type:vk::ImageViewType::TYPE_2D,
            format:depth_image_format,
            components:vk::ComponentMapping{
                r:vk::ComponentSwizzle::IDENTITY,
                g:vk::ComponentSwizzle::IDENTITY,
                b:vk::ComponentSwizzle::IDENTITY,
                a:vk::ComponentSwizzle::IDENTITY,
            },
            subresource_range:vk::ImageSubresourceRange{
                aspect_mask:if depth_image_layout_has_stencil_component{
                    vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
                }else{
                    vk::ImageAspectFlags::DEPTH
                },
                base_mip_level:0,
                level_count:1,
                base_array_layer:0,
                layer_count:1,
            },
            ..Default::default()
        };
        let depth_image_view=unsafe{
            vulkan.device.create_image_view(&depth_image_view_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();

        let attachments=vec![
            vk::AttachmentDescription{
                format:color_image_format,
                samples:vk::SampleCountFlags::TYPE_1,
                load_op:vk::AttachmentLoadOp::CLEAR,
                store_op:vk::AttachmentStoreOp::STORE,
                initial_layout:vk::ImageLayout::UNDEFINED,
                final_layout:color_image_layout,
                ..Default::default()
            },
            vk::AttachmentDescription{
                format:depth_image_format,
                samples:vk::SampleCountFlags::TYPE_1,
                load_op:vk::AttachmentLoadOp::CLEAR,
                store_op:vk::AttachmentStoreOp::STORE,
                initial_layout:vk::ImageLayout::UNDEFINED,
                final_layout:depth_image_layout,
                ..Default::default()
            },
        ];
        let color_attachments=vec![
            vk::AttachmentReference{
                attachment:0,
                layout:color_image_layout,
            }
        ];
        let depth_stencil_attachment=vk::AttachmentReference{
            attachment:1,
            layout:depth_image_layout,
        };
        let subpasses=vec![
            vk::SubpassDescription{
                pipeline_bind_point:vk::PipelineBindPoint::GRAPHICS,
                color_attachment_count:color_attachments.len() as u32,
                p_color_attachments:color_attachments.as_ptr(),
                p_depth_stencil_attachment:&depth_stencil_attachment,
                ..Default::default()
            }
        ];
        let subpass_dependencies=vec![];
        let render_pass_create_info=vk::RenderPassCreateInfo{
            attachment_count:attachments.len() as u32,
            p_attachments:attachments.as_ptr(),
            subpass_count:subpasses.len() as u32,
            p_subpasses:subpasses.as_ptr(),
            dependency_count:subpass_dependencies.len() as u32,
            p_dependencies:subpass_dependencies.as_ptr(),
            ..Default::default()
        };
        let render_pass=unsafe{
            vulkan.device.create_render_pass(&render_pass_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();

        let framebuffer_attachments=vec![
            color_image_view,
            depth_image_view,
        ];
        let framebuffer_create_info=vk::FramebufferCreateInfo{
            render_pass,
            attachment_count:framebuffer_attachments.len() as u32,
            p_attachments:framebuffer_attachments.as_ptr(),
            width:window.extent.width,
            height:window.extent.height,
            layers:1,
            ..Default::default()
        };
        let framebuffer=unsafe{
            vulkan.device.create_framebuffer(&framebuffer_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();

        Self{
            vulkan:vulkan.clone(),

            render_pass,

            extent:window.extent,

            color_image_format,
            color_image_layout,
            color_image,
            color_image_memory,
            color_image_view,

            depth_image_format,
            depth_image_layout,
            depth_image,
            depth_image_memory,
            depth_image_view,

            framebuffer,

            pipelines:std::collections::HashMap::new(),

            rendering_done:vulkan.create_semaphore().unwrap(),
        }
    }

    pub fn new_graphics_pipeline(&mut self,filename:&str)->std::sync::Arc<GraphicsPipeline>{
        if let Some(pipeline)=self.pipelines.get(&String::from(filename)){
            return pipeline.clone();
        }

        use std::io::BufRead;
        let pipeline_file=std::fs::File::open(filename).unwrap();
        let pipeline_file_content=std::io::BufReader::new(pipeline_file);

        let mut lines=pipeline_file_content.lines();
        let attribute=lines.next().unwrap();
        assert!(attribute.unwrap()=="#version");

        let version=lines.next().unwrap().unwrap().parse::<u32>().unwrap();
        println!("parsing graphics pipeline of version {}",version);

        assert!(lines.next().unwrap().unwrap()=="#settings");

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="textured");
        let textured=line.next().unwrap()=="true";

        assert!(lines.next().unwrap().unwrap()=="#descriptor_set_pool");

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="max_set_count");//max number of copies of all descriptor sets
        let max_set_count=line.next().unwrap().parse::<u32>().unwrap();

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="single_set_count");
        let single_set_count=line.next().unwrap().parse::<u32>().unwrap();

        assert!(lines.next().unwrap().unwrap()=="#vertex_shader");

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="path");
        let vertex_shader_path=line.next().unwrap();

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="set_count");
        let vertex_shader_set_count=line.next().unwrap().parse::<u32>().unwrap();

        for i in 0..vertex_shader_set_count{
            //TODO parse this
        }

        assert!(lines.next().unwrap().unwrap()=="#fragment_shader");

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="path");
        let fragment_shader_path=line.next().unwrap();

        let line=lines.next().unwrap().unwrap();
        let mut line=line.split('=');
        let attribute=line.next().unwrap();
        assert!(attribute=="set_count");
        let fragment_shader_set_count=line.next().unwrap().parse::<u32>().unwrap();

        for i in 0..fragment_shader_set_count{
            //TODO parse this
            /*
            #fragment_shader_descriptor_set.0
            set_index=0
            binding_count=1
            binding.0=combined_image_sampler
            */
        }


        //create layout
        let set_bindings=vec![
            vec![
                vk::DescriptorSetLayoutBinding{
                    binding:0,//position of this descriptor in set
                    descriptor_type:vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    descriptor_count:1,//represented in shader as array of this length
                    stage_flags:vk::ShaderStageFlags::FRAGMENT,
                    ..Default::default()
                }
            ]
        ];
        let descriptor_set_layout_create_info=vk::DescriptorSetLayoutCreateInfo{
            binding_count:set_bindings[0].len() as u32,
            p_bindings:set_bindings[0].as_ptr(),
            ..Default::default()
        };
        let descriptor_set_layouts=vec![
            unsafe{
                self.vulkan.device.create_descriptor_set_layout(&descriptor_set_layout_create_info,self.vulkan.get_allocation_callbacks())
            }.unwrap()
        ];

        //allocate descriptor pool
        let descriptor_pool_sizes=vec![
            vk::DescriptorPoolSize{
                ty:vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count:1,
            }
        ];
        let descriptor_pool_create_info=vk::DescriptorPoolCreateInfo{
            max_sets:max_set_count,
            pool_size_count:descriptor_pool_sizes.len() as u32,
            p_pool_sizes:descriptor_pool_sizes.as_ptr(),
            ..Default::default()
        };
        let descriptor_pool=unsafe{
            self.vulkan.device.create_descriptor_pool(&descriptor_pool_create_info,self.vulkan.get_allocation_callbacks())
        }.unwrap();

        let push_constant_ranges=vec![];

        let graphics_pipeline_layout_create_info=vk::PipelineLayoutCreateInfo{
            set_layout_count:descriptor_set_layouts.len() as u32,
            p_set_layouts:descriptor_set_layouts.as_ptr(),
            push_constant_range_count:push_constant_ranges.len() as u32,
            p_push_constant_ranges:push_constant_ranges.as_ptr(),
            ..Default::default()
        };
        let graphics_pipeline_layout=unsafe{
            self.vulkan.device.create_pipeline_layout(&graphics_pipeline_layout_create_info,self.vulkan.get_allocation_callbacks())
        }.unwrap();

        use std::io::Read;
        
        let mut shader_code=Vec::new();
        let mut shader_file=std::fs::File::open(vertex_shader_path).unwrap();
        shader_file.read_to_end(&mut shader_code).unwrap();
        let shader_module_create_info=vk::ShaderModuleCreateInfo{
            code_size:shader_code.len(),
            p_code:shader_code.as_ptr() as *const u32,
            ..Default::default()
        };
        let vertex_shader=unsafe{
            self.vulkan.device.create_shader_module(&shader_module_create_info,self.vulkan.get_allocation_callbacks())
        }.unwrap();
        
        let mut shader_code=Vec::new();
        let mut shader_file=std::fs::File::open(fragment_shader_path).unwrap();
        shader_file.read_to_end(&mut shader_code).unwrap();
        let shader_module_create_info=vk::ShaderModuleCreateInfo{
            code_size:shader_code.len(),
            p_code:shader_code.as_ptr() as *const u32,
            ..Default::default()
        };
        let fragment_shader=unsafe{
            self.vulkan.device.create_shader_module(&shader_module_create_info,self.vulkan.get_allocation_callbacks())
        }.unwrap();

        let shader_stages=vec![
            vk::PipelineShaderStageCreateInfo{
                stage:vk::ShaderStageFlags::VERTEX,
                module:vertex_shader,
                p_name:"main\0".as_ptr() as *const i8,
                ..Default::default()
            },
            vk::PipelineShaderStageCreateInfo{
                stage:vk::ShaderStageFlags::FRAGMENT,
                module:fragment_shader,
                p_name:"main\0".as_ptr() as *const i8,
                ..Default::default()
            }
        ];

        let vertex_input_binding_descriptions=if textured{
            vec![
                vk::VertexInputBindingDescription{
                    binding:0,
                    stride:std::mem::size_of::<crate::decoder::TexturedVertex>() as u32,
                    input_rate:vk::VertexInputRate::VERTEX,
                }
            ]
        }else{
            vec![
                vk::VertexInputBindingDescription{
                    binding:0,
                    stride:std::mem::size_of::<crate::decoder::Vertex>() as u32,
                    input_rate:vk::VertexInputRate::VERTEX,
                }
            ]
        };

        let vertex_input_attribute_descriptions=if textured{
            vec![
                vk::VertexInputAttributeDescription{
                    location:0,
                    binding:vertex_input_binding_descriptions[0].binding,
                    format:vk::Format::R32G32B32A32_SFLOAT,//space position
                    offset:0,
                },
                vk::VertexInputAttributeDescription{
                    location:1,
                    binding:vertex_input_binding_descriptions[0].binding,
                    format:vk::Format::R32G32B32_SFLOAT,//texture position
                    offset:0,
                }
            ]
        }else{
            vec![
                vk::VertexInputAttributeDescription{
                    location:0,
                    binding:vertex_input_binding_descriptions[0].binding,
                    format:vk::Format::R32G32B32A32_SFLOAT,//space position
                    offset:0,
                }
            ]
        };
        let vertex_input_state=vk::PipelineVertexInputStateCreateInfo{
            vertex_binding_description_count:vertex_input_binding_descriptions.len() as u32,
            p_vertex_binding_descriptions:vertex_input_binding_descriptions.as_ptr(),
            vertex_attribute_description_count:vertex_input_attribute_descriptions.len() as u32,
            p_vertex_attribute_descriptions:vertex_input_attribute_descriptions.as_ptr(),
            ..Default::default()
        };

        let input_assembly_state_create_info=vk::PipelineInputAssemblyStateCreateInfo{
            topology:vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable:false as u32,
            ..Default::default()
        };

        let viewport_state_create_info=vk::PipelineViewportStateCreateInfo{
            //intentionally blank. viewport and scissor are dynamic
            viewport_count:1,
            scissor_count:1,
            ..Default::default()
        };

        let rasterization_state_create_info=vk::PipelineRasterizationStateCreateInfo{
            depth_clamp_enable:false as u32,
            rasterizer_discard_enable:false as u32,//why the fuck would you disable this?
            polygon_mode:vk::PolygonMode::FILL,
            cull_mode:vk::CullModeFlags::BACK,
            front_face:vk::FrontFace::COUNTER_CLOCKWISE,
            depth_bias_enable:false as u32,
            line_width:1.0,//specs state this must be 1.0 if wide lines feature is not enabled
            ..Default::default()
        };

        let multisample_state_create_info=vk::PipelineMultisampleStateCreateInfo{
            rasterization_samples:vk::SampleCountFlags::TYPE_1,
            sample_shading_enable:false as u32,
            min_sample_shading:1.0,//must be in [0,1] range, no matter sample shading enable value
            alpha_to_coverage_enable:false as u32,
            alpha_to_one_enable:false as u32,
            ..Default::default()
        };

        let depth_stencil_state_create_info=vk::PipelineDepthStencilStateCreateInfo{
            depth_test_enable:false as u32,
            depth_write_enable:false as u32,
            depth_compare_op:vk::CompareOp::LESS,
            depth_bounds_test_enable:false as u32,
            stencil_test_enable:false as u32,
            ..Default::default()
        };

        let color_blend_attachment_states=vec![
            vk::PipelineColorBlendAttachmentState{
                blend_enable:false as u32,
                color_write_mask:vk::ColorComponentFlags::all(),//do not mix colors, just forward input (and use all of its color channels)
                ..Default::default()
            }
        ];
        let color_blend_state_create_info=vk::PipelineColorBlendStateCreateInfo{
            logic_op_enable:false as u32,
            attachment_count:color_blend_attachment_states.len() as u32,
            p_attachments:color_blend_attachment_states.as_ptr(),
            blend_constants:[1.0,1.0,1.0,1.0],
            ..Default::default()
        };

        let dynamic_states=vec![
            vk::DynamicState::VIEWPORT,
            vk::DynamicState::SCISSOR,
        ];
        let pipeline_dynamic_state_create_info=vk::PipelineDynamicStateCreateInfo{
            dynamic_state_count:dynamic_states.len() as u32,
            p_dynamic_states:dynamic_states.as_ptr(),
            ..Default::default()
        };

        let graphics_pipeline_create_info=vk::GraphicsPipelineCreateInfo{
            stage_count:shader_stages.len() as u32,
            p_stages:shader_stages.as_ptr(),
            p_vertex_input_state:&vertex_input_state,
            p_input_assembly_state:&input_assembly_state_create_info,
            p_viewport_state:&viewport_state_create_info,
            p_rasterization_state:&rasterization_state_create_info,
            p_multisample_state:&multisample_state_create_info,
            p_depth_stencil_state:&depth_stencil_state_create_info,
            p_color_blend_state:&color_blend_state_create_info,
            p_dynamic_state:&pipeline_dynamic_state_create_info,
            layout:graphics_pipeline_layout,
            render_pass:self.render_pass,
            subpass:0,
            ..Default::default()
        };
        let graphics_pipelines=unsafe{
            self.vulkan.device.create_graphics_pipelines(vk::PipelineCache::null(),&[graphics_pipeline_create_info],self.vulkan.get_allocation_callbacks())
        }.unwrap();
        let graphics_pipeline=graphics_pipelines[0];

        let pipeline=std::sync::Arc::new(GraphicsPipeline{
            set_bindings,

            layout:graphics_pipeline_layout,
            pipeline:graphics_pipeline,
        
            vertex:vertex_shader,
            fragment:fragment_shader,
            
            descriptor_pool,
            descriptor_set_layouts,
        });

        self.pipelines.insert(String::from(filename),pipeline.clone());

        pipeline
    }
}
impl Drop for RenderPass{
    fn drop(&mut self){
        unsafe{
            for (_,pipeline) in self.pipelines.iter(){
                self.vulkan.device.destroy_pipeline(pipeline.pipeline, self.vulkan.get_allocation_callbacks());
                self.vulkan.device.destroy_pipeline_layout(pipeline.layout, self.vulkan.get_allocation_callbacks());
                
                self.vulkan.device.destroy_shader_module(pipeline.vertex,self.vulkan.get_allocation_callbacks());
                self.vulkan.device.destroy_shader_module(pipeline.fragment,self.vulkan.get_allocation_callbacks());

                self.vulkan.device.destroy_descriptor_pool(pipeline.descriptor_pool,self.vulkan.get_allocation_callbacks());
                for descriptor_set_layout in pipeline.descriptor_set_layouts.iter(){
                    self.vulkan.device.destroy_descriptor_set_layout(*descriptor_set_layout,self.vulkan.get_allocation_callbacks());
                }
            }

            self.vulkan.device.destroy_semaphore(self.rendering_done,self.vulkan.get_allocation_callbacks());

            self.vulkan.device.destroy_framebuffer(self.framebuffer,self.vulkan.get_allocation_callbacks());

            self.vulkan.device.destroy_render_pass(self.render_pass,self.vulkan.get_allocation_callbacks());
            
            self.vulkan.device.destroy_image_view(self.color_image_view,self.vulkan.get_allocation_callbacks());
            self.vulkan.device.destroy_image_view(self.depth_image_view,self.vulkan.get_allocation_callbacks());

            self.vulkan.device.free_memory(self.color_image_memory,self.vulkan.get_allocation_callbacks());
            self.vulkan.device.free_memory(self.depth_image_memory,self.vulkan.get_allocation_callbacks());

            self.vulkan.device.destroy_image(self.color_image,self.vulkan.get_allocation_callbacks());
            self.vulkan.device.destroy_image(self.depth_image,self.vulkan.get_allocation_callbacks());
        }
    }
}

pub struct WindowAttachments{
    pub extent:vk::Extent2D,

    pub surface:vk::SurfaceKHR,

    pub image_available:vk::Semaphore,
    pub image_transferable:vk::Semaphore,
    pub image_presentable:vk::Semaphore,
    pub copy_done:vk::Semaphore,
    
    pub swapchain:extensions::khr::Swapchain,
    pub swapchain_handle:vk::SwapchainKHR,
    pub swapchain_images:Vec<vk::Image>,
    pub swapchain_image_views:Vec<vk::ImageView>,
    pub swapchain_image_framebuffers:Vec<vk::Framebuffer>,

    pub render_pass_2d:RenderPass,
    pub render_pass_3d:RenderPass,
}

pub struct Painter{
    pub vulkan:std::sync::Arc<crate::VulkanBase>,

    pub surface:extensions::khr::Surface,

    pub frame_sync_fence:vk::Fence,

    pub present_queue:crate::Queue,

    pub present_render_pass:vk::RenderPass,

    pub swapchain_surface_format:vk::SurfaceFormatKHR,

    pub window_attachments:std::collections::HashMap<u32,WindowAttachments>,

    pub graphics_queue:crate::Queue
}

impl Drop for Painter{
    fn drop(&mut self){
        unsafe{
            self.vulkan.device.destroy_fence(self.frame_sync_fence,self.vulkan.get_allocation_callbacks());
        }
    }
}

impl Painter{
    pub fn new(vulkan:&std::sync::Arc<crate::VulkanBase>,test_window:&crate::TestWindow,graphics_queue:crate::Queue,present_queue:crate::Queue)->Self{
        let surface=extensions::khr::Surface::new(&vulkan.entry,&vulkan.instance);
        //used to wait for last frame to be finished (and synchronized with max framerate) before new frame starts
        //must be signaled to simulate last frame being finished on first frame
        let fence_create_info=vk::FenceCreateInfo{
            flags:vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };
        let frame_sync_fence=unsafe{
            vulkan.device.create_fence(&fence_create_info,vulkan.get_allocation_callbacks())
        }.unwrap();
        
        //create render pass for simple rendering operations
        let surface_formats=unsafe{
            surface.get_physical_device_surface_formats(vulkan.physical_device, test_window.platform_surface)
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

        let mut present_queue=present_queue;
        let mut graphics_queue=graphics_queue;
        //ensure existence of at least 2 command buffers
        let range=present_queue.create_command_buffers(2);
        println!("created command buffers in this range: {:?}",range);
        let range=graphics_queue.create_command_buffers(3);
        println!("created command buffers in this range: {:?}",range);

        Self{
            vulkan:vulkan.clone(),

            surface,

            frame_sync_fence,

            swapchain_surface_format,

            present_queue,
            present_render_pass:vk::RenderPass::null(),
            window_attachments:std::collections::HashMap::new(),

            graphics_queue,
        }
    }

    //wait for fence on start?
    //signal fence when done?
    //pub fn draw(&mut self,mesh:std::sync::Arc<crate::Mesh>)->crate::ControlFlow{
    pub fn draw(&mut self,scene:&crate::Scene)->crate::ControlFlow{
        let window_ids=self.window_attachments.keys();
        for id in window_ids{
            let window_attachment=self.window_attachments.get(id).unwrap();
            //for each renderpass (2d and 3d):
            {
                let command_buffer=self.graphics_queue.command_buffers[0];
                let command_buffer_begin_info=vk::CommandBufferBeginInfo{
                    flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.begin_command_buffer(command_buffer,&command_buffer_begin_info)
                }.unwrap();
                let clear_values=vec![
                    vk::ClearValue{
                        color:vk::ClearColorValue{
                            float32:[0.9,0.5,0.2,1.0],
                        }
                    },
                    vk::ClearValue{
                        depth_stencil:vk::ClearDepthStencilValue{
                            depth:1.0,
                            stencil:0,
                        }
                    }
                ];
                let render_pass_begin_info=vk::RenderPassBeginInfo{
                    render_pass:window_attachment.render_pass_2d.render_pass,
                    framebuffer:window_attachment.render_pass_2d.framebuffer,
                    render_area:vk::Rect2D{
                        offset:vk::Offset2D{
                            x:0,
                            y:0,
                        },
                        extent:window_attachment.render_pass_2d.extent,
                    },
                    clear_value_count:clear_values.len() as u32,
                    p_clear_values:clear_values.as_ptr(),
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.cmd_begin_render_pass(command_buffer,&render_pass_begin_info,vk::SubpassContents::INLINE)
                };


                //bind first pipeline, statically
                unsafe{
                    self.vulkan.device.cmd_bind_pipeline(command_buffer,vk::PipelineBindPoint::GRAPHICS,window_attachment.render_pass_2d.pipelines.iter().next().unwrap().1.pipeline);
                }

                let viewport=vk::Viewport{
                    x:0.0,
                    y:0.0,
                    width:window_attachment.extent.width as f32,
                    height:window_attachment.extent.height as f32,
                    min_depth:0.0,
                    max_depth:1.0,
                };
                let scissor=vk::Rect2D{
                    offset:vk::Offset2D{
                        x:0,
                        y:0,  
                    },
                    extent:vk::Extent2D{
                        width:window_attachment.extent.width,
                        height:window_attachment.extent.height,
                    }
                };
                unsafe{
                    self.vulkan.device.cmd_set_viewport(command_buffer,0,&[viewport]);
                    self.vulkan.device.cmd_set_scissor(command_buffer,0,&[scissor]);
                }

                unsafe{
                    self.vulkan.device.cmd_bind_vertex_buffers(command_buffer,0,&[scene.objects[0].mesh.vertices.buffer],&[0]);
                    self.vulkan.device.cmd_bind_index_buffer(command_buffer,scene.objects[0].mesh.vertex_indices.buffer,0,vk::IndexType::UINT16);
                    self.vulkan.device.cmd_draw_indexed(command_buffer,scene.objects[0].mesh.vertex_indices.item_count as u32,1,0,0,0);
                }

                unsafe{
                    self.vulkan.device.cmd_end_render_pass(command_buffer)
                };

                //transition image to source for copy to swapchain image
                let image_memory_barrier=vk::ImageMemoryBarrier{
                    src_access_mask:vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    dst_access_mask:vk::AccessFlags::TRANSFER_READ,
                    old_layout:vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    new_layout:vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    src_queue_family_index:self.graphics_queue.family_index,
                    dst_queue_family_index:self.graphics_queue.family_index,
                    image:window_attachment.render_pass_2d.color_image,
                    subresource_range:vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    },
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::ALL_COMMANDS, 
                        vk::PipelineStageFlags::ALL_COMMANDS, 
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[image_memory_barrier]
                    )
                };

                unsafe{
                    self.vulkan.device.end_command_buffer(command_buffer)
                }.unwrap();

                let wait_semaphores=vec![];
                let command_buffers=vec![
                    command_buffer
                ];
                let signal_semaphores=vec![
                    window_attachment.render_pass_2d.rendering_done
                ];
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
                    self.vulkan.device.queue_submit(self.graphics_queue.queue,&[submit_info],vk::Fence::null())
                }.unwrap();
                //draw camera perspective to framebuffer
                    //TODO
            }
                    
            //acquire swapchain image
            let (image_index,suboptimal)=unsafe{
                window_attachment.swapchain.acquire_next_image(window_attachment.swapchain_handle, u64::MAX, window_attachment.image_available, vk::Fence::null())
            }.unwrap();
            //this means the swapchain should be recreated, but we dont care much right now
            if suboptimal{
                println!("swapchain image acquired is suboptimal");
                return crate::ControlFlow::Stop;
            }

            let swapchain_image=window_attachment.swapchain_images[image_index as usize];

            //hand image over to graphics queue
            {
                let command_buffer=self.present_queue.command_buffers[0];
                //begin transition command buffer 1
                let present_queue_command_buffer_begin_info=vk::CommandBufferBeginInfo{
                    flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.begin_command_buffer(command_buffer,&present_queue_command_buffer_begin_info)
                }.unwrap();
                let image_memory_barrier=vk::ImageMemoryBarrier{
                    src_access_mask:vk::AccessFlags::empty(),
                    dst_access_mask:vk::AccessFlags::TRANSFER_WRITE,
                    old_layout:vk::ImageLayout::UNDEFINED,
                    new_layout:vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    src_queue_family_index:self.present_queue.family_index,
                    dst_queue_family_index:self.graphics_queue.family_index,
                    image:swapchain_image,
                    subresource_range:vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    },
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::ALL_COMMANDS, 
                        vk::PipelineStageFlags::ALL_COMMANDS, 
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[image_memory_barrier]
                    )
                };
                //end transition command buffer 1
                unsafe{
                    self.vulkan.device.end_command_buffer(command_buffer)
                }.unwrap();
                //submit transition command buffer 1
                let wait_semaphores_1=vec![
                    window_attachment.image_available,
                ];
                let command_buffers_1=vec![
                    command_buffer
                ];
                let signal_semaphores_1=vec![
                    window_attachment.image_transferable
                ];
                let submit_info_1=vk::SubmitInfo{
                    wait_semaphore_count:wait_semaphores_1.len() as u32,
                    p_wait_semaphores:wait_semaphores_1.as_ptr(),
                    p_wait_dst_stage_mask:&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    command_buffer_count:command_buffers_1.len() as u32,
                    p_command_buffers:command_buffers_1.as_ptr(),
                    signal_semaphore_count:signal_semaphores_1.len() as u32,
                    p_signal_semaphores:signal_semaphores_1.as_ptr(),
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.queue_submit(self.present_queue.queue,&[submit_info_1],vk::Fence::null())
                }.unwrap();
            }
            //retrieve from present queue
            {
                let command_buffer=self.graphics_queue.command_buffers[2];
                let command_buffer_begin_info=vk::CommandBufferBeginInfo{
                    flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.begin_command_buffer(command_buffer,&command_buffer_begin_info)
                }.unwrap();
                let image_memory_barrier_present_to_graphics=vk::ImageMemoryBarrier{
                    src_access_mask:vk::AccessFlags::empty(),
                    dst_access_mask:vk::AccessFlags::TRANSFER_WRITE,
                    old_layout:if self.graphics_queue.family_index==self.present_queue.family_index{
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL
                    }else{
                        vk::ImageLayout::UNDEFINED
                    },
                    new_layout:vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    src_queue_family_index:self.present_queue.family_index,
                    dst_queue_family_index:self.graphics_queue.family_index,
                    image:swapchain_image,
                    subresource_range:vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    },
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::ALL_COMMANDS,//src_stage_mask
                        vk::PipelineStageFlags::ALL_COMMANDS,
                        vk::DependencyFlags::empty(),
                        &[],//memory barriers
                        &[],//buffer memory barriers
                        &[image_memory_barrier_present_to_graphics]//image memory barriers
                    )
                };

                //copy renderpass image to swapchain
                unsafe{
                    let copy_range_offset_coordinates=[
                        vk::Offset3D{
                            x:0,
                            y:0,
                            z:0,
                        },
                        vk::Offset3D{
                            x:window_attachment.extent.width as i32,
                            y:window_attachment.extent.height as i32,
                            z:1,
                        }
                    ];
                    self.vulkan.device.cmd_blit_image(
                        command_buffer,
                        window_attachment.render_pass_2d.color_image,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                        swapchain_image,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                        &[vk::ImageBlit{
                            src_subresource:vk::ImageSubresourceLayers{
                                aspect_mask:vk::ImageAspectFlags::COLOR,
                                mip_level:0,
                                base_array_layer:0,
                                layer_count:1
                            },
                            src_offsets:copy_range_offset_coordinates,
                            dst_subresource:vk::ImageSubresourceLayers{
                                aspect_mask:vk::ImageAspectFlags::COLOR,
                                mip_level:0,
                                base_array_layer:0,
                                layer_count:1
                            },
                            dst_offsets:copy_range_offset_coordinates,
                        }],
                        vk::Filter::NEAREST,
                    );
                }

                let image_memory_barrier_graphics_to_present=vk::ImageMemoryBarrier{
                    src_access_mask:vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask:vk::AccessFlags::MEMORY_READ,
                    old_layout:vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    new_layout:vk::ImageLayout::PRESENT_SRC_KHR,
                    src_queue_family_index:self.graphics_queue.family_index,
                    dst_queue_family_index:self.present_queue.family_index,
                    image:swapchain_image,
                    subresource_range:vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    },
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::ALL_COMMANDS,//src_stage_mask
                        vk::PipelineStageFlags::ALL_COMMANDS,
                        vk::DependencyFlags::empty(),
                        &[],//memory barriers
                        &[],//buffer memory barriers
                        &[image_memory_barrier_graphics_to_present]//image memory barriers
                    )
                };
                unsafe{
                    self.vulkan.device.end_command_buffer(command_buffer)
                }.unwrap();

                let wait_semaphores=vec![
                    window_attachment.image_transferable,
                    window_attachment.render_pass_2d.rendering_done
                ];
                let wait_dst_stage_masks=vec![
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                ];
                assert!(wait_semaphores.len()==wait_dst_stage_masks.len());
                let command_buffers=vec![
                    command_buffer
                ];
                let signal_semaphores=vec![
                    window_attachment.copy_done
                ];
                let submit_info=vk::SubmitInfo{
                    wait_semaphore_count:wait_semaphores.len() as u32,
                    p_wait_semaphores:wait_semaphores.as_ptr(),
                    p_wait_dst_stage_mask:wait_dst_stage_masks.as_ptr(),
                    command_buffer_count:command_buffers.len() as u32,
                    p_command_buffers:command_buffers.as_ptr(),
                    signal_semaphore_count:signal_semaphores.len() as u32,
                    p_signal_semaphores:signal_semaphores.as_ptr(),
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.queue_submit(self.graphics_queue.queue,&[submit_info],vk::Fence::null())
                }.unwrap();
            }
            //copy 3d image color attachment to swapchain image
            //copy 2d image color attachment to swapchain image
            //return swapchain image to present queue
            {
                let command_buffer=self.present_queue.command_buffers[1];
                //begin transition command buffer 1
                let present_queue_command_buffer_begin_info=vk::CommandBufferBeginInfo{
                    flags:vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.begin_command_buffer(command_buffer,&present_queue_command_buffer_begin_info)
                }.unwrap();
                let image_memory_barrier=vk::ImageMemoryBarrier{
                    src_access_mask:vk::AccessFlags::TRANSFER_WRITE,
                    dst_access_mask:vk::AccessFlags::MEMORY_READ,
                    old_layout:if self.graphics_queue.family_index==self.present_queue.family_index{ //copy old_layout from 'release' operation only if this is a ownership transfer
                        vk::ImageLayout::PRESENT_SRC_KHR
                    }else{
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL
                    },
                    new_layout:vk::ImageLayout::PRESENT_SRC_KHR,
                    src_queue_family_index:self.graphics_queue.family_index,
                    dst_queue_family_index:self.present_queue.family_index,
                    image:swapchain_image,
                    subresource_range:vk::ImageSubresourceRange{
                        aspect_mask:vk::ImageAspectFlags::COLOR,
                        base_mip_level:0,
                        level_count:1,
                        base_array_layer:0,
                        layer_count:1,
                    },
                    ..Default::default()
                };
                unsafe{
                    self.vulkan.device.cmd_pipeline_barrier(
                        command_buffer,
                        vk::PipelineStageFlags::ALL_COMMANDS, 
                        vk::PipelineStageFlags::ALL_COMMANDS, 
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &[image_memory_barrier]
                    )
                };
                //end transition command buffer 1
                unsafe{
                    self.vulkan.device.end_command_buffer(command_buffer)
                }.unwrap();
                //submit transition command buffer 1
                let wait_semaphores_1=vec![
                    window_attachment.copy_done,
                ];
                let command_buffers_1=vec![
                    command_buffer
                ];
                let signal_semaphores_1=vec![
                    window_attachment.image_transferable
                ];
                let submit_info_1=vk::SubmitInfo{
                    wait_semaphore_count:wait_semaphores_1.len() as u32,
                    p_wait_semaphores:wait_semaphores_1.as_ptr(),
                    p_wait_dst_stage_mask:&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                    command_buffer_count:command_buffers_1.len() as u32,
                    p_command_buffers:command_buffers_1.as_ptr(),
                    signal_semaphore_count:signal_semaphores_1.len() as u32,
                    p_signal_semaphores:signal_semaphores_1.as_ptr(),
                    ..Default::default()
                };

                unsafe{
                    self.vulkan.device.queue_submit(self.present_queue.queue,&[submit_info_1],vk::Fence::null())
                }.unwrap();
            }
            //present swapchain image
            {
                let present_wait_semaphores=vec![
                    window_attachment.image_transferable,
                ];
                let mut present_results=vec![
                    vk::Result::SUCCESS,
                ];
                let present_info=vk::PresentInfoKHR{
                    wait_semaphore_count:present_wait_semaphores.len() as u32,
                    p_wait_semaphores:present_wait_semaphores.as_ptr(),
                    swapchain_count:1,
                    p_swapchains:&window_attachment.swapchain_handle,
                    p_image_indices:&image_index,
                    p_results:present_results.as_mut_ptr(),
                    ..Default::default()
                };
                unsafe{
                    window_attachment.swapchain.queue_present(self.present_queue.queue,&present_info)
                }.unwrap();
            }
        }

        return crate::ControlFlow::Continue;


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

        /*

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

        let model=object.transform.model_matrix();

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

        */
    }

    #[cfg(disabled)]
    pub fn window_draw_present(&mut self,window_id:u32){
        
        //wait for last frame to finish
        unsafe{
            self.vulkan.device.wait_for_fences(&[self.frame_sync_fence], true, u64::MAX).unwrap();
            self.vulkan.device.reset_fences(&[self.frame_sync_fence]).unwrap();
        }

        {
            //acquire next swapchain image for drawing and presenting
            //copied
            /*
            let (image_index,suboptimal)=unsafe{
                self.open_windows[0].swapchain.acquire_next_image(self.open_windows[0].swapchain_handle, u64::MAX, self.open_windows[0].image_available, vk::Fence::null())
            }.unwrap();

            //this means the swapchain should be recreated, but we dont care much right now
            if suboptimal{
                println!("swapchain image acquired is suboptimal");
                return ControlFlow::Stop;
            }
            let swapchain_image=self.open_windows[0].swapchain_images[image_index as usize];

            */

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
                //let quad_data=self.decoder.get_mesh("quad.obj",self.painter.graphics_queue_command_buffers[0]);
                let quad_data=self.decoder.get_mesh("bunny.obj",self.painter.graphics_queue_command_buffers[0]);

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

            //copied
            /*
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
            */
        }   
    }
    
    pub fn add_window(&mut self,window:&crate::Window){
        //vulkan spec states this must be done
        if unsafe{
            !self.surface.get_physical_device_surface_support(self.vulkan.physical_device, self.present_queue.family_index, window.surface).unwrap()
        }{
            panic!("new surface does not support presentation like the temporary ones");
        }

        //create swapchain
        let image_available=self.vulkan.create_semaphore().unwrap();
        let image_transferable=self.vulkan.create_semaphore().unwrap();
        let image_presentable=self.vulkan.create_semaphore().unwrap();
        let copy_done=self.vulkan.create_semaphore().unwrap();

        let surface_capabilities=unsafe{
            self.surface.get_physical_device_surface_capabilities(self.vulkan.physical_device, window.surface)
        }.unwrap();

        let mut image_count:u32=surface_capabilities.min_image_count+1;
        if surface_capabilities.max_image_count>0 && image_count>surface_capabilities.max_image_count{
            image_count=surface_capabilities.max_image_count;
        }

        let surface_formats=unsafe{
            self.surface.get_physical_device_surface_formats(self.vulkan.physical_device, window.surface)
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
            swapchain_extent.width=window.extent.width as u32;
            swapchain_extent.height=window.extent.height as u32;
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
            self.surface.get_physical_device_surface_present_modes(self.vulkan.physical_device, window.surface)
        }.unwrap();
        let swapchain_surface_present_mode=if surface_present_modes.contains(&vk::PresentModeKHR::MAILBOX){
            vk::PresentModeKHR::MAILBOX
        }else{
            vk::PresentModeKHR::FIFO
        };

        //queue family indices accessing the swapchain (e.g. presenting to it), for which we have a dedicated queue
        let queue_family_indices=vec![
            self.present_queue.family_index,
        ];
        let swapchain_create_info=vk::SwapchainCreateInfoKHR{
            surface:window.surface,
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
        let swapchain=extensions::khr::Swapchain::new(&self.vulkan.instance,&self.vulkan.device);
        let swapchain_handle=unsafe{
            swapchain.create_swapchain(&swapchain_create_info, self.vulkan.get_allocation_callbacks())
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
                self.vulkan.device.create_image_view(&image_view_create_info, self.vulkan.get_allocation_callbacks())
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
        
        let render_pass_2d=RenderPass::new(&self.vulkan,&self.surface,&window);
        let render_pass_3d=RenderPass::new(&self.vulkan,&self.surface,&window);

        let window_attachment=WindowAttachments{
            extent:swapchain_extent,
            
            surface:window.surface,
        
            image_available,
            image_transferable,
            image_presentable,
            copy_done,
            
            swapchain,
            swapchain_handle,
            swapchain_images,
            swapchain_image_views,
            swapchain_image_framebuffers,
        
            render_pass_2d,
            render_pass_3d,
        };

        self.window_attachments.insert(window.id,window_attachment);
    }

    pub fn remove_window(&mut self, window_id:u32){
        let window_attachment=self.window_attachments.remove(&window_id).unwrap();

        for framebuffer in window_attachment.swapchain_image_framebuffers.iter(){
            unsafe{
                self.vulkan.device.destroy_framebuffer(*framebuffer, self.vulkan.get_allocation_callbacks());
            }
        }
        for image_view in window_attachment.swapchain_image_views.iter(){
            unsafe{
                self.vulkan.device.destroy_image_view(*image_view, self.vulkan.get_allocation_callbacks());
            }
        }
        unsafe{
            self.vulkan.device.destroy_semaphore(window_attachment.image_available, self.vulkan.get_allocation_callbacks());
            self.vulkan.device.destroy_semaphore(window_attachment.image_transferable, self.vulkan.get_allocation_callbacks());
            self.vulkan.device.destroy_semaphore(window_attachment.image_presentable, self.vulkan.get_allocation_callbacks());
            self.vulkan.device.destroy_semaphore(window_attachment.copy_done, self.vulkan.get_allocation_callbacks());
            window_attachment.swapchain.destroy_swapchain(
                window_attachment.swapchain_handle,
                self.vulkan.get_allocation_callbacks()
            );
            self.surface.destroy_surface(window_attachment.surface,self.vulkan.get_allocation_callbacks())
        };
    }
}