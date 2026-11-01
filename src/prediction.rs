use crate::game::{constrain_field, Command, Game, COUNT};
use std::collections::VecDeque;

struct Pending {
    sequence: u64,
    command: Command,
    sent: f64,
}
pub struct Prediction {
    team: usize,
    state: Option<Game>,
    authoritative: Option<Game>,
    pending: VecDeque<Pending>,
    sequence: u64,
    age: f32,
    rtt: f32,
}
impl Default for Prediction {
    fn default() -> Self {
        Self {
            team: 1,
            state: None,
            authoritative: None,
            pending: VecDeque::new(),
            sequence: 0,
            age: 0.,
            rtt: 0.04,
        }
    }
}
impl Prediction {
    pub fn ready(&self) -> bool {
        self.state.is_some()
    }
    pub fn receive(&mut self, server: Game, ack: u64, now: f64) {
        if server.players.len() != COUNT * 2 {
            return;
        }
        if let Some(p) = self.pending.iter().find(|p| p.sequence == ack) {
            self.rtt = self.rtt * 0.75 + ((now - p.sent) as f32).clamp(0., 0.3) * 0.25;
        }
        self.pending
            .retain(|p| p.sequence > ack && p.command.round == server.round);
        let mut state = server.clone();
        if let Some(old) = &self.state {
            let same_phase = old.round == server.round
                && (old.setup > 0.) == (server.setup > 0.)
                && (old.goal_flash > 0.) == (server.goal_flash > 0.);
            if same_phase {
                for i in self.team * COUNT..(self.team + 1) * COUNT {
                    if server.setup <= 0. || self.pending.iter().any(|p| p.command.player == i) {
                        state.players[i].pos = old.players[i].pos;
                        state.players[i].facing = old.players[i].facing;
                    }
                }
            }
        }
        for p in &self.pending {
            state.command(self.team, p.command.clone());
        }
        self.authoritative = Some(server);
        self.state = Some(state);
        self.age = 0.;
    }
    pub fn command(&mut self, command: Command, now: f64) -> Option<u64> {
        let state = self.state.as_mut()?;
        if command.player / COUNT != self.team
            || command.round != state.round
            || state.goal_flash > 0.
        {
            return None;
        }
        state.command(self.team, command.clone());
        self.sequence += 1;
        self.pending.push_back(Pending {
            sequence: self.sequence,
            command,
            sent: now,
        });
        while self.pending.len() > 240 {
            self.pending.pop_front();
        }
        Some(self.sequence)
    }
    pub fn update(&mut self, dt: f32) {
        let dt = dt.clamp(0., 0.1);
        self.age = (self.age + dt).min(0.15);
        let Some(state) = &mut self.state else { return };
        step(state, dt);
        if state.setup > 0. || state.goal_flash > 0. {
            return;
        }
        let Some(server) = &self.authoritative else {
            return;
        };
        let mut expected = server.clone();
        step(&mut expected, (self.age + self.rtt * 0.5).min(0.15));
        let blend = 1. - (-12. * dt).exp();
        for i in self.team * COUNT..(self.team + 1) * COUNT {
            // A snapshot preceding our command must not pull us back while it
            // travels to the host. Reconcile once that command is acknowledged.
            if self.pending.iter().any(|p| p.command.player == i) {
                continue;
            }
            let a = state.players[i].pos;
            let b = expected.players[i].pos;
            let t = if a.distance(b) > 2. { 1. } else { blend };
            state.players[i].pos.x += (b.x - a.x) * t;
            state.players[i].pos.y += (b.y - a.y) * t;
            constrain_field(&mut state.players[i].pos, 0.6, false);
        }
    }
    pub fn apply(&self, display: &mut Game) {
        let Some(state) = &self.state else { return };
        if display.round != state.round
            || (display.setup > 0.) != (state.setup > 0.)
            || (display.goal_flash > 0.) != (state.goal_flash > 0.)
        {
            *display = state.clone();
        }
        for i in self.team * COUNT..(self.team + 1) * COUNT {
            display.players[i] = state.players[i].clone();
        }
        display.round = state.round;
        display.setup = state.setup;
        display.goal_flash = state.goal_flash;
        display.score = state.score;
        display.scoring_team = state.scoring_team;
    }
}
fn step(game: &mut Game, mut dt: f32) {
    while dt > 0.000001 {
        let step = dt.min(1. / 120.);
        game.advance_players(step);
        dt -= step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::Point;
    fn game() -> Game {
        let mut g = Game::new();
        g.setup = 0.;
        g
    }
    #[test]
    fn own_player_moves_on_first_frame_without_a_server_reply() {
        let g = game();
        let mut p = Prediction::default();
        p.receive(g.clone(), 0, 0.);
        let start = g.players[COUNT].pos;
        let command = Command {
            player: COUNT,
            target: Point::new(start.x - 3., start.y),
            round: g.round,
        };
        let sequence = p.command(command, 0.01).unwrap();
        p.update(1. / 60.);
        let mut display = g.clone();
        p.apply(&mut display);
        assert!((display.players[COUNT].pos.x - start.x + 4.5 / 60.).abs() < 0.001);
        // The old target in an unacknowledged snapshot must not cancel input.
        p.receive(g.clone(), 0, 0.03);
        p.update(1. / 60.);
        p.apply(&mut display);
        assert!(display.players[COUNT].pos.x < start.x - 0.1);
        assert_eq!(sequence, 1);
        assert_eq!(display.players[0].pos.distance(g.players[0].pos), 0.);
    }
    #[test]
    fn placement_is_immediate_and_old_snapshots_cannot_undo_it() {
        let g = Game::new();
        let mut p = Prediction::default();
        p.receive(g.clone(), 0, 0.);
        let target = Point::new(6., 1.);
        let seq = p
            .command(
                Command {
                    player: COUNT,
                    target,
                    round: g.round,
                },
                0.,
            )
            .unwrap();
        p.receive(g.clone(), 0, 0.02);
        let mut display = g.clone();
        p.apply(&mut display);
        assert_eq!(display.players[COUNT].pos.distance(target), 0.);
        // A rejected placement is corrected to the authoritative location.
        p.receive(g.clone(), seq, 0.04);
        p.apply(&mut display);
        assert_eq!(
            display.players[COUNT].pos.distance(g.players[COUNT].pos),
            0.
        );
    }
    #[test]
    fn round_changes_clear_pending_movement_and_reset_positions() {
        let g = game();
        let mut p = Prediction::default();
        p.receive(g.clone(), 0, 0.);
        p.command(
            Command {
                player: COUNT,
                target: Point::default(),
                round: g.round,
            },
            0.,
        );
        p.update(0.1);
        let mut next = Game::new();
        next.round = g.round + 1;
        p.receive(next.clone(), 0, 0.1);
        let mut display = g;
        p.apply(&mut display);
        assert_eq!(display.round, next.round);
        assert_eq!(
            display.players[COUNT].pos.distance(next.players[COUNT].pos),
            0.
        );
        assert!(p.pending.is_empty());
    }
    #[test]
    fn acknowledged_movement_reconciles_gradually() {
        let g = game();
        let mut p = Prediction::default();
        p.receive(g.clone(), 0, 0.);
        let target = Point::new(8., 0.);
        let seq = p
            .command(
                Command {
                    player: COUNT,
                    target,
                    round: g.round,
                },
                0.,
            )
            .unwrap();
        p.update(0.1);
        let mut server = g;
        server.command(
            1,
            Command {
                player: COUNT,
                target,
                round: server.round,
            },
        );
        server.advance_players(0.01);
        p.receive(server.clone(), seq, 0.1);
        let before = p.state.as_ref().unwrap().players[COUNT].pos;
        p.update(1. / 60.);
        let after = p.state.as_ref().unwrap().players[COUNT].pos;
        assert!(before.distance(after) < 0.15);
        assert!(p.pending.is_empty());
    }
}
