mod game;
mod models;
mod network;
use game::*;
use macroquad::prelude::*;
use network::{Message, Network};

const BLUE: Color = Color::new(0.22, 0.65, 1., 1.);
const ORANGE: Color = Color::new(1., 0.48, 0.23, 1.);
fn conf() -> Conf {
    Conf {
        window_title: "Fotbal To Je Hra".into(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}
fn v(p: Point, h: f32) -> Vec3 {
    vec3(p.x, h, p.y)
}
fn camera(height: f32) -> Camera3D {
    Camera3D {
        position: vec3(0., 28., 18.),
        target: Vec3::ZERO,
        up: Vec3::Y,
        projection: Projection::Orthographics,
        fovy: height,
        z_near: 0.1,
        z_far: 100.,
        ..Default::default()
    }
}
fn ground_cursor(height: f32) -> Point {
    use macroquad::camera::Camera;
    let (x, y) = mouse_position();
    let ndc = vec2(x / screen_width() * 2. - 1., 1. - y / screen_height() * 2.);
    let inv = camera(height).matrix().inverse();
    let near = inv.project_point3(vec3(ndc.x, ndc.y, -1.));
    let far = inv.project_point3(vec3(ndc.x, ndc.y, 1.));
    let direction = far - near;
    let hit = near + direction * (-near.y / direction.y);
    Point::new(hit.x, hit.z)
}
fn pick_distance(p: Point, height: f32) -> f32 {
    use macroquad::camera::Camera;
    let projected = camera(height).matrix().project_point3(v(p, 0.85));
    let center = vec2(
        (projected.x + 1.) * 0.5 * screen_width(),
        (1. - projected.y) * 0.5 * screen_height(),
    );
    let radius = 0.9 * screen_height() / height;
    center.distance(Vec2::from(mouse_position())) / radius
}
fn line(a: Point, b: Point, c: Color) {
    draw_line_3d(v(a, 0.04), v(b, 0.04), c);
}
fn ring(p: Point, r: f32, c: Color) {
    for i in 0..48 {
        let a = i as f32 / 48. * std::f32::consts::TAU;
        let b = (i + 1) as f32 / 48. * std::f32::consts::TAU;
        line(
            Point::new(p.x + a.cos() * r, p.y + a.sin() * r),
            Point::new(p.x + b.cos() * r, p.y + b.sin() * r),
            c,
        );
    }
}
fn arrow(a: Point, b: Point, c: Color) {
    line(a, b, c);
    let delta = vec2(b.x - a.x, b.y - a.y).normalize_or_zero();
    for s in [-1., 1.] {
        line(
            b,
            Point::new(
                b.x - delta.x * 0.6 - delta.y * s * 0.3,
                b.y - delta.y * 0.6 + delta.x * s * 0.3,
            ),
            c,
        );
    }
}
fn button(rect: Rect, text: &str) -> bool {
    let hover = rect.contains(Vec2::from(mouse_position()));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hover {
            Color::from_hex(0x29445a)
        } else {
            Color::from_hex(0x172b3c)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.,
        if hover {
            BLUE
        } else {
            Color::from_hex(0x385063)
        },
    );
    draw_text(text, rect.x + 20., rect.y + rect.h / 2. + 8., 25., WHITE);
    hover && is_mouse_button_pressed(MouseButton::Left)
}
fn fallback(p: &Player, c: Color, index: usize) {
    let pos = v(p.pos, 0.);
    let walking = p.pos.distance(p.target) > 0.1;
    let stride = if walking {
        (get_time() as f32 * 11. + index as f32).sin() * 0.16
    } else {
        0.
    };
    draw_cube(pos + vec3(0., 1., 0.), vec3(0.52, 0.65, 0.32), None, c);
    draw_sphere(
        pos + vec3(0., 1.65, 0.),
        0.23,
        None,
        Color::from_hex(0xdde8ee),
    );
    for sign in [-1., 1.] {
        draw_cube(
            pos + vec3(sign * 0.18, 0.35, sign * stride),
            vec3(0.18, 0.65, 0.20),
            None,
            c,
        );
        draw_cube(
            pos + vec3(sign * 0.40, 1., -sign * stride),
            vec3(0.16, 0.6, 0.18),
            None,
            Color::from_hex(0xa3b7c5),
        );
    }
    arrow(
        p.pos,
        Point::new(
            p.pos.x + p.facing.cos() * 0.85,
            p.pos.y + p.facing.sin() * 0.85,
        ),
        c,
    );
}
fn scene(
    g: &Game,
    team: usize,
    selected: Option<usize>,
    cursor: Point,
    robots: &[Option<models::Robot>; 2],
    height: f32,
) {
    set_camera(&camera(height));
    draw_plane(Vec3::ZERO, vec2(17., 10.5), None, Color::from_hex(0x18392f));
    for i in 0..10 {
        draw_plane(
            vec3(-13.5 + i as f32 * 3., 0.01, 0.),
            vec2(1.5, 9.),
            None,
            if i % 2 == 0 {
                Color::from_hex(0x28664c)
            } else {
                Color::from_hex(0x2d7053)
            },
        );
    }
    if g.setup > 0. {
        let x = if team == 0 { -7.5 } else { 7.5 };
        draw_plane(
            vec3(x, 0.02, 0.),
            vec2(7.5, 9.),
            None,
            Color::new(0.2, 0.5, 0.8, 0.12),
        );
    }
    let white = Color::new(0.82, 0.92, 0.83, 1.);
    for (a, b) in [
        (Point::new(-W, -H), Point::new(W, -H)),
        (Point::new(W, -H), Point::new(W, H)),
        (Point::new(W, H), Point::new(-W, H)),
        (Point::new(-W, H), Point::new(-W, -H)),
        (Point::new(0., -H), Point::new(0., H)),
    ] {
        line(a, b, white);
    }
    ring(Point::default(), 2.6, white);
    for sign in [-1., 1.] {
        for y in [-5.5, 5.5] {
            line(Point::new(sign * W, y), Point::new(sign * 10., y), white);
        }
        line(
            Point::new(sign * 10., -5.5),
            Point::new(sign * 10., 5.5),
            white,
        );
        let c = if sign < 0. { BLUE } else { ORANGE };
        for y in [-GOAL, GOAL] {
            draw_cube(vec3(sign * W, 1., y), vec3(0.14, 2., 0.14), None, WHITE);
            draw_line_3d(vec3(sign * W, 2., y), vec3(sign * 16.5, 2., y), c);
            draw_line_3d(vec3(sign * 16.5, 0., y), vec3(sign * 16.5, 2., y), c);
        }
        draw_cube(
            vec3(sign * W, 2., 0.),
            vec3(0.14, 0.14, GOAL * 2.),
            None,
            WHITE,
        );
        for i in 0..12 {
            let y = -GOAL + i as f32 * GOAL * 2. / 11.;
            draw_line_3d(
                vec3(sign * 16.5, 0., y),
                vec3(sign * 16.5, 2., y),
                Color::from_hex(0x63827d),
            );
        }
        for h in [0., 0.5, 1., 1.5, 2.] {
            draw_line_3d(
                vec3(sign * 16.5, h, -GOAL),
                vec3(sign * 16.5, h, GOAL),
                Color::from_hex(0x63827d),
            );
        }
    }
    for (i, p) in g.players.iter().enumerate() {
        let c = if i < COUNT { BLUE } else { ORANGE };
        ring(p.pos, 0.55, c);
        if let Some(robot) = &robots[i / COUNT] {
            robot.draw(v(p.pos, 0.), p.facing)
        } else {
            fallback(p, c, i)
        }
        if i / COUNT == team && g.setup <= 0. && p.pos.distance(p.target) > 0.2 {
            arrow(p.pos, p.target, Color::new(c.r, c.g, c.b, 0.45));
        }
    }
    if let Some(i) = selected {
        ring(g.players[i].pos, 0.78, YELLOW);
        arrow(g.players[i].pos, cursor, YELLOW);
        ring(cursor, 0.25, YELLOW);
    }
    draw_sphere(v(g.ball, 0.32), 0.32, None, WHITE);
    draw_cube(v(g.ball, 0.60), vec3(0.22, 0.05, 0.22), None, BLACK);
    set_default_camera();
}

#[macroquad::main(conf)]
async fn main() {
    let mut model_notes = vec![];
    let robots = [("assets/x_bot.glb", BLUE), ("assets/y_bot.glb", ORANGE)].map(|(path, c)| {
        match models::Robot::load(path, c) {
            Ok(m) => Some(m),
            Err(e) => {
                model_notes.push(format!("{path}: {e}"));
                None
            }
        }
    });
    for note in &model_notes {
        eprintln!("{note}");
    }
    let mut game = Game::new();
    let mut net: Option<Network> = None;
    let mut menu = true;
    let mut address = "127.0.0.1:7777".to_string();
    let mut error = String::new();
    let mut selected = None;
    let mut selected_round = 0;
    let mut accumulator = 0.;
    let mut send_timer = 0.;
    let args: Vec<String> = std::env::args().collect();
    let smoke = args.iter().any(|a| a == "--smoke-test");
    let mut frames = 0;
    if smoke {
        menu = false;
    }
    if let Some(mode) = args.get(1) {
        match mode.as_str() {
            "--ai" => menu = false,
            "--host" => {
                match Network::host(args.get(2).map(String::as_str).unwrap_or("0.0.0.0:7777")) {
                    Ok(n) => {
                        net = Some(n);
                        menu = false
                    }
                    Err(e) => error = e.to_string(),
                }
            }
            "--join" => match Network::join(args.get(2).map(String::as_str).unwrap_or(&address)) {
                Ok(n) => {
                    net = Some(n);
                    menu = false
                }
                Err(e) => error = e.to_string(),
            },
            _ => {}
        }
    }
    loop {
        clear_background(Color::from_hex(0x0b1721));
        if is_key_pressed(KeyCode::Escape) {
            menu = true;
            net = None;
            selected = None;
        }
        if menu {
            let x = (screen_width() - 460.) / 2.;
            let y = (screen_height() - 480.) / 2.;
            draw_text("FOTBAL", x, y + 35., 56., WHITE);
            draw_text("TO JE HRA", x, y + 70., 26., BLUE);
            draw_text("Taktika. Pohyb. Gol.", x, y + 108., 22., GRAY);
            let ai = button(Rect::new(x, y + 140., 460., 55.), "Hrat proti pocitacu");
            let host = button(
                Rect::new(x, y + 208., 460., 55.),
                "Vytvorit multiplayer (port 7777)",
            );
            draw_text("Adresa hostitela (IP:port):", x, y + 300., 20., LIGHTGRAY);
            draw_rectangle(x, y + 314., 460., 42., Color::from_hex(0x172b3c));
            while let Some(c) = get_char_pressed() {
                if (c.is_ascii_alphanumeric() || ".:-[]".contains(c)) && address.len() < 100 {
                    address.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                address.pop();
            }
            draw_text(format!("{address}_"), x + 12., y + 342., 23., WHITE);
            let join = button(Rect::new(x, y + 370., 460., 55.), "Pripojit sa k hracovi");
            if ai {
                game = Game::new();
                net = None;
                menu = false;
                error.clear();
            }
            if host || join {
                let result = if host {
                    Network::host("0.0.0.0:7777")
                } else {
                    Network::join(&address)
                };
                match result {
                    Ok(n) => {
                        game = Game::new();
                        net = Some(n);
                        menu = false;
                        error.clear()
                    }
                    Err(e) => error = e.to_string(),
                }
            }
            draw_text(&error, x, y + 454., 19., ORANGE);
            if !model_notes.is_empty() {
                draw_text(
                    "Mixamo GLB chyba: pouzivaju sa nahradne roboty.",
                    x,
                    y + 482.,
                    18.,
                    GRAY,
                );
            }
            accumulator = 0.;
        } else {
            let team = if net.as_ref().is_some_and(|n| !n.host) {
                1
            } else {
                0
            };
            let authority = team == 0;
            if let Some(n) = &mut net {
                for m in n.poll() {
                    match m {
                        Message::Input(c) if n.host => game.command(1, c),
                        Message::State(g) if !n.host => game = g,
                        _ => {}
                    }
                }
            }
            let ready = net.as_ref().is_none_or(|n| n.connected());
            let height = 24_f32.max(36. * screen_height() / screen_width());
            let mouse = mouse_position();
            let cursor = ground_cursor(height);
            if selected_round != game.round {
                selected = None;
                selected_round = game.round;
            }
            if ready && mouse.1 > 110. && is_mouse_button_pressed(MouseButton::Left) {
                selected = (team * COUNT..(team + 1) * COUNT)
                    .filter(|i| {
                        game.players[*i].pos.distance(cursor) < 0.9
                            || pick_distance(game.players[*i].pos, height) < 1.
                    })
                    .min_by(|a, b| {
                        game.players[*a]
                            .pos
                            .distance(cursor)
                            .total_cmp(&game.players[*b].pos.distance(cursor))
                    });
            }
            if let Some(i) = selected {
                if !ready {
                    selected = None;
                } else if (game.setup > 0. && is_mouse_button_down(MouseButton::Left))
                    || is_mouse_button_released(MouseButton::Left)
                {
                    let c = Command {
                        player: i,
                        target: cursor,
                        round: game.round,
                    };
                    if authority {
                        game.command(team, c)
                    } else if let Some(n) = &mut net {
                        n.send(Message::Input(c))
                    }
                }
                if is_mouse_button_released(MouseButton::Left) {
                    selected = None;
                }
            }
            if authority && ready {
                accumulator += get_frame_time().min(0.1);
                while accumulator >= 1. / 120. {
                    if net.is_none() {
                        game.ai(1)
                    }
                    game.tick(1. / 120.);
                    accumulator -= 1. / 120.;
                }
                send_timer += get_frame_time();
                if send_timer >= 1. / 30. {
                    if let Some(n) = &mut net {
                        n.send(Message::State(game.clone()));
                    }
                    send_timer = 0.;
                }
            } else {
                accumulator = 0.;
            }
            scene(&game, team, selected, cursor, &robots, height);
            draw_rectangle(0., 0., screen_width(), 105., Color::from_hex(0x0b1721));
            draw_text("FOTBAL / TO JE HRA", 24., 31., 23., WHITE);
            draw_text(
                format!("X BOT   {}  :  {}   Y BOT", game.score[0], game.score[1]),
                screen_width() / 2. - 150.,
                39.,
                32.,
                WHITE,
            );
            let phase = if !ready {
                "Cakam na pripojenie supera".into()
            } else if game.setup > 0. {
                format!("ROZOSTAVENIE  /  {} s", game.setup.ceil() as u32)
            } else {
                "HRA  /  potiahni hraca a pusti smer".into()
            };
            draw_text(
                &phase,
                24.,
                66.,
                23.,
                if game.setup > 0. { YELLOW } else { GREEN },
            );
            draw_text(
                format!(
                    "Tvoj tim: {}   |   ESC: menu",
                    if team == 0 {
                        "modry / X Bot"
                    } else {
                        "oranzovy / Y Bot"
                    }
                ),
                24.,
                91.,
                18.,
                LIGHTGRAY,
            );
            if let Some(n) = &net {
                draw_text(&n.status, screen_width() - 390., 85., 18., LIGHTGRAY);
            }
            draw_rectangle(
                0.,
                screen_height() - 42.,
                screen_width(),
                42.,
                Color::from_hex(0x0b1721),
            );
            draw_text(
                if game.setup > 0. {
                    "15 sekund: presun svojich hracov na vlastnej polovici. Potom sa hra spusti automaticky."
                } else {
                    "Podrz hraca, namier sipku a pusti. Hraci sa pohybuju sucasne. Kontakt s loptou = kop."
                },
                24.,
                screen_height() - 15.,
                19.,
                LIGHTGRAY,
            );
            if game.goal_flash > 0. {
                draw_text(
                    "G O L !",
                    screen_width() / 2. - 100.,
                    screen_height() / 2.,
                    64.,
                    YELLOW,
                );
            }
        }
        frames += 1;
        if smoke && frames == 8 {
            get_screen_data().export_png("smoke-test.png");
            break;
        }
        next_frame().await;
    }
}
