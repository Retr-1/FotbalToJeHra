use macroquad::prelude::*;
use std::{cell::RefCell, collections::HashMap};

#[derive(Clone, Copy)]
struct Pose {
    translation: Vec3,
    rotation: Quat,
    scale: Vec3,
}
impl Pose {
    fn matrix(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}
struct Node {
    name: String,
    parent: Option<usize>,
    rest: Pose,
}
struct Rig {
    nodes: Vec<Node>,
    order: Vec<usize>,
    bind: Vec<Mat4>,
}
impl Rig {
    fn read(doc: &gltf::Document) -> Self {
        let mut nodes: Vec<_> = doc
            .nodes()
            .map(|n| {
                let (t, r, s) = n.transform().decomposed();
                Node {
                    name: n.name().unwrap_or("").to_string(),
                    parent: None,
                    rest: Pose {
                        translation: Vec3::from_array(t),
                        rotation: Quat::from_array(r).normalize(),
                        scale: Vec3::from_array(s),
                    },
                }
            })
            .collect();
        for n in doc.nodes() {
            for child in n.children() {
                nodes[child.index()].parent = Some(n.index());
            }
        }
        fn visit(i: usize, nodes: &[Node], order: &mut Vec<usize>) {
            order.push(i);
            for child in 0..nodes.len() {
                if nodes[child].parent == Some(i) {
                    visit(child, nodes, order)
                }
            }
        }
        let mut order = vec![];
        for i in 0..nodes.len() {
            if nodes[i].parent.is_none() {
                visit(i, &nodes, &mut order)
            }
        }
        let mut rig = Self {
            bind: vec![Mat4::IDENTITY; nodes.len()],
            nodes,
            order,
        };
        rig.bind = rig.world(&rig.nodes.iter().map(|n| n.rest).collect::<Vec<_>>());
        rig
    }
    fn world(&self, poses: &[Pose]) -> Vec<Mat4> {
        let mut world = vec![Mat4::IDENTITY; self.nodes.len()];
        for &i in &self.order {
            world[i] =
                self.nodes[i].parent.map_or(Mat4::IDENTITY, |p| world[p]) * poses[i].matrix();
        }
        world
    }
}
#[derive(Clone)]
enum Values {
    Translation(Vec<Vec3>),
    Rotation(Vec<Quat>),
    Scale(Vec<Vec3>),
}
struct Track {
    node: usize,
    times: Vec<f32>,
    values: Values,
    step: bool,
}
struct Clip {
    tracks: Vec<Track>,
    duration: f32,
}
impl Clip {
    fn read(
        animation: gltf::Animation<'_>,
        buffers: &[gltf::buffer::Data],
    ) -> Result<Self, String> {
        let mut tracks = vec![];
        let mut duration = 0_f32;
        for channel in animation.channels() {
            if channel.sampler().interpolation() == gltf::animation::Interpolation::CubicSpline {
                return Err("Animacia vyzaduje nepodporovanu interpolaciu CUBICSPLINE".into());
            }
            let reader = channel.reader(|b| Some(&buffers[b.index()].0));
            let times: Vec<_> = reader
                .read_inputs()
                .ok_or("Animacii chybaju casy")?
                .collect();
            use gltf::animation::util::ReadOutputs;
            let values = match reader.read_outputs().ok_or("Animacii chybaju hodnoty")? {
                ReadOutputs::Translations(v) => {
                    Values::Translation(v.map(Vec3::from_array).collect())
                }
                ReadOutputs::Rotations(v) => Values::Rotation(
                    v.into_f32()
                        .map(|q| Quat::from_array(q).normalize())
                        .collect(),
                ),
                ReadOutputs::Scales(v) => Values::Scale(v.map(Vec3::from_array).collect()),
                _ => continue,
            };
            let count = match &values {
                Values::Translation(v) | Values::Scale(v) => v.len(),
                Values::Rotation(v) => v.len(),
            };
            if times.is_empty() || times.len() != count {
                return Err("Neplatne kluce animacie".into());
            }
            duration = duration.max(*times.last().unwrap());
            tracks.push(Track {
                node: channel.target().node().index(),
                times,
                values,
                step: channel.sampler().interpolation() == gltf::animation::Interpolation::Step,
            });
        }
        Ok(Self { tracks, duration })
    }
    fn sample(&self, rig: &Rig, time: f32) -> Vec<Mat4> {
        let time = if self.duration > 0. {
            time.rem_euclid(self.duration)
        } else {
            0.
        };
        let mut poses: Vec<_> = rig.nodes.iter().map(|n| n.rest).collect();
        for track in &self.tracks {
            let a = track
                .times
                .partition_point(|t| *t <= time)
                .saturating_sub(1);
            let b = (a + 1).min(track.times.len() - 1);
            let t = if a == b || track.step {
                0.
            } else {
                ((time - track.times[a]) / (track.times[b] - track.times[a])).clamp(0., 1.)
            };
            match &track.values {
                Values::Translation(v) => poses[track.node].translation = v[a].lerp(v[b], t),
                Values::Rotation(v) => poses[track.node].rotation = v[a].slerp(v[b], t).normalize(),
                Values::Scale(v) => poses[track.node].scale = v[a].lerp(v[b], t),
            }
        }
        rig.world(&poses)
    }
}
struct AnimationSource {
    rig: Rig,
    run: Clip,
    idle: Clip,
    idle_rig: Rig,
}
impl AnimationSource {
    fn load() -> Result<Self, String> {
        let (doc, buffers, _) =
            gltf::import("assets/generated/running.glb").map_err(|e| e.to_string())?;
        let run = doc
            .animations()
            .find(|a| a.channels().next().is_some())
            .ok_or("Model neobsahuje run animaciu")?;
        let (idle_doc, idle_buffers, _) =
            gltf::import("assets/generated/idle.glb").map_err(|e| e.to_string())?;
        let idle = idle_doc
            .animations()
            .find(|a| a.channels().next().is_some())
            .ok_or("Model neobsahuje idle animaciu")?;
        Ok(Self {
            rig: Rig::read(&doc),
            run: Clip::read(run, &buffers)?,
            idle: Clip::read(idle, &idle_buffers)?,
            idle_rig: Rig::read(&idle_doc),
        })
    }
}
struct Skin {
    joints: Vec<usize>,
    inverse: Vec<Mat4>,
}
struct SkinVertex {
    position: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
}
struct Part {
    mesh: Mesh,
    original: Vec<SkinVertex>,
    skin: Option<usize>,
    node: usize,
}
pub struct Robot {
    rig: Rig,
    skins: Vec<Skin>,
    parts: RefCell<Vec<Part>>,
    normalization: Mat4,
    source: AnimationSource,
    mapping: Vec<Option<usize>>,
    idle_mapping: Vec<Option<usize>>,
}
impl Robot {
    pub fn load(path: &str, color: Color) -> Result<Self, String> {
        let (doc, buffers, _) = gltf::import(path).map_err(|e| e.to_string())?;
        let rig = Rig::read(&doc);
        // Each supplied FBX animation has its own reference skeleton.
        // Retarget world-space deltas independently before blending poses.
        let source = AnimationSource::load()?;
        let names: HashMap<_, _> = source
            .rig
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.name.is_empty())
            .map(|(i, n)| (n.name.as_str(), i))
            .collect();
        let mapping = rig
            .nodes
            .iter()
            .map(|n| names.get(n.name.as_str()).copied())
            .collect();
        let idle_names: HashMap<_, _> = source
            .idle_rig
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.name.is_empty())
            .map(|(i, n)| (n.name.as_str(), i))
            .collect();
        let idle_mapping = rig
            .nodes
            .iter()
            .map(|n| idle_names.get(n.name.as_str()).copied())
            .collect();
        let skins: Vec<_> = doc
            .skins()
            .map(|skin| {
                let joints: Vec<_> = skin.joints().map(|j| j.index()).collect();
                let inverse = skin
                    .reader(|b| Some(&buffers[b.index()].0))
                    .read_inverse_bind_matrices()
                    .map(|m| m.map(|a| Mat4::from_cols_array_2d(&a)).collect())
                    .unwrap_or_else(|| vec![Mat4::IDENTITY; joints.len()]);
                Skin { joints, inverse }
            })
            .collect();
        let mut parts = vec![];
        let scene = doc
            .default_scene()
            .or_else(|| doc.scenes().next())
            .ok_or("Model nema scenu")?;
        fn collect(
            node: gltf::Node<'_>,
            buffers: &[gltf::buffer::Data],
            parts: &mut Vec<Part>,
            color: Color,
        ) -> Result<(), String> {
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    if primitive.mode() != gltf::mesh::Mode::Triangles {
                        continue;
                    }
                    let reader = primitive.reader(|b| Some(&buffers[b.index()].0));
                    let positions: Vec<_> = reader
                        .read_positions()
                        .ok_or("Model nema pozicie")?
                        .map(Vec3::from_array)
                        .collect();
                    let joints: Vec<_> = reader
                        .read_joints(0)
                        .map(|j| j.into_u16().collect())
                        .unwrap_or_else(|| vec![[0; 4]; positions.len()]);
                    let weights: Vec<_> = reader
                        .read_weights(0)
                        .map(|w| w.into_f32().collect())
                        .unwrap_or_else(|| vec![[1., 0., 0., 0.]; positions.len()]);
                    let indices: Vec<_> = reader
                        .read_indices()
                        .map(|i| i.into_u32().collect())
                        .unwrap_or_else(|| (0..positions.len() as u32).collect());
                    if !indices.len().is_multiple_of(3) {
                        return Err("Neplatne trojuholniky modelu".into());
                    }
                    for chunk in indices.chunks(3000) {
                        let mut original = vec![];
                        let mut vertices = vec![];
                        for &index in chunk {
                            let i = index as usize;
                            let position = *positions.get(i).ok_or("Neplatny index modelu")?;
                            original.push(SkinVertex {
                                position,
                                joints: *joints.get(i).ok_or("Chybaju klby")?,
                                weights: *weights.get(i).ok_or("Chybaju vahy")?,
                            });
                            vertices.push(Vertex::new2(position, Vec2::ZERO, color));
                        }
                        parts.push(Part {
                            mesh: Mesh {
                                indices: (0..vertices.len() as u16).collect(),
                                vertices,
                                texture: None,
                            },
                            original,
                            skin: node.skin().map(|s| s.index()),
                            node: node.index(),
                        });
                    }
                }
            }
            for child in node.children() {
                collect(child, buffers, parts, color)?
            }
            Ok(())
        }
        for n in scene.nodes() {
            collect(n, &buffers, &mut parts, color)?
        }
        let mut robot = Self {
            rig,
            skins,
            parts: RefCell::new(parts),
            normalization: Mat4::IDENTITY,
            source,
            mapping,
            idle_mapping,
        };
        robot.deform(&robot.rig.bind);
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for part in robot.parts.borrow().iter() {
            for v in &part.mesh.vertices {
                min = min.min(v.position);
                max = max.max(v.position);
            }
        }
        if !min.is_finite() || max.y - min.y < 0.001 {
            return Err("Model ma neplatne rozmery".into());
        }
        let center = vec3((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5);
        robot.normalization =
            Mat4::from_scale(Vec3::splat(1.8 / (max.y - min.y))) * Mat4::from_translation(-center);
        robot.deform(&robot.rig.bind);
        // Bake simple directional lighting into the bind-pose vertex colors.
        for part in robot.parts.borrow_mut().iter_mut() {
            for triangle in part.mesh.vertices.chunks_mut(3) {
                let normal = (triangle[1].position - triangle[0].position)
                    .cross(triangle[2].position - triangle[0].position)
                    .normalize_or_zero();
                let light = 0.45 + 0.55 * normal.dot(vec3(-0.4, 0.8, 0.5).normalize()).abs();
                for v in triangle {
                    v.color =
                        Color::new(color.r * light, color.g * light, color.b * light, 1.).into();
                }
            }
        }
        Ok(robot)
    }
    fn pose(&self, phase: f32, idle_time: f32, running: f32) -> Vec<Mat4> {
        let idle = self.retarget(
            &self.source.idle_rig,
            &self.source.idle,
            idle_time,
            &self.idle_mapping,
        );
        let run = self.retarget(&self.source.rig, &self.source.run, phase, &self.mapping);
        let blended: Vec<_> = idle
            .iter()
            .zip(run)
            .map(|(a, b)| Pose {
                translation: a.translation.lerp(b.translation, running),
                rotation: a.rotation.slerp(b.rotation, running).normalize(),
                scale: a.scale,
            })
            .collect();
        self.rig.world(&blended)
    }
    fn retarget(&self, rig: &Rig, clip: &Clip, time: f32, mapping: &[Option<usize>]) -> Vec<Pose> {
        let sampled = clip.sample(rig, time);
        let mut poses = vec![
            Pose {
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                scale: Vec3::ONE
            };
            self.rig.nodes.len()
        ];
        let mut world = vec![Mat4::IDENTITY; self.rig.nodes.len()];
        for &i in &self.rig.order {
            let node = &self.rig.nodes[i];
            let mut local = node.rest;
            let parent = node.parent.map_or(Mat4::IDENTITY, |p| world[p]);
            if let Some(source) = mapping[i] {
                let (_, bind_rotation, bind_position) =
                    rig.bind[source].to_scale_rotation_translation();
                let (_, animated, animated_position) =
                    sampled[source].to_scale_rotation_translation();
                let (_, target_rotation, _) = self.rig.bind[i].to_scale_rotation_translation();
                let (_, parent_rotation, _) = parent.to_scale_rotation_translation();
                local.rotation = (parent_rotation.inverse()
                    * animated
                    * bind_rotation.inverse()
                    * target_rotation)
                    .normalize();
                if node.name.ends_with(":Hips") {
                    // Retain game-controlled root X/Z; only transfer vertical bounce.
                    let source_height = bind_position.y.abs().max(0.001);
                    let target_height = self.rig.bind[i].w_axis.y.abs();
                    let dy =
                        (animated_position.y - bind_position.y) * target_height / source_height;
                    local.translation += parent.inverse().transform_vector3(Vec3::Y * dy);
                }
            }
            world[i] = parent * local.matrix();
            poses[i] = local;
        }
        poses
    }
    fn deform(&self, world: &[Mat4]) {
        let palettes: Vec<Vec<Mat4>> = self
            .skins
            .iter()
            .map(|s| {
                s.joints
                    .iter()
                    .zip(&s.inverse)
                    .map(|(&joint, inverse)| world[joint] * *inverse)
                    .collect()
            })
            .collect();
        for part in self.parts.borrow_mut().iter_mut() {
            for (vertex, original) in part.mesh.vertices.iter_mut().zip(&part.original) {
                let position = if let Some(skin) = part.skin {
                    let mut p = Vec3::ZERO;
                    let mut total = 0.;
                    for k in 0..4 {
                        let weight = original.weights[k];
                        if weight > 0. {
                            p += palettes[skin][original.joints[k] as usize]
                                .transform_point3(original.position)
                                * weight;
                            total += weight;
                        }
                    }
                    if total > 0. {
                        p / total
                    } else {
                        world[part.node].transform_point3(original.position)
                    }
                } else {
                    world[part.node].transform_point3(original.position)
                };
                vertex.position = self.normalization.transform_point3(position);
            }
        }
    }
    pub fn draw(&self, pos: Vec3, facing: f32, phase: f32, idle_time: f32, running: f32) {
        self.deform(&self.pose(phase, idle_time, running.clamp(0., 1.)));
        let transform = Mat4::from_translation(pos)
            * Mat4::from_rotation_y(-facing + std::f32::consts::FRAC_PI_2);
        unsafe {
            get_internal_gl().quad_gl.push_model_matrix(transform);
        }
        for part in self.parts.borrow().iter() {
            draw_mesh(&part.mesh);
        }
        unsafe {
            get_internal_gl().quad_gl.pop_model_matrix();
        }
    }
}

