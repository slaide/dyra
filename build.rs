use std::process::Command;

use std::str::FromStr;

fn main(){
    for shader_name in &["resources/shaders/textured_polygon_2d","resources/shaders/untextured_polygon_2d","resources/shaders/textured_polygon_3d"]{
        //add flags in the future to optimize shaders? using spirv-tools optimizer (spirv-opt) as part of lunarg-sdk
        let shader_name=String::from_str(shader_name).unwrap();

        let mut arg1=shader_name.clone();
        arg1.push_str(".vert");
        let mut arg2=shader_name.clone();
        arg2.push_str(".vert.spv");
        Command::new("glslangValidator").args(&[arg1.as_str(),"--target-env","vulkan1.2","-o",arg2.as_str()]).status().unwrap();

        let mut arg1=shader_name.clone();
        arg1.push_str(".frag");
        let mut arg2=shader_name.clone();
        arg2.push_str(".frag.spv");
        Command::new("glslangValidator").args(&[arg1.as_str(),"--target-env","vulkan1.2","-o",arg2.as_str()]).status().unwrap();

        println!("cargo:rerun-if-changed={}.frag",shader_name);
        println!("cargo:rerun-if-changed={}.vert",shader_name);
    }
}