use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Debug)]
pub struct Transform {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
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
}

#[derive(Clone, Debug)]
pub struct Model {
    pub name: Option<String>,
    pub transform: Transform,
    pub primitives: Vec<Primitive>,
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub transform: Transform,
    pub yfov: f32,
    pub zfar: Option<f32>,
    pub znear: f32,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub camera: Camera,
    pub models: Vec<Model>,
}
impl Scene {
    pub fn from_glb(path: &str) -> Result<Self, ()> {
        let file = std::fs::File::open(path).map_err(|_| ())?;
        let reader = std::io::BufReader::new(file);
        let gltf = gltf::Gltf::from_reader(reader).map_err(|_| ())?;

        let visitor = Visitor::visit(gltf);

        Ok(Self {
            camera: visitor.camera.expect("scene need at least one camera"),
            models: visitor.models,
        })
    }
}

impl core::fmt::Display for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Scene")?;

        writeln!(f, "Camera")?;
        writeln!(f, "{}", self.camera.transform)?;

        for model in self.models.iter() {
            writeln!(
                f,
                "\"{}\" - {} primitive{}",
                model.name.as_deref().unwrap_or(""),
                model.primitives.len(),
                if model.primitives.len() > 2 { "s" } else { "" }
            )?;
            writeln!(f, "{}", model.transform,)?;
        }
        Ok(())
    }
}

#[derive(Default)]
struct Visitor {
    camera: Option<Camera>,
    models: Vec<Model>,
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
            println!();
            for node in scene.nodes() {
                visitor.do_visit(&buffer_data, &node);
            }
        }

        visitor
    }

    fn do_visit(&mut self, buffer_data: &[&[u8]], node: &gltf::Node) {
        if let Some(mesh) = node.mesh() {
            let mut primitives = Vec::new();
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()]));
                let positions = reader.read_positions().unwrap();
                let indices = reader.read_indices().unwrap();

                primitives.push(Primitive {
                    vertices: positions
                        .map(|position| crate::renderer::Vertex {
                            position: position.into(),
                        })
                        .collect(),
                    indices: indices.into_u32().collect(),
                })
            }
            self.models.push(Model {
                name: node.name().map(|a| a.to_owned()),
                transform: node.transform().into(),
                primitives,
            });
        }

        if let Some(camera) = node.camera() {
            self.camera = match camera.projection() {
                gltf::camera::Projection::Orthographic(_orthographic) => todo!(),
                gltf::camera::Projection::Perspective(perspective) => Some(Camera {
                    transform: node.transform().into(),
                    yfov: perspective.yfov(),
                    zfar: perspective.zfar(),
                    znear: perspective.znear(),
                }),
            }
        }

        for child in node.children() {
            self.do_visit(buffer_data, &child);
        }
    }
}
