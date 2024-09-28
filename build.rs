use std::{
    fs::ReadDir,
    path::Path,
    process::{exit, Command},
};

fn main() {
    let out_dir = match std::env::var("OUT_DIR") {
        Ok(od) => od,
        Err(_) => {
            exit(-1);
        }
    };

    let shader_target_root = Path::new(&out_dir).join("shaders");
    let shader_target_vertex = shader_target_root.join("vertex");

    let shader_source_root = Path::new("src/graphics/shaders");
    let shader_source_vertex = shader_source_root.join("vertex");

    println!("Output directory: {}", out_dir);
}
