use macroquad::prelude::*;

/// Static GLB geometry with node transforms and original material colors.
pub struct BallModel {
    meshes: Vec<Mesh>,
}
impl BallModel {
    pub fn load(path: &str) -> Result<Self, String> {
        let (doc, buffers, _) = crate::asset_data::import(path).map_err(|e| e.to_string())?;
        fn visit(
            node: gltf::Node<'_>,
            parent: Mat4,
            buffers: &[gltf::buffer::Data],
            meshes: &mut Vec<Mesh>,
        ) -> Result<(), String> {
            let matrix = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    if primitive.mode() != gltf::mesh::Mode::Triangles {
                        continue;
                    }
                    let reader = primitive.reader(|b| Some(&buffers[b.index()].0));
                    let positions: Vec<_> = reader
                        .read_positions()
                        .ok_or("Lopta nema vrcholy")?
                        .map(|p| matrix.transform_point3(Vec3::from_array(p)))
                        .collect();
                    let indices: Vec<_> = reader
                        .read_indices()
                        .map(|i| i.into_u32().collect())
                        .unwrap_or_else(|| (0..positions.len() as u32).collect());
                    if !indices.len().is_multiple_of(3) {
                        return Err("Neplatna geometria lopty".into());
                    }
                    let color = Color::from(
                        primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_factor(),
                    );
                    for chunk in indices.chunks(3000) {
                        let mut vertices = Vec::with_capacity(chunk.len());
                        for &i in chunk {
                            vertices.push(Vertex::new2(
                                *positions.get(i as usize).ok_or("Neplatny index lopty")?,
                                Vec2::ZERO,
                                color,
                            ));
                        }
                        for triangle in vertices.chunks_mut(3) {
                            let normal = (triangle[1].position - triangle[0].position)
                                .cross(triangle[2].position - triangle[0].position)
                                .normalize_or_zero();
                            let light =
                                0.55 + 0.45 * normal.dot(vec3(-0.4, 0.8, 0.5).normalize()).abs();
                            for v in triangle {
                                v.color = Color::new(
                                    color.r * light,
                                    color.g * light,
                                    color.b * light,
                                    color.a,
                                )
                                .into();
                            }
                        }
                        meshes.push(Mesh {
                            indices: (0..vertices.len() as u16).collect(),
                            vertices,
                            texture: None,
                        });
                    }
                }
            }
            for child in node.children() {
                visit(child, matrix, buffers, meshes)?
            }
            Ok(())
        }
        let scene = doc
            .default_scene()
            .or_else(|| doc.scenes().next())
            .ok_or("Lopta nema scenu")?;
        let mut meshes = vec![];
        for node in scene.nodes() {
            visit(node, Mat4::IDENTITY, &buffers, &mut meshes)?
        }
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for mesh in &meshes {
            for v in &mesh.vertices {
                min = min.min(v.position);
                max = max.max(v.position);
            }
        }
        let center = (min + max) * 0.5;
        let radius = meshes
            .iter()
            .flat_map(|m| &m.vertices)
            .map(|v| v.position.distance(center))
            .fold(0_f32, f32::max);
        if !center.is_finite() || !radius.is_finite() || radius < 0.0001 {
            return Err("Neplatne rozmery lopty".into());
        }
        for mesh in &mut meshes {
            for v in &mut mesh.vertices {
                v.position = (v.position - center) * (crate::game::BALL_RADIUS / radius);
            }
        }
        Ok(Self { meshes })
    }
    pub fn draw(&self, pos: Vec3, rotation: Quat) {
        unsafe {
            get_internal_gl()
                .quad_gl
                .push_model_matrix(Mat4::from_rotation_translation(rotation, pos));
        }
        for mesh in &self.meshes {
            draw_mesh(mesh);
        }
        unsafe {
            get_internal_gl().quad_gl.pop_model_matrix();
        }
    }
}

pub struct BallRolling {
    previous: Option<Vec3>,
    round: u32,
    pub rotation: Quat,
}
impl Default for BallRolling {
    fn default() -> Self {
        Self {
            previous: None,
            round: 0,
            rotation: Quat::IDENTITY,
        }
    }
}
impl BallRolling {
    pub fn update(&mut self, position: Vec3, round: u32, active: bool) {
        if self.round != round || !active || self.previous.is_none() {
            self.previous = Some(position);
            self.round = round;
            self.rotation = Quat::IDENTITY;
            return;
        }
        let delta = position - self.previous.unwrap();
        self.previous = Some(position);
        let travel = vec3(delta.x, 0., delta.z);
        let distance = travel.length();
        if distance > 0.000001 {
            let axis = Vec3::Y.cross(travel) / distance;
            // Rolling without slip: angle = distance / radius, applied about
            // a world-space axis perpendicular to the actual movement.
            self.rotation = (Quat::from_axis_angle(axis, distance / crate::game::BALL_RADIUS)
                * self.rotation)
                .normalize();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rolling_follows_distance_reverses_and_stops() {
        let mut rolling = BallRolling::default();
        rolling.update(Vec3::ZERO, 1, true);
        let step = crate::game::BALL_RADIUS * std::f32::consts::FRAC_PI_2;
        rolling.update(Vec3::X * step, 1, true);
        assert!((rolling.rotation * Vec3::Y).distance(Vec3::X) < 0.0001);
        let orientation = rolling.rotation;
        rolling.update(Vec3::X * step, 1, true);
        assert_eq!(orientation, rolling.rotation);
        rolling.update(Vec3::ZERO, 1, true);
        assert!((rolling.rotation * Vec3::Y).distance(Vec3::Y) < 0.0001);
    }
    #[test]
    fn turns_accumulate_about_world_axes_and_round_reset_does_not_spin() {
        let mut rolling = BallRolling::default();
        rolling.update(Vec3::ZERO, 1, true);
        let step = crate::game::BALL_RADIUS * std::f32::consts::FRAC_PI_2;
        rolling.update(Vec3::X * step, 1, true);
        rolling.update((Vec3::X + Vec3::Z) * step, 1, true);
        let expected = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
            * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2);
        assert!((rolling.rotation * Vec3::Z).distance(expected * Vec3::Z) < 0.0001);
        rolling.update(Vec3::ZERO, 2, false);
        assert_eq!(rolling.rotation, Quat::IDENTITY);
    }
    #[test]
    fn supplied_ball_preserves_panels_and_matches_collision_radius() {
        let ball = BallModel::load("assets/models/ball.glb").unwrap();
        let vertices: Vec<_> = ball.meshes.iter().flat_map(|m| &m.vertices).collect();
        assert!(vertices.len() > 100);
        assert!(vertices
            .iter()
            .all(|v| v.position.is_finite()
                && v.position.length() <= crate::game::BALL_RADIUS + 0.0001));
        assert!(vertices.iter().any(|v| v.color[0] < 10));
        assert!(vertices.iter().any(|v| v.color[0] > 150));
    }
}
