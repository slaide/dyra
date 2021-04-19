use std::process::Command;

fn main(){
    Command::new("glslangValidator").args(&["shader.vert","--target-env","vulkan1.2"]).status().unwrap();
    Command::new("glslangValidator").args(&["shader.frag","--target-env","vulkan1.2"]).status().unwrap();
    //add flags in the future to optimize shaders? using spirv-tools optimizer (spirv-opt) as part of lunarg-sdk
    println!("cargo:rerun-if-changed=shader.frag");
    println!("cargo:rerun-if-changed=shader.vert");
    println!("cargo:rerun-if-changed=vert.spv");
    println!("cargo:rerun-if-changed=frag.spv");
}