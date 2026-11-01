use crate::game::{Game, Point, COUNT};
use std::collections::VecDeque;

const DELAY: f64 = 0.10;
struct Sample {
    time: f64,
    state: Game,
}
#[derive(Default)]
pub struct Interpolation {
    samples: VecDeque<Sample>,
    clock: Option<f64>,
}
impl Interpolation {
    pub fn push(&mut self, time: f64, state: Game) {
        if !time.is_finite()
            || state.players.len() != COUNT * 2
            || self.samples.back().is_some_and(|s| time <= s.time)
        {
            return;
        }
        if self.clock.is_none() {
            self.clock = Some(time - DELAY);
        }
        self.samples.push_back(Sample { time, state });
        while self.samples.len() > 96 {
            self.samples.pop_front();
        }
    }
    pub fn render(&mut self, dt: f64) -> Option<Game> {
        let latest = self.samples.back()?.time;
        let clock = self.clock.as_mut()?;
        *clock = (*clock + dt.max(0.)).min(latest);
        if latest - *clock > DELAY + 0.25 {
            *clock = latest - DELAY;
        }
        while self.samples.len() > 2 && self.samples[1].time <= *clock {
            self.samples.pop_front();
        }
        let a = &self.samples[0];
        if self.samples.len() == 1 || *clock <= a.time {
            return Some(a.state.clone());
        }
        let b = &self.samples[1];
        if *clock >= b.time {
            return Some(b.state.clone());
        }
        // Round changes are teleports, not movement across the whole pitch.
        if a.state.round != b.state.round {
            return Some(a.state.clone());
        }
        let t = ((*clock - a.time) / (b.time - a.time)).clamp(0., 1.) as f32;
        let mut result = a.state.clone();
        for ((p, a), b) in result
            .players
            .iter_mut()
            .zip(&a.state.players)
            .zip(&b.state.players)
        {
            p.pos = lerp(a.pos, b.pos, t);
            let delta = (b.facing - a.facing + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            p.facing = a.facing + delta * t;
        }
        result.ball = lerp(a.state.ball, b.state.ball, t);
        result.velocity = lerp(a.state.velocity, b.state.velocity, t);
        if a.state.setup > 0. && b.state.setup > 0. {
            result.setup = a.state.setup + (b.state.setup - a.state.setup) * t;
        }
        if a.state.goal_flash > 0. && b.state.goal_flash > 0. {
            result.goal_flash = a.state.goal_flash + (b.state.goal_flash - a.state.goal_flash) * t;
        }
        Some(result)
    }
}
fn lerp(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn moving(time: f32) -> Game {
        let mut g = Game::new();
        g.setup = 0.;
        g.players[0].pos.x = time * 4.5;
        g.ball.x = time * 10.;
        g
    }
    #[test]
    fn intermediate_frames_move_between_thirty_hz_updates() {
        let mut buffer = Interpolation::default();
        for i in 0..7 {
            let time = i as f64 / 30.;
            buffer.push(time, moving(time as f32));
        }
        let a = buffer.render(0.2).unwrap();
        let b = buffer.render(1. / 60.).unwrap();
        assert!((a.players[0].pos.x - 0.45).abs() < 0.001);
        assert!((b.players[0].pos.x - a.players[0].pos.x - 4.5 / 60.).abs() < 0.001);
        assert!((b.ball.x - a.ball.x - 10. / 60.).abs() < 0.001);
    }
    #[test]
    fn batched_jittered_delivery_keeps_smooth_playback() {
        let mut buffer = Interpolation::default();
        let mut next = 0;
        let mut previous: Option<f32> = None;
        for frame in 0..120 {
            // Deliver pairs of 30 Hz snapshots together every four 60 Hz frames.
            if frame % 4 == 0 {
                for _ in 0..2 {
                    let t = next as f64 / 30.;
                    buffer.push(t, moving(t as f32));
                    next += 1;
                }
            }
            let state = buffer.render(1. / 60.).unwrap();
            if frame > 12 {
                if let Some(x) = previous {
                    assert!((state.players[0].pos.x - x - 4.5 / 60.).abs() < 0.001);
                }
            }
            previous = Some(state.players[0].pos.x);
        }
    }
    #[test]
    fn round_reset_snaps_and_missing_packets_do_not_extrapolate() {
        let mut buffer = Interpolation::default();
        let mut a = moving(0.);
        a.ball.x = 16.;
        let mut b = moving(0.);
        b.round += 1;
        buffer.push(0., a);
        buffer.push(0.2, b);
        assert_eq!(buffer.render(0.2).unwrap().ball.x, 16.);
        let after = buffer.render(0.11).unwrap();
        assert_eq!(after.ball.x, 0.);
        assert_eq!(after.round, 2);
        assert_eq!(buffer.render(2.).unwrap().ball.x, 0.);
    }
    #[test]
    fn facing_uses_shortest_arc() {
        let mut buffer = Interpolation::default();
        let mut a = moving(0.);
        a.players[0].facing = 350_f32.to_radians();
        let mut b = moving(0.2);
        b.players[0].facing = 10_f32.to_radians();
        buffer.push(0., a);
        buffer.push(0.2, b);
        let angle = buffer.render(0.2).unwrap().players[0].facing;
        assert!(angle.sin().abs() < 0.001 && angle.cos() > 0.99);
    }
}