/// Visual state is local: measure displacement, including remote snapshots,
/// rather than playing a run cycle merely because a movement target exists.
#[derive(Default)]
pub struct Locomotion {
    previous: Option<Vec3>,
    round: u32,
    pub phase: f32,
    pub blend: f32,
    speed: f32,
}
impl Locomotion {
    pub fn update(&mut self, position: Vec3, round: u32, active: bool, dt: f32) {
        if self.round != round || !active || self.previous.is_none() {
            self.previous = Some(position);
            self.round = round;
            self.blend = 0.;
            self.speed = 0.;
            self.phase = 0.;
            return;
        }
        let distance = self.previous.unwrap().distance(position);
        self.previous = Some(position);
        if dt <= 0. {
            return;
        }
        let speed = (distance / dt).min(9.);
        self.speed += (speed - self.speed) * (1. - (-dt / 0.10).exp());
        let desired = (self.speed / 3.).clamp(0., 1.);
        self.blend += (desired - self.blend) * (1. - (-dt / 0.10).exp());
        if self.speed > 0.05 {
            self.phase += dt * (self.speed / 4.5).clamp(0., 1.8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vertices(robot: &Robot) -> Vec<Vec3> {
        robot
            .parts
            .borrow()
            .iter()
            .flat_map(|p| p.mesh.vertices.iter().map(|v| v.position))
            .collect()
    }
    #[test]
    fn both_bots_are_skinned_and_run_in_place() {
        for path in ["assets/generated/x_bot.glb", "assets/generated/y_bot.glb"] {
            let robot = Robot::load(path, WHITE).unwrap();
            assert!(!robot.skins.is_empty());
            assert!(robot.source.run.duration > 0.);
            robot.deform(&robot.pose(0., 0., 1.));
            let a = vertices(&robot);
            robot.deform(&robot.pose(robot.source.run.duration * 0.4, 0., 1.));
            let b = vertices(&robot);
            assert!(
                a.iter().zip(&b).any(|(a, b)| a.distance(*b) > 0.05),
                "{path}: run must deform vertices"
            );
            assert!(
                b.iter().all(|v| v.is_finite() && v.length() < 4.),
                "{path}: retargeted pose must keep human proportions"
            );
            let pose = robot.pose(robot.source.run.duration * 0.8, 0., 1.);
            let hips = robot
                .rig
                .nodes
                .iter()
                .position(|n| n.name.ends_with(":Hips"))
                .unwrap();
            let offset = pose[hips].w_axis - robot.rig.bind[hips].w_axis;
            assert!(
                offset.x.abs() < 0.001 && offset.z.abs() < 0.001,
                "animation must not move the game root"
            );
        }
    }
    #[test]
    fn locomotion_runs_only_when_moving_and_resets_on_round() {
        let mut state = Locomotion::default();
        let dt = 1. / 60.;
        state.update(Vec3::ZERO, 1, true, dt);
        for i in 1..61 {
            state.update(vec3(i as f32 * 4.5 * dt, 0., 0.), 1, true, dt);
        }
        assert!(state.blend > 0.9 && state.phase > 0.5);
        let pos = vec3(4.5, 0., 0.);
        for _ in 0..120 {
            state.update(pos, 1, true, dt);
        }
        assert!(state.blend < 0.01);
        state.update(vec3(12., 0., 0.), 2, false, dt);
        assert_eq!(state.phase, 0.);
        assert_eq!(state.blend, 0.);
    }
}
