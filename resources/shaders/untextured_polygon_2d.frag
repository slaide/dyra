#version 450

//layout(set=0,binding=0) uniform sampler2D u_Texture;

layout(location=0) in vec2 v_Texcoord;

layout(location=0) out vec4 o_Color;

void main(){
    //o_Color=texture(u_Texture,v_Texcoord);
    //o_Color=vec4(0.9,0.9,0.9,0.5);
    o_Color=vec4(gl_FragCoord.xyz*0.005,1.0);
}