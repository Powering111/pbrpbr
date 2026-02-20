use glam::{Mat3, Mat4, Vec3};

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

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
    pub rot: Mat3,
}

impl Instance {
    const ATTRIBS: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4,
        6 => Float32x3, 7 => Float32x3, 8 => Float32x3
    ];

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

pub(crate) struct UniformGroup {
    sizes: Vec<u64>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_groups: Vec<(Vec<wgpu::Buffer>, wgpu::BindGroup)>,
}

impl UniformGroup {
    // each entry of `sizes` represent size of each binding
    pub(crate) fn new(device: &wgpu::Device, sizes: &[u64]) -> Self {
        let layout_entries: Vec<wgpu::BindGroupLayoutEntry> = (0..sizes.len())
            .map(|i| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &layout_entries,
        });

        Self {
            sizes: sizes.to_owned(),
            bind_group_layout,
            bind_groups: Vec::new(),
        }
    }

    // create bindable bind group. returns bind group id.
    pub(crate) fn add_bind_group(&mut self, device: &wgpu::Device) -> u64 {
        let buffers: Vec<wgpu::Buffer> = self
            .sizes
            .iter()
            .map(|size| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: *size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
                    mapped_at_creation: false,
                })
            })
            .collect();

        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = Vec::new();
        for (i, _size) in self.sizes.iter().enumerate() {
            entries.push(wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffers[i],
                    offset: 0,
                    size: None,
                }),
            });
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &entries,
        });
        self.bind_groups.push((buffers, bind_group));
        (self.bind_groups.len() - 1) as u64
    }

    pub(crate) fn has_bind_group(&self, bind_group_id: u64) -> bool {
        bind_group_id < self.bind_groups.len() as u64
    }

    // write entire bind group.
    // each entry of `data` represent each binding.
    pub(crate) fn write(&self, queue: &wgpu::Queue, bind_group_id: u64, data: &[&[u8]]) {
        for (i, (_size, data_entry)) in self.sizes.iter().zip(data).enumerate() {
            queue.write_buffer(
                &self.bind_groups[bind_group_id as usize].0[i],
                0,
                data_entry,
            );
        }
    }

    pub(crate) fn set(
        &self,
        render_pass: &mut wgpu::RenderPass,
        bind_group_index: u32,
        bind_group_id: u64,
    ) {
        render_pass.set_bind_group(
            bind_group_index,
            &self.bind_groups[bind_group_id as usize].1,
            &[],
        );
    }
}
