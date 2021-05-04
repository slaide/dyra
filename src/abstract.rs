enum VulkanLifecycle{
    //data set once during scene initialization, and read multiple times
    Scene,
    //data initialized once per frame, then read multiple times
    Frame,
    //data initialized to be used for a single draw call
    Draw,
}
//dynamic value types specified by pipeline
//used to assert that data uploaded to pipeline data slots is of correct format, on cpu side
//not all types are allowed for all lifetimes? (will not be for a while probably, because thats hard)
enum DynamicValueType{
    f32,
    f64,
    u32,
    u64,
    vec3,
    vec4,
    mat4,
    texture,
    transform,
}
enum DynamicValue{

}
GraphicsPipeline{
    //these two are required
    //add more shader modules as optionals in the future
    vertex_shader:shadermodule,
    fragment_shader:shadermodule,
    //one descriptor pool per lifecycle (except Draw, which is implemented as push constants)
    //reset descriptor pools at corresponding intervals
    descriptor_pools:HashMap<VulkanLifecycle,DescriptorPool>,
    //all data with scene or frame lifetime live inside descriptor sets
    //where the frame descriptor pool is reset at the beginning of each frame
    //and the scene pool is never reset, instead assumed to be alive for the exact lifetime of the scene
    //(assume that a pipeline is used for material which does not outlive the scene. probably good point for optimizations in the future, but difficult to implement with non-static knowledge of pool requirements)
    //data is upload at the beginning of each lifecycle (true for all lifetimes, not just descriptor sets)
    descriptor_sets:HashMap<{
        name:String,
        bindings:HashMap<{
            binding:u32,
            descriptortype,
            count:u32,
            shader_stages:ShaderStageFlags,
            value_type:DynamicValueType,
        }>
    }>,
    push_constants:HashMap<{
        name:String,
        lifetime:VulkanLifecycle,
        value_type:DynamicValueType,
    }>
}
struct Material{
    //the pipeline this material contains the actual data for (pipeline itself describes the layout of the.. well, pipeline, and its bindpoints to data referenced in the shaders, which must be filled in at frametime by a material and its associated data)
    graphics_pipeline:GraphicsPipeline,
    static_data:HashMap{name:string,value:DynamicValue},
}
struct Object{
    name:String,
    mesh:Arc<Mesh>,
    material:Arc<Material>,
    //a lot of associated data, that will be part of the scripting environment
    //like transform, accessible to materials
    //or references to other objects?
    values:HashMap<String,Arc<DynamicValue>>
}
PerRenderPassData{
    render_pass_create_info:RenderPassCreateInfo,
    graphics_pipelines:Vec<Arc<GraphicsPipeline>>,
    material:Vec<Arc<Material>>,
    meshes:Vec<Arc<Mesh>>,
    scene:Vec<Arc<Object>>
}
impl PerRenderPassData{
    pub fn is_compatible(&self,&RenderPassCreateInfo)->bool{
        //Two attachment references are compatible if they have matching format and sample count, or are both VK_ATTACHMENT_UNUSED or the pointer that would contain the reference is NULL.
        //Two arrays of attachment references are compatible if all corresponding pairs of attachments are compatible. If the arrays are of different lengths, attachment references not present in the smaller array are treated as VK_ATTACHMENT_UNUSED.
        //Two render passes are compatible if their corresponding color, input, resolve, and depth/stencil attachment references are compatible and if they are otherwise identical except for:
        //Initial and final image layout in attachment descriptions
        //Load and store operations in attachment descriptions
        //Image layout in attachment references
        return true;
    }
    //TODO move mesh upload to static data uploader
    pub fn add_object_to_scene(&mut self,meshfilename:String,materialfilename:String){
        //parse mesh file, and stage upload to gpu
        let mesh=Mesh::parse(meshfilename);
        //parse material, parse corresponding graphics pipeline and shaders
        //upload scene specific material data
        //rather: stage all those things
        let material=Material::parse(materialfilename);
        self.scene.push(
            Object{
                name:meshfilename,
                mesh,
                material,
                //assume no associated values so far (read from some scene-object file in the future? or execute some script on load, specified as argument?)
                values:HashMap::new(),
            }
        );
    }
}
struct Camera2D{
    //width and height of 2d view
    width:f32,
    height:f32,
    //how deep into scene this camera can see (number of layers? something like that..)
    depth:f32,//or far_clip?
    //position, scale and rotation of camera view in view plane (strict x-y plane, though any part of it, with custom depth as well)
    transform2d:Transform2D,
    //color format of this view (does not require rgba channels, any subset works as well)
    color_format:vk::Format,
    //renderpass data, because this camera is a render target and needs the appropriate vulkan structs
    color_attachment:Attachment,
    depth_attachment:Attachment,
    renderpass_specific_data:Arc<PerRenderPassData>,
}
impl Camera2D{
    pub fn render(&mut self){
        //make sure that all work is only done once, even when this render pass data is the same as for another render pass!
        for pipeline in self.renderpass_specific_data.graphics_pipelines.iter(){
            //reset frame descriptor pools for new round of writing data
            //(per draw 'pools' are push constants that dont need reset, and per scene are never reset, only deleted)
            for pool in pipeline.descriptor_pool.get_mut(&VulkanLifetime::Frame).unwrap(){
                pool.reset();
            }
        }
        //start render pass
        self.renderpass_specific_data.bind_renderpass();
        //render each object
        for object in self.renderpass_specific_data.scene.iter(){
            //update material per frame data (assume the material is only used by a single object, would need &mut object for all following rendering calls if the material data was object specific, instead of material specific)
            object.material.update_frame_data(&object);
            //bind material for use (binds pipeline as well as all scene and frame material values)
            object.material.pipeline.bind(&material);
            //bind per draw data for current objects
            object.material.bind_draw_data(&object);
            //something like that
            object.draw();
        }
        //end render pass
        self.renderpass_specific_data.unbind_renderpass();
    }
}
struct Camera3D{
    //width and height of plane everything is projected on
    width:f32,
    height:f32,
    //clip range
    near_clip:f32,
    far_clip:f32,
    //transform of camera view ('eye' data)
    transform:Transform,
    //color format of this view (does not require rgba channels, any subset works as well)
    color_format:vk::Format,
    //renderpass data, because this camera is a render target and needs the appropriate vulkan structs
    color_attachment:Attachment,
    depth_attachment:Attachment,
    renderpass_specific_data:Arc<PerRenderPassData>,
}
struct Window{
    view_2d:Camera2D,
    view_3d:Camera3D,
    image:Image,
}

struct Application{
    windows:Vec<Arc<Window>>,
    per_render_pass_data:HashMap<RenderPassCreateInfo,Arc<PerRenderPassData>>
}
impl Application{
    pub fn init(&mut self){
        //use resource uploader to upload data, and read required rendering environment data from renderpass structs
        //actually, upload static data via static data upload pipeline (like mesh and textures)
        //stage material creation by graphics pipeline
        //then flush static upload pipeline first
        //wait for finish
        //then flush graphics upload pipeline (e.g. texture upload needs to be finished before it can be used)
        self.resource_uploader.upload_object(&view_2d.renderpass_specific_data,"quad.do","inteltrucktexture.dm");
    }
    pub fn render(&mut self){
        //stage resource upload for static and material data that has been added during last frame
        //flush static data upload pipeline, wait for finish, flush graphics afterwards, wait again
        //then actually render all of that stuff
        for window in windows{
            view_2d.render();
            view_3d.render();
        
            window.image.copy_from(render_pass_2d.color_attachment);
            window.image.copy_from(render_pass_3d.color_attachment);
        
            window.image.present();
        }
    }
}