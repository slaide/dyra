#version 450

layout(location=0) in vec4 i_Position;
layout(location=1) in vec3 i_uv_Position;

out gl_PerVertex{
    vec4 gl_Position;
};

layout(location=0) out vec3 o_uv_Position;

void main(){
    gl_Position=i_Position;
    o_uv_Position=i_uv_Position;
}