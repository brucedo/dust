use std::f64;

trait Vulkanic {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>);
    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize;
}
pub struct F32Vertex3 {
    vertex: [f32; 3],
}

impl Vulkanic for F32Vertex3 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[f32; 3]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);

        size_of::<[f32; 3]>()
    }
}

pub struct F32Vertex2 {
    vertex: [f32; 2],
}

impl Vulkanic for F32Vertex2 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[f32; 2]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);

        size_of::<[f32; 2]>()
    }
}

pub struct F32Vertex4 {
    vertex: [f32; 4],
}

impl Vulkanic for F32Vertex4 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[f32; 4]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[f32; 4]>()
    }
}

pub struct U8Vertex2 {
    vertex: [u8; 2],
}

impl Vulkanic for U8Vertex2 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[u8; 2]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[u8; 2]>()
    }
}
pub struct U8Vertex3 {
    vertex: [u8; 3],
}

impl Vulkanic for U8Vertex3 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[u8; 3]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[u8; 3]>()
    }
}
pub struct U8Vertex4 {
    vertex: [u8; 3],
}
impl Vulkanic for U8Vertex4 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[u8; 4]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[u8; 4]>()
    }
}

pub struct F64Vertex2 {
    vertex: [f64; 2],
}
impl Vulkanic for F64Vertex2 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[f64; 2]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[f64; 2]>()
    }
}

pub struct F64Vertex3 {
    vertex: [f64; 3],
}
impl Vulkanic for F64Vertex3 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[f64; 3]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[f64; 3]>()
    }
}

pub struct F64Vertex4 {
    vertex: [f64; 4],
}
impl Vulkanic for F64Vertex4 {
    fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .for_each(|byte| buffer.push(byte));
    }

    fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
        assert!(buffer.len() >= start + size_of::<[f64; 4]>());

        self.vertex
            .iter()
            .flat_map(|component| component.to_ne_bytes())
            .enumerate()
            .for_each(|(index, byte)| buffer[start + index] = byte);
        size_of::<[f64; 4]>()
    }
}

pub mod sampled_vertex_3 {
    pub struct SampledVertex3 {
        texture_coord: super::F32Vertex2,
        vertex: super::F32Vertex3,
    }

    impl super::Vulkanic for SampledVertex3 {
        fn copy_into_vk_buffer(&self, buffer: &mut [u8], start: usize) -> usize {
            let texture_size = self.texture_coord.copy_into_vk_buffer(buffer, start);
            let vertex_size = self
                .vertex
                .copy_into_vk_buffer(buffer, start + size_of_val(&self.texture_coord.vertex));

            texture_size + vertex_size
        }
        fn copy_into_vk_vec(&self, buffer: &mut Vec<u8>) {
            self.texture_coord.copy_into_vk_vec(buffer);
            self.vertex.copy_into_vk_vec(buffer);
        }
    }

    pub fn new(texture_coord: [f32; 2], vertex: [f32; 3]) -> SampledVertex3 {
        SampledVertex3 {
            texture_coord,
            vertex,
        }
    }
}
