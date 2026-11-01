use crate::game::{Game, Player, Point, COUNT};

pub const MAX_PACKET: usize = 1200;
pub type Token = [u8; 16];
pub enum Payload {
    Register,
    Ready,
    Snapshot { time: f64, state: Game, ack: u64 },
}
pub struct Packet {
    pub token: Token,
    pub sequence: u64,
    pub payload: Payload,
}

fn header(token: Token, sequence: u64, kind: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(300);
    out.extend_from_slice(b"FTH1");
    out.push(kind);
    out.extend_from_slice(&token);
    out.extend_from_slice(&sequence.to_le_bytes());
    out
}
pub fn registration(token: Token) -> Vec<u8> {
    header(token, 0, 0)
}
pub fn ready(token: Token) -> Vec<u8> {
    header(token, 0, 1)
}
pub fn snapshot(token: Token, sequence: u64, time: f64, state: &Game, ack: u64) -> Option<Vec<u8>> {
    if state.players.len() != COUNT * 2 || !time.is_finite() {
        return None;
    }
    let mut out = header(token, sequence, 2);
    out.extend_from_slice(&time.to_le_bytes());
    out.extend_from_slice(&ack.to_le_bytes());
    for score in state.score {
        out.extend_from_slice(&score.to_le_bytes());
    }
    for value in [state.setup, state.goal_flash, state.ball_pressure] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(state.scoring_team.map_or(255, |t| t as u8));
    out.extend_from_slice(&state.round.to_le_bytes());
    point(&mut out, state.ball);
    point(&mut out, state.velocity);
    out.push(state.players.len() as u8);
    for player in &state.players {
        point(&mut out, player.pos);
        point(&mut out, player.target);
        out.extend_from_slice(&player.facing.to_le_bytes());
    }
    (out.len() <= MAX_PACKET).then_some(out)
}
fn point(out: &mut Vec<u8>, p: Point) {
    out.extend_from_slice(&p.x.to_le_bytes());
    out.extend_from_slice(&p.y.to_le_bytes());
}
struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let end = self.offset.checked_add(N)?;
        let value = self.bytes.get(self.offset..end)?.try_into().ok()?;
        self.offset = end;
        Some(value)
    }
    fn byte(&mut self) -> Option<u8> {
        Some(self.take::<1>()?[0])
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take()?))
    }
    fn float(&mut self) -> Option<f32> {
        let v = f32::from_le_bytes(self.take()?);
        v.is_finite().then_some(v)
    }
    fn point(&mut self) -> Option<Point> {
        Some(Point::new(self.float()?, self.float()?))
    }
}
pub fn decode(bytes: &[u8]) -> Option<Packet> {
    if bytes.len() > MAX_PACKET {
        return None;
    }
    let mut r = Reader { bytes, offset: 0 };
    if &r.take::<4>()? != b"FTH1" {
        return None;
    }
    let kind = r.byte()?;
    let token = r.take()?;
    let sequence = r.u64()?;
    let payload = match kind {
        0 => Payload::Register,
        1 => Payload::Ready,
        2 => {
            let time = f64::from_le_bytes(r.take()?);
            if !time.is_finite() || time < 0. {
                return None;
            }
            let ack = r.u64()?;
            let score = [r.u32()?, r.u32()?];
            let setup = r.float()?;
            let goal_flash = r.float()?;
            let ball_pressure = r.float()?;
            if !(0.0..=10.).contains(&setup)
                || !(0.0..=3.).contains(&goal_flash)
                || !(0.0..=1.01).contains(&ball_pressure)
            {
                return None;
            }
            let scoring_team = match r.byte()? {
                255 => None,
                t @ 0..=1 => Some(t as usize),
                _ => return None,
            };
            let round = r.u32()?;
            let ball = r.point()?;
            let velocity = r.point()?;
            if r.byte()? as usize != COUNT * 2 {
                return None;
            }
            let mut players = Vec::with_capacity(COUNT * 2);
            for _ in 0..COUNT * 2 {
                players.push(Player {
                    pos: r.point()?,
                    target: r.point()?,
                    facing: r.float()?,
                });
            }
            let state = Game {
                players,
                ball,
                velocity,
                score,
                setup,
                goal_flash,
                scoring_team,
                round,
                ball_pressure,
            };
            Payload::Snapshot { time, state, ack }
        }
        _ => return None,
    };
    (r.offset == bytes.len()).then_some(Packet {
        token,
        sequence,
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_snapshot_fits_one_small_datagram_and_round_trips() {
        let mut game = Game::new();
        game.score = [12, 8];
        game.goal_flash = 0.8;
        game.scoring_team = Some(1);
        game.ball = Point::new(15.7, -2.);
        let bytes = snapshot([7; 16], 42, 1.25, &game, 123).unwrap();
        assert!(bytes.len() < 512);
        let packet = decode(&bytes).unwrap();
        assert_eq!(packet.token, [7; 16]);
        assert_eq!(packet.sequence, 42);
        let Payload::Snapshot { time, state, ack } = packet.payload else {
            panic!("not a snapshot")
        };
        assert_eq!(time, 1.25);
        assert_eq!(ack, 123);
        assert_eq!(
            serde_json::to_value(state).unwrap(),
            serde_json::to_value(game).unwrap()
        );
    }
    #[test]
    fn truncated_extra_and_nonfinite_packets_are_rejected() {
        let bytes = snapshot([0; 16], 1, 1., &Game::new(), 0).unwrap();
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_none());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(decode(&extra).is_none());
        let mut bad = bytes;
        bad[29..37].copy_from_slice(&f64::NAN.to_le_bytes());
        assert!(decode(&bad).is_none());
    }
}
