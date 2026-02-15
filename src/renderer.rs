use glam::{Mat4, Vec3};

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: Vec3,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Instance {
    pub model: Mat4,
}

impl Instance {
    const ATTRIBS: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBS,
        }
    }
}

pub(crate) struct VertexBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
}

impl VertexBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vertex buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: 0,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("index buffer"),
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            size: 0,
            mapped_at_creation: false,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: 0,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
        }
    }

    pub fn write(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertices: &[Vertex],
        indices: &[u32],
        instances: &[Instance],
    ) {
        if self.vertex_buffer.size() < vertices.len() as u64 {
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("vertex buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: size_of_val(vertices) as u64,
                mapped_at_creation: false,
            });
        }
        if self.index_buffer.size() < indices.len() as u64 {
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("index buffer"),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                size: size_of_val(indices) as u64,
                mapped_at_creation: false,
            });
        }
        if self.instance_buffer.size() < instances.len() as u64 {
            self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("instance buffer"),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                size: size_of_val(instances) as u64,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(indices));
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instances));
    }

    pub fn set(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    }
}

pub(crate) struct Uniform {
    buffer: wgpu::Buffer,
    pub bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Uniform {
    pub(crate) fn new(device: &wgpu::Device, size: u64) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size: None,
                }),
            }],
        });
        Self {
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub(crate) fn write(&self, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_buffer(&self.buffer, 0, data);
    }

    pub(crate) fn set(&self, render_pass: &mut wgpu::RenderPass, bind_group_index: u32) {
        render_pass.set_bind_group(bind_group_index, &self.bind_group, &[]);
    }
}
