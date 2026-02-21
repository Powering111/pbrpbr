use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

#[derive(Clone, Debug)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    pub fn rot(&self) -> Mat3 {
        Mat3::from_quat(self.rotation) * Mat3::from_diagonal(self.scale.recip())
    }
}

impl core::fmt::Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Translation: {}", self.translation)?;
        writeln!(f, "Rotation: {}", self.rotation)?;
        writeln!(f, "Scale: {}", self.scale)?;
        Ok(())
    }
}

impl From<gltf::scene::Transform> for Transform {
    fn from(value: gltf::scene::Transform) -> Self {
        let decomposed = value.decomposed();
        Self {
            translation: decomposed.0.into(),
            rotation: Quat::from_array(decomposed.1),
            scale: decomposed.2.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Primitive {
    pub vertices: Vec<crate::renderer::Vertex>,
    pub indices: Vec<u32>,
    pub material: Material,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Material {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub name: Option<String>,
    pub transform: Transform,
    pub primitives: Vec<Primitive>,
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,

    pub yfov: f32,
    pub zfar: Option<f32>,
    pub znear: f32,
}

impl Camera {
    pub fn get_matrix(&self, aspect_ratio: f32) -> Mat4 {
        (match self.zfar {
            Some(zfar) => Mat4::perspective_rh(self.yfov, aspect_ratio, self.znear, zfar),
            None => Mat4::perspective_infinite_rh(self.yfov, aspect_ratio, self.znear),
        }) * Mat4::look_to_rh(self.position, self.direction(), self.up_vec())
    }

    fn direction(&self) -> Vec3 {
        Quat::from_euler(glam::EulerRot::ZXYEx, self.roll, self.pitch, self.yaw) * Vec3::NEG_Z
    }

    fn up_vec(&self) -> Vec3 {
        Quat::from_euler(glam::EulerRot::ZXYEx, self.roll, self.pitch, self.yaw) * Vec3::Y
    }

    pub fn forward_vec(&self) -> Vec3 {
        Vec3::NEG_Z.rotate_axis(Vec3::Y, self.yaw)
    }

    fn yaw_pitch_roll(quat: Quat) -> (f32, f32, f32) {
        let (roll, pitch, yaw) = quat.to_euler(glam::EulerRot::ZXYEx);
        (yaw, pitch, roll)
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Light {
    pub pos: Vec3,
    pub typ: u32,
    pub color: Vec3,
    pub radiant_flux: f32,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub camera: Camera,
    pub lights: Vec<Light>,
    pub meshes: Vec<Mesh>,
}
impl Scene {
    pub fn from_glb(path: &str) -> Result<Self, ()> {
        let file = std::fs::File::open(path).map_err(|_| ())?;
        let reader = std::io::BufReader::new(file);
        let gltf = gltf::Gltf::from_reader(reader).map_err(|_| ())?;

        let visitor = Visitor::visit(gltf);

        assert!(visitor.lights.len() <= 4);
        Ok(Self {
            camera: visitor.camera.unwrap_or(Camera {
                position: Vec3::ZERO,
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
                yfov: 1.0,
                zfar: None,
                znear: 0.001,
            }),
            lights: visitor.lights,
            meshes: visitor.meshes,
        })
    }
}

impl core::fmt::Display for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Scene")?;

        writeln!(f, "Camera")?;
        writeln!(
            f,
            "position: {}, yaw: {}, pitch: {}, roll: {}",
            self.camera.position, self.camera.yaw, self.camera.pitch, self.camera.roll
        )?;

        for mesh in self.meshes.iter() {
            writeln!(
                f,
                "\"{}\" - {} primitive{}",
                mesh.name.as_deref().unwrap_or(""),
                mesh.primitives.len(),
                if mesh.primitives.len() > 2 { "s" } else { "" }
            )?;
            writeln!(f, "{}", mesh.transform,)?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct Visitor {
    camera: Option<Camera>,
    lights: Vec<Light>,
    meshes: Vec<Mesh>,
}

impl Visitor {
    pub fn visit(gltf: gltf::Gltf) -> Self {
        let mut buffer_data = Vec::new();
        for buffer in gltf.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Bin => {
                    if let Some(blob) = gltf.blob.as_deref() {
                        buffer_data.push(blob);
                    };
                }
                gltf::buffer::Source::Uri(_) => todo!(),
            }
        }

        let mut visitor = Self::default();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                visitor.do_visit(&buffer_data, &node);
            }
        }

        visitor
    }

    fn do_visit(&mut self, buffer_data: &[&[u8]], node: &gltf::Node) {
        let transform: Transform = node.transform().into();

        if let Some(mesh) = node.mesh() {
            let mut primitives = Vec::new();
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()]));
                let positions = reader.read_positions().unwrap();
                let normals = reader.read_normals().unwrap();
                let indices = reader.read_indices().unwrap();
                assert_eq!(positions.len(), normals.len());

                let i_material = primitive.material();

                let pbr_metallic_roughness = i_material.pbr_metallic_roughness();
                let base_color = pbr_metallic_roughness.base_color_factor();
                let metallic = pbr_metallic_roughness.metallic_factor();
                let roughness = pbr_metallic_roughness.roughness_factor();
                assert!(
                    i_material
                        .pbr_metallic_roughness()
                        .metallic_roughness_texture()
                        .is_none()
                );
                let material = Material {
                    base_color: base_color.into(),
                    metallic,
                    roughness,
                };

                primitives.push(Primitive {
                    vertices: positions
                        .zip(normals)
                        .map(|(position, normal)| crate::renderer::Vertex {
                            position: position.into(),
                            normal: normal.into(),
                        })
                        .collect(),
                    indices: indices.into_u32().collect(),
                    material,
                })
            }
            self.meshes.push(Mesh {
                name: node.name().map(|a| a.to_owned()),
                transform: node.transform().into(),
                primitives,
            });
        }

        if let Some(camera) = node.camera() {
            self.camera = match camera.projection() {
                gltf::camera::Projection::Orthographic(_orthographic) => {
                    todo!("Orthographic camera")
                }
                gltf::camera::Projection::Perspective(perspective) => {
                    let (yaw, pitch, roll) = Camera::yaw_pitch_roll(transform.rotation);
                    Some(Camera {
                        position: transform.translation,
                        yaw,
                        pitch,
                        roll,

                        yfov: perspective.yfov(),
                        zfar: perspective.zfar(),
                        znear: perspective.znear(),
                    })
                }
            }
        }

        if let Some(light) = node.light() {
            let radiant_flux = light.intensity() * 4.0 * std::f32::consts::PI / 683.0;
            let color = light.color().into();
            match light.kind() {
                gltf::khr_lights_punctual::Kind::Point => self.lights.push(Light {
                    typ: 1,
                    pos: transform.translation,
                    color,
                    radiant_flux,
                }),
                gltf::khr_lights_punctual::Kind::Directional => self.lights.push(Light {
                    typ: 2,
                    pos: transform.rotation * Vec3::NEG_Z,
                    color: light.color().into(),
                    radiant_flux,
                }),
                _ => todo!(),
            }
        }

        for child in node.children() {
            self.do_visit(buffer_data, &child);
        }
    }
}
