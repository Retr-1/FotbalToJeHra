use macroquad::prelude::*;

#[derive(Default)]
pub struct Pointer {
    pub position: Vec2,
    pub pressed: bool,
    pub down: bool,
    pub released: bool,
    pub cancelled: bool,
    active: Option<u64>,
}
impl Pointer {
    pub fn update(&mut self) {
        self.pressed = false;
        self.released = false;
        self.cancelled = false;
        let mut contacts = touches();
        contacts.sort_by_key(|t| t.id);
        self.apply_touches(&contacts, miniquad::window::dpi_scale());
        if self.active.is_none() && contacts.is_empty() && !self.cancelled {
            self.position = Vec2::from(mouse_position());
            self.pressed = is_mouse_button_pressed(MouseButton::Left);
            self.down = is_mouse_button_down(MouseButton::Left);
            self.released = is_mouse_button_released(MouseButton::Left);
        }
    }
    fn apply_touches(&mut self, contacts: &[Touch], dpi: f32) {
        if self.active.is_none() {
            if let Some(t) = contacts.iter().find(|t| t.phase == TouchPhase::Started) {
                self.active = Some(t.id);
                self.pressed = true;
                self.down = true;
            }
        }
        if let Some(id) = self.active {
            if let Some(t) = contacts.iter().find(|t| t.id == id) {
                self.position = t.position / dpi;
                match t.phase {
                    TouchPhase::Ended => {
                        self.released = true;
                        self.down = false;
                        self.active = None;
                    }
                    TouchPhase::Cancelled => {
                        self.cancelled = true;
                        self.down = false;
                        self.active = None;
                    }
                    _ => self.down = true,
                }
            } else {
                self.cancelled = true;
                self.down = false;
                self.active = None;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn touch(id: u64, phase: TouchPhase, x: f32) -> Touch {
        Touch {
            id,
            phase,
            position: vec2(x, 20.),
        }
    }
    #[test]
    fn second_finger_does_not_steal_or_release_drag() {
        let mut p = Pointer::default();
        p.apply_touches(&[touch(1, TouchPhase::Started, 100.)], 2.);
        assert!(p.pressed && p.down);
        assert_eq!(p.position.x, 50.);
        p.apply_touches(
            &[
                touch(1, TouchPhase::Moved, 120.),
                touch(2, TouchPhase::Ended, 500.),
            ],
            2.,
        );
        assert!(p.down && !p.released);
        assert_eq!(p.position.x, 60.);
        p.apply_touches(&[touch(1, TouchPhase::Ended, 130.)], 2.);
        assert!(p.released && !p.down);
    }
    #[test]
    fn interrupted_drag_is_cancelled_not_committed() {
        let mut p = Pointer::default();
        p.apply_touches(&[touch(1, TouchPhase::Started, 100.)], 1.);
        p.apply_touches(&[touch(1, TouchPhase::Cancelled, 100.)], 1.);
        assert!(p.cancelled && !p.released && !p.down);
    }
}
