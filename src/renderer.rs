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

pub(crate) struct Renderer {
    render_pipeline: wgpu::RenderPipeline,
    render_pipeline_shadow_map: wgpu::RenderPipeline,
    render_pipeline_full: wgpu::RenderPipeline,
    pub depth_texture: crate::texture::Texture,
    shadow_maps: Vec<(crate::texture::Texture, wgpu::BindGroup)>,
    sampler: wgpu::Sampler,
    depth_bind_group: wgpu::BindGroup,

    vertex_buffer: VertexBuffer,
    scene_uniform: UniformGroup,
    primitive_uniform: UniformGroup,

    draws: Vec<Draw>,
    width: u32,
    height: u32,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, surface_configuration: &wgpu::SurfaceConfiguration) -> Self {
        let depth_texture = crate::texture::Texture::create_depth_texture(
            device,
            surface_configuration.width,
            surface_configuration.height,
        );

        // Uniforms
        let mut scene_uniform = UniformGroup::new(
            device,
            &[
                size_of::<Mat4>() as u64,
                size_of::<Vec3>() as u64,
                4 * size_of::<crate::model::LightRaw>() as u64,
                size_of::<Vec3>() as u64,
            ],
        );
        for _ in 0..5 {
            scene_uniform.add_bind_group(device);
        }
        let primitive_uniform = UniformGroup::new(device, &[32]);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader/shader.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &scene_uniform.bind_group_layout,
                &primitive_uniform.bind_group_layout,
            ],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("3D"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_configuration.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        let render_pipeline_shadow_map =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Shadow map"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Vertex::desc(), Instance::desc()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_noop"),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        let bind_group_layout_full =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });
        let pipeline_layout_full = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout_full],
            immediate_size: 0,
        });
        let render_pipeline_full = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Full"),
            layout: Some(&pipeline_layout_full),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_full"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_full"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_configuration.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });
        let vertex_buffer = VertexBuffer::new(device);

        let shadow_maps: Vec<(crate::texture::Texture, wgpu::BindGroup)> = (0..4)
            .map(|_| {
                let texture = crate::texture::Texture::create_depth_texture(device, 1024, 1024);
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &bind_group_layout_full,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&texture.view),
                        },
                    ],
                });
                (texture, bind_group)
            })
            .collect();

        let depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout_full,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth_texture.view),
                },
            ],
        });

        Self {
            render_pipeline,
            render_pipeline_shadow_map,
            render_pipeline_full,

            depth_texture,
            shadow_maps,
            sampler,
            depth_bind_group,

            vertex_buffer,
            scene_uniform,
            primitive_uniform,
            draws: Vec::new(),
            width: surface_configuration.width,
            height: surface_configuration.height,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.depth_texture = crate::texture::Texture::create_depth_texture(device, width, height);
    }

    pub fn write_vertex(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &crate::model::Scene,
    ) {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut instances: Vec<Instance> = Vec::new();
        self.draws = Vec::new();
        for mesh in scene.meshes.iter() {
            for primitive in mesh.primitives.iter() {
                let instance_num = instances.len() as u32;
                while !self.primitive_uniform.has_bind_group(instance_num as u64) {
                    self.primitive_uniform.add_bind_group(device);
                }
                self.primitive_uniform.write(
                    queue,
                    instance_num as u64,
                    &[bytemuck::cast_slice(&[primitive.material])],
                );

                let base_index = vertices.len() as i32;
                self.draws.push(Draw {
                    index_start: indices.len() as u32,
                    index_end: indices.len() as u32 + primitive.indices.len() as u32,
                    base_index,
                    instance_num,
                });
                vertices.extend_from_slice(primitive.vertices.as_slice());
                indices.extend_from_slice(primitive.indices.as_slice());

                instances.push(Instance {
                    model: mesh.transform.matrix(),
                    rot: mesh.transform.rot(),
                });
            }
        }
        self.vertex_buffer
            .write(device, queue, &vertices, &indices, &instances);
    }

    pub fn render_shadow_map(
        &self,
        queue: &wgpu::Queue,
        shadow_map_view: &wgpu::TextureView,
        command_encoder: &mut wgpu::CommandEncoder,
        light: &crate::model::Light,
        idx: u64,
    ) {
        let camera_matrix = light.matrix();
        self.scene_uniform.write(
            queue,
            idx,
            &[bytemuck::cast_slice(&[camera_matrix]), &[], &[]],
        );

        // println!("0,0,0: {:?}", camera_matrix * glam::Vec4::new(0.0, 0.0, 0.0, 1.0));

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: shadow_map_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline_shadow_map);

        self.vertex_buffer.set(&mut render_pass);
        self.scene_uniform.set(&mut render_pass, 0, idx);

        for Draw {
            index_start,
            index_end,
            base_index,
            instance_num,
        } in self.draws.iter()
        {
            self.primitive_uniform
                .set(&mut render_pass, 1, *instance_num as u64);
            render_pass.draw_indexed(
                *index_start..*index_end,
                *base_index,
                *instance_num..*instance_num + 1,
            );
        }
    }

    pub fn render(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        queue: &wgpu::Queue,
        scene: &crate::model::Scene,
    ) {
        for (i, light) in scene.lights.iter().enumerate() {
            self.render_shadow_map(
                queue,
                &self.shadow_maps[i].0.view,
                command_encoder,
                light,
                i as u64 + 1,
            );
        }

        let aspect_ratio = self.width as f32 / self.height as f32;
        let camera_matrix = scene.camera.get_matrix(aspect_ratio);

        // println!("camera: {:?}", scene.camera);
        // println!("dir: {:?}", scene.camera.direction());
        // println!("0,0,0: {:?}", camera_matrix * glam::Vec4::new(0.0, 0.0, 0.0, 1.0));
        // let k = scene.camera.direction() * 1.0;
        // println!("+1: {:?}", camera_matrix * glam::Vec4::new(k.x, k.y, k.z, 1.0));

        let lights: Vec<crate::model::LightRaw> =
            scene.lights.iter().map(|light| light.raw()).collect();
        self.scene_uniform.write(
            queue,
            0,
            &[
                bytemuck::cast_slice(&[camera_matrix]),
                bytemuck::cast_slice(&[scene.camera.position]),
                bytemuck::cast_slice(&lights),
            ],
        );

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline);

        self.vertex_buffer.set(&mut render_pass);
        self.scene_uniform.set(&mut render_pass, 0, 0);

        for Draw {
            index_start,
            index_end,
            base_index,
            instance_num,
        } in self.draws.iter()
        {
            self.primitive_uniform
                .set(&mut render_pass, 1, *instance_num as u64);
            render_pass.draw_indexed(
                *index_start..*index_end,
                *base_index,
                *instance_num..*instance_num + 1,
            );
        }

        drop(render_pass);
        // return;

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        render_pass.set_pipeline(&self.render_pipeline_full);
        render_pass.set_bind_group(0, &self.shadow_maps[0].1, &[]);
        // render_pass.set_bind_group(0, &self.depth_bind_group, &[]);

        render_pass.draw(0..6, 0..1);
    }
}

pub struct Draw {
    pub index_start: u32,
    pub index_end: u32,
    pub base_index: i32,
    pub instance_num: u32,
}
