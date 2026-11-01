use serde::{Deserialize, Serialize};
pub const W: f32 = 15.;
pub const H: f32 = 9.;
pub const GOAL: f32 = 2.7;
pub const COUNT: usize = 5;
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
            setup: 15.,
            goal_flash: 0.,
            round: 0,
        };
        g.reset();
        g
    }
    fn reset(&mut self) {
        self.round += 1;
        self.setup = 15.;
        self.ball = Point::default();
        self.velocity = Point::default();
        self.players.clear();
        for team in 0..2 {
            for i in 0..COUNT {
                let sign = if team == 0 { -1. } else { 1. };
                let (x, y) = match i {
                    0 => (12., 0.),
                    1 => (8., -4.),
                    2 => (8., 4.),
                    3 => (3.8, -2.5),
                    _ => (3.8, 2.5),
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
                Point::new(-sign * 12.5, self.ball.y.clamp(-2.3, 2.3))
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
            p.pos.x = p.pos.x.clamp(-14.4, 14.4);
            p.pos.y = p.pos.y.clamp(-8.4, 8.4);
        }
        self.ball.x += self.velocity.x * dt;
        self.ball.y += self.velocity.y * dt;
        for (i, p) in self.players.iter().enumerate() {
            let d = p.pos.distance(self.ball);
            if d < 0.92 {
                let (dx, dy) = if d > 0.001 {
                    ((self.ball.x - p.pos.x) / d, (self.ball.y - p.pos.y) / d)
                } else {
                    (p.facing.cos(), p.facing.sin())
                };
                self.ball = Point::new(p.pos.x + dx * 0.93, p.pos.y + dy * 0.93);
                let dot = motion[i].x * dx + motion[i].y * dy;
                let speed = if dot > 0.1 { 10.5 } else { 2. };
                self.velocity = Point::new(dx * speed, dy * speed);
            }
        }
        if self.ball.x.abs() > W + 0.3 && self.ball.y.abs() < GOAL - 0.3 {
            let team = if self.ball.x > 0. { 0 } else { 1 };
            self.score[team] += 1;
            self.reset();
            self.goal_flash = 2.5;
            return;
        }
        if self.ball.y.abs() > H - 0.3 {
            self.ball.y = self.ball.y.clamp(-H + 0.3, H - 0.3);
            self.velocity.y *= -0.8;
        }
        if self.ball.x.abs() > W - 0.3 && self.ball.y.abs() >= GOAL - 0.3 {
            self.ball.x = self.ball.x.clamp(-W + 0.3, W - 0.3);
            self.velocity.x *= -0.8;
        }
        let drag = (-0.65 * dt).exp();
        self.velocity.x *= drag;
        self.velocity.y *= drag;
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
        let old = g.players[5].pos;
        g.command(
            0,
            Command {
                player: 5,
                target: Point::new(5., 5.),
                round: g.round,
            },
        );
        assert_eq!(g.players[5].pos.x, old.x);
        g.command(
            1,
            Command {
                player: 5,
                target: Point::new(6., 6.),
                round: g.round - 1,
            },
        );
        assert_eq!(g.players[5].pos.x, old.x);
    }
    #[test]
    fn goal_scores_and_resets() {
        let mut g = Game::new();
        g.setup = 0.;
        g.ball = Point::new(15.4, 0.);
        g.tick(1. / 120.);
        assert_eq!(g.score, [1, 0]);
        assert_eq!(g.setup, 15.);
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
        assert_eq!(g.setup, 14.);
    }
    #[test]
    fn both_goals_preserve_existing_score() {
        let mut g = Game::new();
        g.score = [3, 4];
        g.setup = 0.;
        g.ball = Point::new(-15.4, 0.);
        g.tick(1. / 120.);
        assert_eq!(g.score, [3, 5]);
        assert_eq!(g.ball.distance(Point::default()), 0.);
        assert_eq!(g.players.len(), COUNT * 2);
    }
    #[test]
    fn movement_command_causes_contact_kick() {
        let mut g = Game::new();
        g.setup = 0.;
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
}
