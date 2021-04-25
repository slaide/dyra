#version 450

layout(location=0) in vec4 i_Position;
layout(location=1) in vec4 i_Color;

layout(push_constant) uniform PushConstants{
    mat4 model;
    mat4 view;
    mat4 projection;
}constants;

out gl_PerVertex{
    vec4 gl_Position;
};

layout(location=0) out vec4 v_Color;

void main(){
    gl_Position=constants.view*constants.model*i_Position;
    gl_Position=constants.projection*constants.view*constants.model*i_Position;

    v_Color=i_Color;
}