use ash::vk::Result;

#[derive(Debug)]
pub enum DustError {
    NoMatchingMemoryType,
    CreateShaderModuleFailed(Result),
}
