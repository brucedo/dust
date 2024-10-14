use ash::vk::Result;

#[derive(Debug)]
pub enum DustError {
    NoMatchingMemoryType,
    DeviceMemoryAllocationFailed(Result),
    CreateShaderModuleFailed(Result),
}
