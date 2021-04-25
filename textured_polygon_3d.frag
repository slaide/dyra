#version 450

layout(set=0,binding=0) uniform sampler2D u_Texture;

layout(location=0) in vec4 i_Coord;
layout(location=1) in vec2 v_Texcoord;

layout(location=0) out vec4 o_Color;

void main(){
    o_Color=texture(u_Texture,v_Texcoord);
    
    o_Color=vec4(
        vec3(1.0,1.0,1.0)*i_Coord.z/3,
        1.0
    );
}