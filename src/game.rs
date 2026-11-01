use serde::{Deserialize, Serialize};
pub const W: f32 = 15.;
pub const H: f32 = 9.;
pub const GOAL: f32 = 4.2;
pub const COUNT: usize = 4;
pub const BALL_RADIUS: f32 = 0.45;
pub const SETUP_SECONDS: f32 = 5.;
const CONTACT_RADIUS: f32 = 0.6 + BALL_RADIUS;
pub const CORNER_RADIUS: f32 = 2.5;
const MAX_BALL_SPEED: f32 = 18.;

/// Project a circle inside the rounded pitch. Goals remain open for the ball.
pub fn constrain_field(p: &mut Point, radius: f32, goals: bool) -> Option<Point> {
    let old = *p;
    p.y = p.y.clamp(-H + radius, H - radius);
    if !goals || p.y.abs() >= GOAL - radius {
        p.x = p.x.clamp(-W + radius, W - radius);
    }
    let cx = W - CORNER_RADIUS;
    let cy = H - CORNER_RADIUS;
    if p.x.abs() > cx && p.y.abs() > cy {
        let dx = p.x.abs() - cx;
        let dy = p.y.abs() - cy;
        let d = dx.hypot(dy);
        let limit = CORNER_RADIUS - radius;
        if d > limit {
            p.x = p.x.signum() * (cx + dx / d * limit);
            p.y = p.y.signum() * (cy + dy / d * limit);
        }
    }
    let d = old.distance(*p);
    if d > 0.000001 {
        Some(Point::new((p.x - old.x) / d, (p.y - old.y) / d))
    } else {
        None
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
    pub fn distance(self, b: Self) -> f32 {
        ((self.x - b.x).powi(2) + (self.y - b.y).powi(2)).sqrt()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub pos: Point,
    pub target: Point,
    pub facing: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Game {
    pub players: Vec<Player>,
    pub ball: Point,
    pub velocity: Point,
    pub score: [u32; 2],
    pub setup: f32,
    pub goal_flash: f32,
    pub round: u32,
    pub ball_pressure: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Command {
    pub player: usize,
    pub target: Point,
    pub round: u32,
}
impl Game {
    pub fn new() -> Self {
        let mut g = Self {
            players: vec![],
            ball: Point::default(),
            velocity: Point::default(),
            score: [0, 0],
            setup: SETUP_SECONDS,
            goal_flash: 0.,
            round: 0,
            ball_pressure: 0.,
        };
        g.reset();
        g
    }
    fn reset(&mut self) {
        self.round += 1;
        self.setup = SETUP_SECONDS;
        self.ball = Point::default();
        self.ball_pressure = 0.;
        self.velocity = Point::default();
        self.players.clear();
        for team in 0..2 {
            for i in 0..COUNT {
                let sign = if team == 0 { -1. } else { 1. };
                let (x, y) = match i {
                    0 => (12., 0.),
                    1 => (8., -4.),
                    2 => (8., 4.),
                    _ => (3.8, 0.),
                };
                let pos = Point::new(sign * x, y);
                self.players.push(Player {
                    pos,
                    target: pos,
                    facing: if team == 0 { 0. } else { std::f32::consts::PI },
                });
            }
        }
    }
    pub fn command(&mut self, team: usize, c: Command) {
        if team > 1
            || c.player >= self.players.len()
            || c.player / COUNT != team
            || !c.target.x.is_finite()
            || !c.target.y.is_finite()
            || c.round != self.round
        {
            return;
        }
        let mut t = Point::new(
            c.target.x.clamp(-W + 0.65, W - 0.65),
            c.target.y.clamp(-H + 0.65, H - 0.65),
        );
        constrain_field(&mut t, 0.65, false);
        if self.setup > 0. {
            t.x = if team == 0 {
                t.x.min(-0.7)
            } else {
                t.x.max(0.7)
            };
            if self
                .players
                .iter()
                .enumerate()
                .any(|(i, p)| i != c.player && p.pos.distance(t) < 1.15)
            {
                return;
            }
            self.players[c.player].pos = t;
        }
        self.players[c.player].target = t;
    }
    pub fn ai(&mut self, team: usize) {
        if self.setup > 0. {
            return;
        }
        let start = team * COUNT;
        let nearest = (start + 1..start + COUNT)
            .min_by(|a, b| {
                self.players[*a]
                    .pos
                    .distance(self.ball)
                    .total_cmp(&self.players[*b].pos.distance(self.ball))
            })
            .unwrap();
        let sign = if team == 0 { 1. } else { -1. };
        for i in start..start + COUNT {
            let target = if i == nearest {
                let p = self.players[i].pos;
                let behind = Point::new(self.ball.x - sign * 1.3, self.ball.y);
                if (self.ball.x - p.x) * sign > 0.4 && (self.ball.y - p.y).abs() < 0.9 {
                    Point::new(self.ball.x + sign * 2., self.ball.y * 0.85)
                } else if (p.x - self.ball.x) * sign > 0. && (p.y - self.ball.y).abs() < 1.4 {
                    Point::new(
                        behind.x,
                        self.ball.y + if p.y > self.ball.y { 1.8 } else { -1.8 },
                    )
                } else {
                    behind
                }
            } else if i == start {
                Point::new(-sign * 12.5, self.ball.y.clamp(-GOAL + 0.6, GOAL - 0.6))
            } else {
                Point::new(
                    (self.ball.x - sign * 3. - sign * (i % 2) as f32 * 3.).clamp(-12., 12.),
                    if i % 2 == 0 { 4. } else { -4. },
                )
            };
            self.command(
                team,
                Command {
                    player: i,
                    target,
                    round: self.round,
                },
            );
        }
    }
    pub fn tick(&mut self, dt: f32) {
        self.goal_flash = (self.goal_flash - dt).max(0.);
        if self.setup > 0. {
            self.setup = (self.setup - dt).max(0.);
            return;
        }
        let mut motion = vec![Point::default(); self.players.len()];
        for (i, p) in self.players.iter_mut().enumerate() {
            let d = p.pos.distance(p.target);
            if d > 0.04 {
                let speed = (4.5 * dt).min(d);
                let dx = (p.target.x - p.pos.x) / d;
                let dy = (p.target.y - p.pos.y) / d;
                p.facing = dy.atan2(dx);
                p.pos.x += dx * speed;
                p.pos.y += dy * speed;
                motion[i] = Point::new(dx * 4.5, dy * 4.5);
            }
        }
        for a in 0..self.players.len() {
            for b in a + 1..self.players.len() {
                let pa = self.players[a].pos;
                let pb = self.players[b].pos;
                let d = pa.distance(pb);
                if d < 1.05 {
                    let (dx, dy) = if d > 0.001 {
                        ((pb.x - pa.x) / d, (pb.y - pa.y) / d)
                    } else {
                        (1., 0.)
                    };
                    let push = (1.05 - d) * 0.5;
                    self.players[a].pos.x -= dx * push;
                    self.players[a].pos.y -= dy * push;
                    self.players[b].pos.x += dx * push;
                    self.players[b].pos.y += dy * push;
                }
            }
        }
        for p in &mut self.players {
            constrain_field(&mut p.pos, 0.6, false);
        }
        self.ball.x += self.velocity.x * dt;
        self.ball.y += self.velocity.y * dt;
        self.solve_ball_contacts(&motion, dt);
        if self.ball.x.abs() > W + BALL_RADIUS && self.ball.y.abs() < GOAL - BALL_RADIUS {
            let team = if self.ball.x > 0. { 0 } else { 1 };
            self.score[team] += 1;
            self.reset();
            self.goal_flash = 2.5;
            return;
        }
        if let Some(n) = constrain_field(&mut self.ball, BALL_RADIUS, true) {
            let incoming = self.velocity.x * n.x + self.velocity.y * n.y;
            if incoming < 0. {
                let restitution = 0.8 + self.ball_pressure * 0.6;
                self.velocity.x -= (1. + restitution) * incoming * n.x;
                self.velocity.y -= (1. + restitution) * incoming * n.y;
            }
        }
        let speed = self.velocity.x.hypot(self.velocity.y);
        if speed > MAX_BALL_SPEED {
            self.velocity.x *= MAX_BALL_SPEED / speed;
            self.velocity.y *= MAX_BALL_SPEED / speed;
        }
        let drag = (-0.65 * dt).exp();
        self.velocity.x *= drag;
        self.velocity.y *= drag;
    }
    fn solve_ball_contacts(&mut self, motion: &[Point], dt: f32) {
        let mut contacts = vec![];
        let mut normals = vec![];
        let mut load = 0.;
        let mut correction = Point::default();
        for (i, p) in self.players.iter().enumerate() {
            let d = p.pos.distance(self.ball);
            if d < CONTACT_RADIUS + 0.06 {
                let n = if d > 0.001 {
                    Point::new((self.ball.x - p.pos.x) / d, (self.ball.y - p.pos.y) / d)
                } else {
                    Point::new(p.facing.cos(), p.facing.sin())
                };
                let drive = (motion[i].x * n.x + motion[i].y * n.y).max(0.);
                load += drive + (CONTACT_RADIUS - d).max(0.) * 20.;
                normals.push(n);
                contacts.push((i, n, drive));
                let overlap = (CONTACT_RADIUS - d).max(0.);
                correction.x += n.x * overlap;
                correction.y += n.y * overlap;
            }
        }
        let mut probe = self.ball;
        if let Some(n) = constrain_field(&mut probe, BALL_RADIUS + 0.06, true) {
            normals.push(n);
        }
        let compressed = normals.iter().enumerate().any(|(i, a)| {
            normals[i + 1..]
                .iter()
                .any(|b| a.x * b.x + a.y * b.y < -0.25)
        });
        if compressed && load > 0.1 {
            self.ball_pressure = (self.ball_pressure + dt * (0.7 + load * 0.18)).min(1.);
        } else {
            self.ball_pressure *= (-dt * 6.).exp();
        }
        if !contacts.is_empty() {
            let count = contacts.len() as f32;
            // Simultaneous impulses: no player overwrites another player's kick.
            let mut impulse = Point::default();
            for &(_, n, drive) in &contacts {
                let normal_speed = self.velocity.x * n.x + self.velocity.y * n.y;
                let rebound = (-normal_speed).max(0.) * (0.75 + self.ball_pressure);
                let kick: f32 = if drive > 0.1 { 10.5 } else { 2. };
                let change = (kick.max(rebound) - normal_speed).max(0.) / count;
                impulse.x += n.x * change;
                impulse.y += n.y * change;
            }
            self.velocity.x += impulse.x;
            self.velocity.y += impulse.y;
            // Only resolve local overlap; never relocate the ball to a free spot.
            self.ball.x += correction.x / count;
            self.ball.y += correction.y / count;
        }
        if compressed && self.ball_pressure > 0.01 {
            // A compressed ball squirts through the widest nearby gap. Looking
            // a short distance ahead breaks perfectly symmetric opposed forces.
            let mut best = Point::default();
            let mut best_score = f32::NEG_INFINITY;
            for i in 0..48 {
                let angle = i as f32 * std::f32::consts::TAU / 48.;
                let n = Point::new(angle.cos(), angle.sin());
                let probe = Point::new(self.ball.x + n.x * 0.85, self.ball.y + n.y * 0.85);
                let mut inside = probe;
                constrain_field(&mut inside, BALL_RADIUS, true);
                let clearance = self
                    .players
                    .iter()
                    .map(|p| p.pos.distance(probe) - CONTACT_RADIUS)
                    .fold(2_f32, f32::min);
                let toward_center = -(n.x * self.ball.x / W + n.y * self.ball.y / H);
                let score = clearance - probe.distance(inside) * 8. + toward_center * 0.08;
                if score > best_score {
                    best_score = score;
                    best = n;
                }
            }
            // Stored compression supplies an outgoing collision impulse, not
            // just acceleration that the next contact could cancel every step.
            let outgoing = self.velocity.x * best.x + self.velocity.y * best.y;
            let impulse = (16. * self.ball_pressure * self.ball_pressure - outgoing).max(0.);
            self.velocity.x += best.x * impulse;
            self.velocity.y += best.y * impulse;
            // Players are not infinitely strong static walls: compression
            // pushes their bodies back, opening a real gap for the ball.
            let recoil = dt * 10. * self.ball_pressure * self.ball_pressure;
            for &(i, n, _) in &contacts {
                self.players[i].pos.x -= n.x * recoil;
                self.players[i].pos.y -= n.y * recoil;
                constrain_field(&mut self.players[i].pos, 0.6, false);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placement_stays_on_own_half() {
        let mut g = Game::new();
        g.command(
            0,
            Command {
                player: 0,
                target: Point::new(10., 8.),
                round: g.round,
            },
        );
        assert_eq!(g.players[0].pos.x, -0.7);
    }
    #[test]
    fn rejects_opponent_and_stale_commands() {
        let mut g = Game::new();
        let old = g.players[COUNT].pos;
        g.command(
            0,
            Command {
                player: crate::game::COUNT,
                target: Point::new(5., 5.),
                round: g.round,
            },
        );
        assert_eq!(g.players[COUNT].pos.x, old.x);
        g.command(
            1,
            Command {
                player: crate::game::COUNT,
                target: Point::new(6., 6.),
                round: g.round - 1,
            },
        );
        assert_eq!(g.players[COUNT].pos.x, old.x);
    }
    #[test]
    fn goal_scores_and_resets() {
        let mut g = Game::new();
        g.setup = 0.;
        g.ball = Point::new(W + BALL_RADIUS + 0.1, 0.);
        g.tick(1. / 120.);
        assert_eq!(g.score, [1, 0]);
        assert_eq!(g.setup, SETUP_SECONDS);
        assert_eq!(g.round, 2);
    }
    #[test]
    fn outside_goal_bounces() {
        let mut g = Game::new();
        g.setup = 0.;
        g.ball = Point::new(15., 5.);
        g.velocity.x = 2.;
        g.tick(0.01);
        assert_eq!(g.score, [0, 0]);
        assert!(g.velocity.x < 0.);
    }
    #[test]
    fn setup_freezes_ball() {
        let mut g = Game::new();
        g.velocity.x = 5.;
        g.tick(1.);
        assert_eq!(g.ball.x, 0.);
        assert_eq!(g.setup, SETUP_SECONDS - 1.);
    }
    #[test]
    fn both_goals_preserve_existing_score() {
        let mut g = Game::new();
        g.score = [3, 4];
        g.setup = 0.;
        g.ball = Point::new(-W - BALL_RADIUS - 0.1, 0.);
        g.tick(1. / 120.);
        assert_eq!(g.score, [3, 5]);
        assert_eq!(g.ball.distance(Point::default()), 0.);
        assert_eq!(g.players.len(), COUNT * 2);
    }
    #[test]
    fn movement_command_causes_contact_kick() {
        let mut g = Game::new();
        g.setup = 0.;
        for (i, p) in g.players.iter_mut().enumerate().skip(1) {
            p.pos = Point::new(-12. + i as f32 * 3., 6.);
            p.target = p.pos;
        }
        g.players[0].pos = Point::new(-1.5, 0.);
        g.command(
            0,
            Command {
                player: 0,
                target: Point::new(3., 0.),
                round: g.round,
            },
        );
        for _ in 0..60 {
            g.tick(1. / 120.);
        }
        assert!(g.ball.x > 1.);
        assert!(g.velocity.x > 0.);
        assert_eq!(g.score, [0, 0]);
    }
    #[test]
    fn teams_have_four_players_and_three_defenders_leave_a_scoring_lane() {
        let mut g = Game::new();
        assert_eq!(g.players.len(), 8);
        assert_eq!(COUNT, 4);
        g.setup = 0.;
        for (i, y) in [-2.1, 0., 2.1].iter().enumerate() {
            let p = &mut g.players[COUNT + i];
            p.pos = Point::new(W - 0.6, *y);
            p.target = p.pos;
        }
        g.ball = Point::new(12.5, 3.6);
        g.velocity = Point::new(10., 0.);
        for _ in 0..120 {
            g.tick(1. / 120.);
            if g.score[0] > 0 {
                break;
            }
        }
        assert_eq!(
            g.score,
            [1, 0],
            "a ball-sized lane must remain beside three defenders"
        );
    }
    #[test]
    fn rejects_invalid_and_overlapping_placement() {
        let mut g = Game::new();
        let old = g.players[0].pos;
        for (player, target) in [
            (0, Point::new(f32::NAN, 0.)),
            (usize::MAX, Point::default()),
            (0, g.players[1].pos),
        ] {
            g.command(
                0,
                Command {
                    player,
                    target,
                    round: g.round,
                },
            );
        }
        assert_eq!(g.players[0].pos.distance(old), 0.);
    }
    #[test]
    fn compressed_ball_escapes_without_position_jump() {
        for sign in [-1., 1.] {
            let mut g = Game::new();
            g.setup = 0.;
            g.ball = Point::new(0., sign * (H - BALL_RADIUS));
            g.players[0].pos = Point::new(-0.6, sign * 8.4);
            g.players[COUNT].pos = Point::new(0.6, sign * 8.4);
            g.players[0].target = Point::new(1., sign * 8.35);
            g.players[COUNT].target = Point::new(-1., sign * 8.35);
            let mut escaped = false;
            let mut pressure = 0_f32;
            for _ in 0..720 {
                let old = g.ball;
                g.tick(1. / 120.);
                pressure = pressure.max(g.ball_pressure);
                assert!(old.distance(g.ball) < 0.4, "no teleport");
                if g.ball.y.abs() < H - 2. {
                    escaped = true;
                    break;
                }
            }
            assert!(pressure > 0.);
            assert!(
                escaped,
                "ball={:?} velocity={:?} pressure={}",
                g.ball, g.velocity, g.ball_pressure
            );
            assert_eq!(g.score, [0, 0]);
        }
    }
    #[test]
    fn unattended_ball_does_not_gain_energy() {
        let mut g = Game::new();
        g.setup = 0.;
        g.ball = Point::new(0., H - BALL_RADIUS);
        let old = g.ball;
        for _ in 0..600 {
            g.tick(1. / 120.);
        }
        assert_eq!(g.ball.distance(old), 0.);
        assert_eq!(g.ball_pressure, 0.);
    }
    #[test]
    fn all_corners_deflect_diagonal_shots_inward() {
        for sx in [-1., 1.] {
            for sy in [-1., 1.] {
                let mut g = Game::new();
                g.setup = 0.;
                g.ball = Point::new(sx * 13.5, sy * 7.5);
                g.velocity = Point::new(sx * 10., sy * 10.);
                for _ in 0..30 {
                    g.tick(1. / 120.);
                }
                assert!(g.velocity.x * sx < 0. && g.velocity.y * sy < 0.);
                let mut inside = g.ball;
                assert!(constrain_field(&mut inside, BALL_RADIUS, true).is_none());
            }
        }
    }
    #[test]
    fn corner_placement_is_inside_curved_boundary() {
        let mut g = Game::new();
        g.command(
            0,
            Command {
                player: 0,
                target: Point::new(-W, -H),
                round: g.round,
            },
        );
        let mut pos = g.players[0].pos;
        assert!(constrain_field(&mut pos, 0.6, false).is_none());
        assert!(g.players[0].pos.x > -14.35 && g.players[0].pos.y > -8.35);
    }
    #[test]
    fn pressure_increases_rebound_and_dissipates_when_free() {
        let mut low = Game::new();
        low.ball = Point::default();
        low.players[0].pos = Point::new(-0.9, 0.);
        low.velocity = Point::new(-8., 0.);
        let mut high = low.clone();
        high.ball_pressure = 1.;
        let motion = vec![Point::default(); COUNT * 2];
        low.solve_ball_contacts(&motion, 1. / 120.);
        high.solve_ball_contacts(&motion, 1. / 120.);
        assert!(high.velocity.x > low.velocity.x);
        high.ball = Point::new(0., 7.);
        let before = high.ball_pressure;
        high.solve_ball_contacts(&motion, 0.1);
        assert!(high.ball_pressure < before);
    }
    #[test]
    fn corner_squeezes_escape_with_bounded_speed() {
        for sx in [-1., 1.] {
            for sy in [-1., 1.] {
                let mut g = Game::new();
                g.setup = 0.;
                let component = (CORNER_RADIUS - BALL_RADIUS) / 2_f32.sqrt();
                g.ball = Point::new(
                    sx * (W - CORNER_RADIUS + component),
                    sy * (H - CORNER_RADIUS + component),
                );
                let start = g.ball;
                g.players[0].pos = Point::new(start.x - sx * 0.85, start.y);
                g.players[COUNT].pos = Point::new(start.x, start.y - sy * 0.85);
                for i in [0, COUNT] {
                    constrain_field(&mut g.players[i].pos, 0.6, false);
                    g.players[i].target = start;
                }
                let mut escaped = false;
                for _ in 0..720 {
                    let old = g.ball;
                    g.tick(1. / 120.);
                    assert!(g.velocity.x.hypot(g.velocity.y) <= MAX_BALL_SPEED + 0.001);
                    assert!(old.distance(g.ball) < 0.4);
                    if start.distance(g.ball) > 2. {
                        escaped = true;
                        break;
                    }
                }
                assert!(escaped, "corner {sx}/{sy}: {:?}", g.ball);
            }
        }
    }
    #[test]
    fn contact_order_does_not_favor_the_last_player() {
        let mut a = Game::new();
        a.ball = Point::default();
        a.players[0].pos = Point::new(-0.7, 0.);
        a.players[COUNT].pos = Point::new(0.7, 0.);
        let mut b = a.clone();
        b.players.swap(0, COUNT);
        let motion = vec![Point::default(); COUNT * 2];
        a.solve_ball_contacts(&motion, 1. / 120.);
        b.solve_ball_contacts(&motion, 1. / 120.);
        assert!(a.ball.distance(b.ball) < 0.0001);
        assert!(a.velocity.distance(b.velocity) < 0.0001);
    }
}
