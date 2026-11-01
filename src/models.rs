use macroquad::prelude::*;

pub struct Robot {
    meshes: Vec<Mesh>,
}
impl Robot {
    // Import static bind-pose geometry, including each node's scene transform.
    pub fn load(path: &str, color: Color) -> Result<Self, String> {
        let (doc, buffers, _) = gltf::import(path).map_err(|e| e.to_string())?;
        let mut meshes = vec![];
        fn visit(
            node: gltf::Node<'_>,
            parent: Mat4,
            buffers: &[gltf::buffer::Data],
            out: &mut Vec<Mesh>,
        ) -> Result<(), String> {
            let transform = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    if primitive.mode() != gltf::mesh::Mode::Triangles {
                        continue;
                    }
                    let reader = primitive.reader(|b| Some(&buffers[b.index()].0));
                    let positions: Vec<_> = reader
                        .read_positions()
                        .ok_or("Model nema pozicie")?
                        .collect();
                    let indices: Vec<u32> = reader
                        .read_indices()
                        .map(|i| i.into_u32().collect())
                        .unwrap_or_else(|| (0..positions.len() as u32).collect());
                    if !indices.len().is_multiple_of(3) {
                        return Err("Neplatne trojuholniky modelu".into());
                    }
                    // Split triangles to fit macroquad's u16 index buffers and batch limits.
                    for chunk in indices.chunks(3000) {
                        let mut vertices = Vec::with_capacity(chunk.len());
                        for &i in chunk {
                            let p = positions.get(i as usize).ok_or("Neplatny index modelu")?;
                            vertices.push(Vertex::new2(
                                transform.transform_point3(Vec3::from_array(*p)),
                                Vec2::ZERO,
                                WHITE,
                            ));
                        }
                        out.push(Mesh {
                            indices: (0..vertices.len() as u16).collect(),
                            vertices,
                            texture: None,
                        });
                    }
                }
            }
            for child in node.children() {
                visit(child, transform, buffers, out)?
            }
            Ok(())
        }
        let scene = doc
            .default_scene()
            .or_else(|| doc.scenes().next())
            .ok_or("Model nema scenu")?;
        for n in scene.nodes() {
            visit(n, Mat4::IDENTITY, &buffers, &mut meshes)?
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for m in &meshes {
            for v in &m.vertices {
                min = min.min(v.position);
                max = max.max(v.position);
            }
        }
        if !min.is_finite() || max.y - min.y < 0.001 {
            return Err("Model ma neplatne rozmery".into());
        }
        let scale = 1.8 / (max.y - min.y);
        let center = vec3((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
        for m in &mut meshes {
            for triangle in m.vertices.chunks_mut(3) {
                let normal = (triangle[1].position - triangle[0].position)
                    .cross(triangle[2].position - triangle[0].position)
                    .normalize_or_zero();
                let light = 0.45 + 0.55 * normal.dot(vec3(-0.4, 0.8, 0.5).normalize()).abs();
                let tint = Color::new(color.r * light, color.g * light, color.b * light, 1.);
                for v in triangle {
                    v.position = (v.position - center) * scale;
                    v.color = tint.into();
                }
            }
        }
        Ok(Self { meshes })
    }
    pub fn draw(&self, pos: Vec3, facing: f32) {
        let transform = Mat4::from_translation(pos)
            * Mat4::from_rotation_y(-facing + std::f32::consts::FRAC_PI_2);
        unsafe {
            get_internal_gl().quad_gl.push_model_matrix(transform);
        }
        for m in &self.meshes {
            // Team tint makes untextured Mixamo bind poses distinguishable.
            draw_mesh(m);
        }
        unsafe {
            get_internal_gl().quad_gl.pop_model_matrix();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_mixamo_models_import() {
        for path in ["assets/x_bot.glb", "assets/y_bot.glb"] {
            let robot = Robot::load(path, WHITE).unwrap();
            assert!(!robot.meshes.is_empty());
            assert!(robot
                .meshes
                .iter()
                .all(|m| m.vertices.iter().all(|v| v.position.is_finite()
                    && v.position.y >= -0.001
                    && v.position.y <= 1.801)));
        }
    }
}
