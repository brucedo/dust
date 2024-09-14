use std::{collections::HashMap, fs::read_dir, path::Path};
#[cfg(all(target_os = "linux", not(target_os = "windows")))]
use std::{
    fs::File,
    io::{Error, ErrorKind},
    os::unix::fs::MetadataExt,
};

use log::debug;

#[cfg(all(target_os = "windows", not(target_os = "linux")))]
use std::{
    fs::File,
    io::{Error, ErrorKind},
    os::windows::fs::MetadataExt,
};

pub enum ShaderType {
    Vertex(Vec<u32>),
    Fragment(Vec<u32>),
    TesselationControl(Vec<u32>),
    TesselationEval(Vec<u32>),
    Geometry(Vec<u32>),
    Compute(Vec<u32>),
}

// *** load_shader(file_name: &mut File) -> Result<Vec<u32>, Error>
//
// load_shader does the grunt work of loading a compiled SPIR-V shader file into memory.
// It makes no effort to confirm the contents of the file.  It does, however, confirm the
// length physical property defined in https://registry.khronos.org/SPIR-V/papers/WhitePaper.pdf,
// which claims that every SPIR-V program consists of a stream of 32-bit words.  Thus, any compiled
// file should be a multiple of 4 bytes long, and we reject any File that fails this property
// check.
//
pub fn load_shader(file_name: &mut File) -> Result<Vec<u32>, Error> {
    #[cfg(all(target_os = "linux", not(target_os = "windows")))]
    let file_size = file_name.metadata()?.size();

    #[cfg(all(target_os = "windows", not(target_os = "linux")))]
    let file_size = file_name.metadata()?.file_size();

    let quad_count = file_size / 4;

    if quad_count % 4 != 0 {
        Err(std::io::Error::from(ErrorKind::UnexpectedEof))
    } else {
        let mut shader_bytes = Vec::with_capacity(quad_count as usize);

        let read_buffer = Vec::<u8>::with_capacity(file_size as usize);

        for chunk in read_buffer.chunks_exact(4) {
            let array_ref: &[u8; 4] = chunk.try_into().unwrap();
            shader_bytes.push(u32::from_be_bytes(*array_ref));
        }

        Ok(shader_bytes)
    }
}

pub fn load_shaders() -> HashMap<String, ShaderType> {
    let mut current_path = match std::env::current_dir() {
        Ok(path) => path,
        Err(msg) => {
            panic!("There appears to be no current directory? {:?}", msg);
        }
    };

    current_path.push("shaders");
    let mut storage = HashMap::new();

    process_shader_directory(&current_path, &mut storage);

    storage
}

fn process_shader_directory(path: &Path, storage: &mut HashMap<String, ShaderType>) {
    let dir_contents = match read_dir(path) {
        Ok(dir) => dir,
        Err(msg) => {
            panic!(
                "Unable to read the shaders subdirectory of the current directory {:?}: {:?}",
                path, msg
            );
        }
    };

    for entry in dir_contents.flatten() {
        if entry.path().is_dir() {
            process_shader_directory(&entry.path(), storage);
        } else if entry.path().is_file() {
            process_shader_file(&entry.path(), storage);
        }
    }
}

fn process_shader_file(path: &Path, storage: &mut HashMap<String, ShaderType>) {
    if let Ok(mut file) = File::open(path) {
        let shader_contents = match load_shader(&mut file) {
            Ok(vec) => vec,
            Err(msg) => {
                debug!(
                    "Attempting to load the shader resulted in an IO or file failure: {:?}",
                    msg
                );
                return;
            }
        };

        let shader_type = match path.to_str() {
            Some(path_str) if path_str.contains("fragment") => {
                ShaderType::Fragment(shader_contents)
            }

            Some(path_str) if path_str.contains("vertex") => ShaderType::Vertex(shader_contents),
            Some(path_str) if path_str.contains("geometry") => {
                ShaderType::Geometry(shader_contents)
            }
            Some(path_str) if path_str.contains("tess_ctrl") => {
                ShaderType::TesselationControl(shader_contents)
            }
            Some(path_str) if path_str.contains("tess_eval") => {
                ShaderType::TesselationEval(shader_contents)
            }
            Some(path_str) if path_str.contains("compute") => ShaderType::Compute(shader_contents),

            Some(_) => unreachable!("Shaders must be one of fragment, vertex, geometry, tesselation control, \
                tesselation evaluation, or compute, and must be in an appropriately named subdirectory."),
            None => {
                unreachable!("There should be no path that has allowed me to open a file and read it that then has no path.");
            }
        };
    }
}
