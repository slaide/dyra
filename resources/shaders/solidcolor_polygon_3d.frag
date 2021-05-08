#version 450

layout(set=0,binding=0) uniform PerFrameMaterialData{
    vec4 i_Color;
};

layout(location=0) in vec4 i_Coord;

layout(location=0) out vec4 o_Color;

void main(){
    o_Color=i_Color;
}