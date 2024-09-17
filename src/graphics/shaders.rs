use std::{collections::HashMap, fs::read_dir, path::Path, sync::OnceLock};
#[cfg(all(target_os = "linux", not(target_os = "windows")))]
use std::{
    fs::File,
    io::{Error, ErrorKind},
    os::unix::fs::MetadataExt,
};

use std::sync::Arc;

use ash::vk::{ShaderModule, ShaderModuleCreateFlags, ShaderModuleCreateInfo};
use ash::Device;
use log::{debug, error};

#[cfg(all(target_os = "windows", not(target_os = "linux")))]
use std::{
    fs::File,
    io::{Error, ErrorKind},
    os::windows::fs::MetadataExt,
};

use crate::{dust_errors::DustError, setup::instance::VkContext};

static LOGICAL_DEVICE: OnceLock<Arc<Device>> = OnceLock::new();

pub fn init(device: Arc<Device>) {
    match LOGICAL_DEVICE.set(device) {
        Ok(_) => {}
        Err(_) => {
            panic!("The logical device Arc could not be set in the shader module.");
        }
    };
}

pub fn destroy(ctxt: &VkContext) {}

pub enum ShaderType {
    Vertex,
    Fragment,
    TesselationControl,
    TesselationEval,
    Geometry,
    Compute,
}

pub struct ShaderWrapper {
    shader_module: ShaderModule,
    name: String,
    shader_type: ShaderType,
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
fn load_shader(file_name: &mut File) -> Result<Vec<u32>, Error> {
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
    let mut current_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(msg) => {
            panic!("There appears to be no current directory? {:?}", msg);
        }
    };

    // Using current exe - need to drop the executable from the tail of the path.
    current_path.pop();
    current_path.push("shaders");
    debug!("Shader root path: {:?}", current_path);

    let mut storage = HashMap::new();

    process_shader_directory(&current_path, &mut storage);

    // for (name, shader_type) in storage {
    //     match shader_type {
    //         ShaderType::Vertex(raw_code) => {
    //             ShaderModuleCreateInfo::default()
    //                 .flags(ShaderModuleCreateFlags::empty())
    //                 .code(&raw_code);
    //         }
    //         _ => {}
    //     }
    // }

    storage
}

pub fn destroy_shaders(mut shader_map: HashMap<String, ShaderWrapper>) {
    match LOGICAL_DEVICE.get() {
        Some(device) => shader_map.drain().for_each(|(name, shader_type)| unsafe {
            device.destroy_shader_module(shader_type.shader_module, None)
        }),

        None => {
            panic!("The Vulkan environment has not been initialized.");
        }
    };
}

fn make_shader_module(bytecode: &[u32]) -> Result<ShaderModule, DustError> {
    let create_info = ShaderModuleCreateInfo::default()
        .flags(ShaderModuleCreateFlags::empty())
        .code(bytecode);

    match LOGICAL_DEVICE.get() {
        Some(device) => match unsafe { device.create_shader_module(&create_info, None) } {
            Ok(module) => Ok(module),
            Err(msg) => Err(DustError::CreateShaderModuleFailed(msg)),
        },
        None => {
            panic!("The Vulkan environment has not been initialized.  We cannot continue.");
        }
    }
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

        let module = match make_shader_module(&shader_contents) {
            Ok(module) => module,
            Err(msg) => {
                error!("Shader load operation failed: {:?}", msg);
                return;
            }
        };

        let shader_type = match path.to_str() {
            Some(path_str) if path_str.contains("fragment") => {
                ShaderType::Fragment(module)
            }

            Some(path_str) if path_str.contains("vertex") => ShaderType::Vertex(module),
            Some(path_str) if path_str.contains("geometry") => {
                ShaderType::Geometry(module)
            }
            Some(path_str) if path_str.contains("tess_ctrl") => {
                ShaderType::TesselationControl(module)
            }
            Some(path_str) if path_str.contains("tess_eval") => {
                ShaderType::TesselationEval(module)
            }
            Some(path_str) if path_str.contains("compute") => ShaderType::Compute(module),

            Some(_) => unreachable!("Shaders must be one of fragment, vertex, geometry, tesselation control, \
                tesselation evaluation, or compute, and must be in an appropriately named subdirectory."),
            None => {
                unreachable!("There should be no path that has allowed me to open a file and read it that then has no path.");
            }
        };
    }
}
