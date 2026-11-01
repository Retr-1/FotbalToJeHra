mod asset_data;
mod ball_model;
mod game;
mod models;
mod network;
mod pointer;
use game::*;
use macroquad::prelude::*;
use network::{Message, Network};

const BLUE: Color = Color::new(0.22, 0.65, 1., 1.);
const RED: Color = Color::new(0.78, 0.40, 0.42, 1.);
const ORANGE: Color = Color::new(1., 0.48, 0.23, 1.);
fn conf() -> Conf {
    let phone_preview = std::env::args().any(|a| a.starts_with("--smoke-mobile"));
    Conf {
        window_title: "Fotbal To Je Hra".into(),
        window_width: if phone_preview { 960 } else { 1280 },
        window_height: if phone_preview { 432 } else { 800 },
        high_dpi: !cfg!(target_os = "android"),
        sample_count: if cfg!(target_os = "android") { 1 } else { 4 },
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
fn ground_cursor(height: f32, position: Vec2) -> Point {
    use macroquad::camera::Camera;
    let (x, y) = (position.x, position.y);
    let ndc = vec2(x / screen_width() * 2. - 1., 1. - y / screen_height() * 2.);
    let inv = camera(height).matrix().inverse();
    let near = inv.project_point3(vec3(ndc.x, ndc.y, -1.));
    let far = inv.project_point3(vec3(ndc.x, ndc.y, 1.));
    let direction = far - near;
    let hit = near + direction * (-near.y / direction.y);
    Point::new(hit.x, hit.z)
}
fn pick_distance(p: Point, height: f32, position: Vec2) -> f32 {
    use macroquad::camera::Camera;
    let projected = camera(height).matrix().project_point3(v(p, 0.85));
    let center = vec2(
        (projected.x + 1.) * 0.5 * screen_width(),
        (1. - projected.y) * 0.5 * screen_height(),
    );
    let radius = (0.9 * screen_height() / height).max(if cfg!(target_os = "android") {
        (screen_height() / 24.).clamp(22., 60.)
    } else {
        0.
    });
    center.distance(position) / radius
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
fn ui_scale() -> f32 {
    (screen_width() / 1000.)
        .min(screen_height() / 600.)
        .min(1.5)
}
fn ui_width() -> f32 {
    screen_width() / ui_scale()
}
fn ui_height() -> f32 {
    screen_height() / ui_scale()
}
fn ui_camera() {
    let mut camera = Camera2D::from_display_rect(Rect::new(0., 0., ui_width(), ui_height()));
    camera.zoom.y = -camera.zoom.y;
    set_camera(&camera);
}
fn button(rect: Rect, text: &str, input: &pointer::Pointer) -> bool {
    let hover = rect.contains(input.position / ui_scale());
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
    hover && input.pressed
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
struct RenderAssets {
    robots: [Option<models::Robot>; 2],
    ball: Option<ball_model::BallModel>,
}
fn draw_goal(sign: f32) {
    let front = sign * W;
    let back = sign * (W + 1.5);
    let post = GOAL + 0.07;
    let frame = Color::from_hex(0xe2e9ed);
    let net = Color::from_hex(0x728b8a);
    // All frame members and net panels share the same front/back planes.
    for x in [front, back] {
        for z in [-post, post] {
            draw_cube(vec3(x, 1., z), vec3(0.14, 2.14, 0.14), None, frame);
        }
        draw_cube(
            vec3(x, 2., 0.),
            vec3(0.14, 0.14, 2. * (post + 0.07)),
            None,
            frame,
        );
    }
    for z in [-post, post] {
        for y in [0.07, 2.] {
            draw_cube(
                vec3((front + back) * 0.5, y, z),
                vec3(1.5, 0.10, 0.10),
                None,
                frame,
            );
        }
    }
    for i in 0..=18 {
        let z = -post + 2. * post * i as f32 / 18.;
        draw_line_3d(vec3(back, 0.07, z), vec3(back, 2., z), net);
        draw_line_3d(vec3(front, 2., z), vec3(back, 2., z), net);
    }
    for i in 0..=8 {
        let y = 0.07 + (2. - 0.07) * i as f32 / 8.;
        draw_line_3d(vec3(back, y, -post), vec3(back, y, post), net);
        for z in [-post, post] {
            draw_line_3d(vec3(front, y, z), vec3(back, y, z), net);
        }
    }
    for i in 0..=6 {
        let x = front + (back - front) * i as f32 / 6.;
        for z in [-post, post] {
            draw_line_3d(vec3(x, 0.07, z), vec3(x, 2., z), net);
        }
        draw_line_3d(vec3(x, 2., -post), vec3(x, 2., post), net);
    }
}
fn draw_triangle_3d(a: Vec3, b: Vec3, c: Vec3, color: Color) {
    draw_mesh(&Mesh {
        vertices: vec![
            Vertex::new2(a, Vec2::ZERO, color),
            Vertex::new2(b, Vec2::ZERO, color),
            Vertex::new2(c, Vec2::ZERO, color),
        ],
        indices: vec![0, 1, 2],
        texture: None,
    });
}
fn scene(
    g: &Game,
    team: usize,
    selected: Option<usize>,
    cursor: Point,
    assets: &RenderAssets,
    locomotion: &[models::Locomotion],
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
        (
            Point::new(-W + CORNER_RADIUS, -H),
            Point::new(W - CORNER_RADIUS, -H),
        ),
        (
            Point::new(W, -H + CORNER_RADIUS),
            Point::new(W, H - CORNER_RADIUS),
        ),
        (
            Point::new(W - CORNER_RADIUS, H),
            Point::new(-W + CORNER_RADIUS, H),
        ),
        (
            Point::new(-W, H - CORNER_RADIUS),
            Point::new(-W, -H + CORNER_RADIUS),
        ),
        (Point::new(0., -H), Point::new(0., H)),
    ] {
        line(a, b, white);
    }
    for sx in [-1., 1.] {
        for sy in [-1., 1.] {
            let center = Point::new(sx * (W - CORNER_RADIUS), sy * (H - CORNER_RADIUS));
            for i in 0..32 {
                let a = i as f32 * std::f32::consts::FRAC_PI_2 / 32.;
                let b = (i + 1) as f32 * std::f32::consts::FRAC_PI_2 / 32.;
                let p = Point::new(
                    center.x + sx * CORNER_RADIUS * a.cos(),
                    center.y + sy * CORNER_RADIUS * a.sin(),
                );
                let q = Point::new(
                    center.x + sx * CORNER_RADIUS * b.cos(),
                    center.y + sy * CORNER_RADIUS * b.sin(),
                );
                // Cover the rectangular turf outside the collision boundary.
                draw_triangle_3d(
                    vec3(sx * W, 0.03, sy * H),
                    v(p, 0.03),
                    v(q, 0.03),
                    Color::from_hex(0x18392f),
                );
                line(p, q, white);
            }
        }
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
        draw_goal(sign);
    }
    for (i, p) in g.players.iter().enumerate() {
        let c = if i < COUNT { RED } else { BLUE };
        ring(p.pos, 0.55, c);
        if let Some(robot) = &assets.robots[i / COUNT] {
            let animation = &locomotion[i];
            robot.draw(
                v(p.pos, 0.),
                p.facing,
                animation.phase,
                get_time() as f32 + i as f32 * 0.2,
                animation.blend,
            )
        } else {
            fallback(p, c, i)
        }
        if i / COUNT == team && g.setup <= 0. && p.pos.distance(p.target) > 0.2 {
            arrow(p.pos, p.target, Color::new(c.r, c.g, c.b, 0.45));
        }
    }
    if let Some(i) = selected {
        ring(g.players[i].pos, 0.78, YELLOW);
        if g.setup <= 0. {
            arrow(g.players[i].pos, cursor, YELLOW);
        }
        ring(cursor, 0.25, YELLOW);
    }
    if let Some(ball) = &assets.ball {
        ball.draw(v(g.ball, BALL_RADIUS));
    } else {
        draw_sphere(v(g.ball, BALL_RADIUS), BALL_RADIUS, None, WHITE);
    }
    set_default_camera();
}

#[macroquad::main(conf)]
async fn main() {
    simulate_mouse_with_touch(false);
    let mut input = pointer::Pointer::default();
    let mut editing_address = false;
    let mut replace_address = false;
    let mut model_notes = vec![];
    let robots = [
        ("assets/generated/x_bot.glb", RED),
        ("assets/generated/y_bot.glb", BLUE),
    ]
    .map(|(path, c)| match models::Robot::load(path, c) {
        Ok(m) => Some(m),
        Err(e) => {
            model_notes.push(format!("{path}: {e}"));
            None
        }
    });
    let ball = match ball_model::BallModel::load("assets/models/ball.glb") {
        Ok(ball) => Some(ball),
        Err(e) => {
            model_notes.push(format!("assets/models/ball.glb: {e}"));
            None
        }
    };
    let assets = RenderAssets { robots, ball };
    for note in &model_notes {
        eprintln!("{note}");
    }
    let mut game = Game::new();
    let mut net: Option<Network> = None;
    let mut menu = true;
    let mut address = "127.0.0.1:7777".to_string();
    let mut error = String::new();
    let mut selected = None;
    let mut locomotion: Vec<models::Locomotion> = (0..COUNT * 2)
        .map(|_| models::Locomotion::default())
        .collect();
    let mut selected_round = 0;
    let mut accumulator = 0.;
    let mut send_timer = 0.;
    let args: Vec<String> = std::env::args().collect();
    let run_preview = args.iter().find_map(|a| {
        a.strip_prefix("--smoke-run=")
            .and_then(|v| v.parse::<f32>().ok())
    });
    let mobile_preview = args.iter().any(|a| a.starts_with("--smoke-mobile"));
    let smoke = args.iter().any(|a| a == "--smoke-test") || run_preview.is_some() || mobile_preview;
    let mut frames = 0;
    if smoke {
        menu = args.iter().any(|a| a == "--smoke-mobile-menu");
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
        input.update();
        clear_background(Color::from_hex(0x0b1721));
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Back) {
            miniquad::window::show_keyboard(false);
            editing_address = false;
            menu = true;
            net = None;
            selected = None;
        }
        if menu {
            ui_camera();
            let x = (ui_width() - 460.) / 2.;
            let y = (ui_height() - 480.) / 2.;
            draw_text("FOTBAL", x, y + 35., 56., WHITE);
            draw_text("TO JE HRA", x, y + 70., 26., BLUE);
            draw_text("Taktika. Pohyb. Gol.", x, y + 108., 22., GRAY);
            let ai = button(
                Rect::new(x, y + 140., 460., 55.),
                "Hrat proti pocitacu",
                &input,
            );
            let host = button(
                Rect::new(x, y + 208., 460., 55.),
                "Vytvorit multiplayer (port 7777)",
                &input,
            );
            draw_text("Adresa hostitela (IP:port):", x, y + 300., 20., LIGHTGRAY);
            draw_rectangle(x, y + 314., 460., 42., Color::from_hex(0x172b3c));
            let address_rect = Rect::new(x, y + 314., 460., 42.);
            if input.pressed && address_rect.contains(input.position / ui_scale()) {
                editing_address = true;
                replace_address = true;
                miniquad::window::show_keyboard(true);
            }
            while let Some(c) = get_char_pressed() {
                if (editing_address || !cfg!(target_os = "android"))
                    && (c.is_ascii_alphanumeric() || ".:-[]".contains(c))
                    && address.len() < 100
                {
                    if replace_address {
                        address.clear();
                        replace_address = false;
                    }
                    address.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                address.pop();
            }
            draw_text(format!("{address}_"), x + 12., y + 342., 23., WHITE);
            let join = button(
                Rect::new(x, y + 370., 460., 55.),
                "Pripojit sa k hracovi",
                &input,
            );
            if ai || host || join {
                editing_address = false;
                miniquad::window::show_keyboard(false);
            }
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
                    "Niektory model chyba: pouziva sa nahradna geometria.",
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
            // Fit the complete pitch and both goals without reserving HUD bars.
            let height = 16.8_f32.max(33.6 * screen_height() / screen_width());
            let cursor = ground_cursor(height, input.position);
            if selected_round != game.round {
                selected = None;
                selected_round = game.round;
            }
            let menu_rect = Rect::new(ui_width() - 120., 4., 105., 38.);
            if ready && input.pressed && !menu_rect.contains(input.position / ui_scale()) {
                selected = (team * COUNT..(team + 1) * COUNT)
                    .filter(|i| {
                        game.players[*i].pos.distance(cursor) < 0.9
                            || pick_distance(game.players[*i].pos, height, input.position) < 1.
                    })
                    .min_by(|a, b| {
                        game.players[*a]
                            .pos
                            .distance(cursor)
                            .total_cmp(&game.players[*b].pos.distance(cursor))
                    });
            }
            if let Some(i) = selected {
                if !ready || input.cancelled {
                    selected = None;
                } else if (game.setup > 0. && input.down) || input.released {
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
                if input.released {
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
            for (state, player) in locomotion.iter_mut().zip(&game.players) {
                state.update(
                    v(player.pos, 0.),
                    game.round,
                    ready && game.setup <= 0.,
                    get_frame_time().min(0.1),
                );
                if let Some(phase) = run_preview {
                    state.phase = phase;
                    state.blend = 1.;
                }
            }
            scene(&game, team, selected, cursor, &assets, &locomotion, height);
            ui_camera();
            let score = format!("{}  :  {}", game.score[0], game.score[1]);
            let score_width = measure_text(&score, None, 30, 1.).width;
            draw_rectangle(
                ui_width() / 2. - score_width / 2. - 14.,
                4.,
                score_width + 28.,
                38.,
                Color::new(0.03, 0.08, 0.1, 0.75),
            );
            draw_text(&score, ui_width() / 2. - score_width / 2., 32., 30., WHITE);
            if ready && game.setup > 0. {
                let countdown = format!("{} s", game.setup.ceil() as u32);
                draw_rectangle(12., 4., 76., 38., Color::new(0.03, 0.08, 0.1, 0.75));
                draw_text(&countdown, 24., 32., 30., YELLOW);
            }
            if button(menu_rect, "Menu", &input) {
                menu = true;
                net = None;
                selected = None;
            }
            if let Some(n) = &net {
                if n.waiting() {
                    let address = n
                        .host_address
                        .as_deref()
                        .unwrap_or("IP nedostupna - skontroluj Wi-Fi");
                    let width = measure_text(address, None, 32, 1.).width;
                    draw_rectangle(
                        ui_width() / 2. - width / 2. - 24.,
                        ui_height() / 2. - 34.,
                        width + 48.,
                        64.,
                        Color::new(0.03, 0.08, 0.1, 0.92),
                    );
                    draw_text(
                        address,
                        ui_width() / 2. - width / 2.,
                        ui_height() / 2. + 10.,
                        32.,
                        WHITE,
                    );
                }
            }
        }
        frames += 1;
        if smoke && frames == 8 {
            get_screen_data().export_png(if mobile_preview {
                "smoke-mobile.png"
            } else if run_preview.is_some() {
                "smoke-run.png"
            } else {
                "smoke-test.png"
            });
            break;
        }
        next_frame().await;
    }
}
