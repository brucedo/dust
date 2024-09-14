use std::{
    fs::File,
    io::{Error, Read},
    os::unix::fs::MetadataExt,
};

pub fn load_shader(file_name: File) -> Result<Vec<u32>, Error> {
    let file_data = file_name.metadata()?;

    let mut quad_count = file_data.size() / 4;
    if quad_count % 4 != 0 {
        quad_count += 1;
    }

    let shader_bytes = Vec::with_capacity(quad_count as usize);
    let raw_bytes = Vec::<u8>::with_capacity(file_data.size() as usize);

    Ok(shader_bytes)
}
