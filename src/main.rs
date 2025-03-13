use sdl2::{pixels::Color, event::Event, keyboard::Keycode, mouse::MouseButton, rect::Rect, render::Canvas, video::Window, EventPump, ttf::Sdl2TtfContext};
use std::time::{Duration, Instant};
use rand::Rng;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;
const PLAYER_SPEED: i32 = 5;
const BULLET_SPEED: i32 = 10;
const ENEMY_SPEED: i32 = 2;
const FIRE_RATE: Duration = Duration::from_secs(1);

struct Player {
    x: i32,
    y: i32,
    health: i32,
    ammo: i32,
    last_shot: Instant,
}

struct Bullet {
    x: i32,
    y: i32,
    vx: i32,
    vy: i32,
    active: bool,
}

struct Enemy {
    x: i32,
    y: i32,
    alive: bool,
}

impl Player {
    fn move_player(&mut self, keys: &[Keycode]) {
        if keys.contains(&Keycode::A) { self.x -= PLAYER_SPEED; }
        if keys.contains(&Keycode::D) { self.x += PLAYER_SPEED; }
        if keys.contains(&Keycode::W) { self.y -= PLAYER_SPEED; }
        if keys.contains(&Keycode::S) { self.y += PLAYER_SPEED; }
    }

    fn shoot(&mut self, target_x: i32, target_y: i32) -> Option<Bullet> {
        if self.ammo > 0 && self.last_shot.elapsed() >= FIRE_RATE {
            self.ammo -= 1;
            self.last_shot = Instant::now();
            let dx = target_x - self.x;
            let dy = target_y - self.y;
            let distance = ((dx * dx + dy * dy) as f64).sqrt();
            let vx = (BULLET_SPEED as f64 * dx as f64 / distance) as i32;
            let vy = (BULLET_SPEED as f64 * dy as f64 / distance) as i32;
            Some(Bullet { x: self.x, y: self.y, vx, vy, active: true })
        } else {
            None
        }
    }
}

impl Bullet {
    fn update(&mut self) {
        self.x += self.vx;
        self.y += self.vy;
        self.active &= self.x >= 0 && self.x <= SCREEN_WIDTH as i32 && self.y >= 0 && self.y <= SCREEN_HEIGHT as i32;
    }
}

impl Enemy {
    fn update(&mut self, player_x: i32, player_y: i32) {
        if self.x < player_x {
            self.x += ENEMY_SPEED;
        } else if self.x > player_x {
            self.x -= ENEMY_SPEED;
        }

        if self.y < player_y {
            self.y += ENEMY_SPEED;
        } else if self.y > player_y {
            self.y -= ENEMY_SPEED;
        }

        self.alive &= self.y <= SCREEN_HEIGHT as i32;
    }
}

fn render_hud(canvas: &mut Canvas<Window>, ttf_context: &Sdl2TtfContext, player: &Player) {
    let font = ttf_context.load_font("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 24).unwrap();
    let texture_creator = canvas.texture_creator();
    let surface = font.render(&format!("Health: {}  Ammo: {}", player.health, player.ammo))
        .blended(Color::WHITE).unwrap();
    let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
    canvas.copy(&texture, None, Rect::new(10, 10, 200, 30)).unwrap();
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let window = video_subsystem
        .window("Doom-like FPS", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut player = Player { x: 400, y: 500, health: 100, ammo: 50, last_shot: Instant::now() };
    let mut bullets = vec![];
    let mut enemies = vec![
        Enemy { x: 400, y: 100, alive: true },
        Enemy { x: 450, y: 100, alive: true },
        Enemy { x: 500, y: 100, alive: true },
        Enemy { x: 550, y: 100, alive: true }
    ];

    'running: loop {
        let mouse_state = event_pump.mouse_state();
        let mouse_x = mouse_state.x();
        let mouse_y = mouse_state.y();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                Event::MouseButtonDown { mouse_btn: MouseButton::Left, .. } => {
                    if let Some(bullet) = player.shoot(mouse_x, mouse_y) {
                        bullets.push(bullet);
                    }
                }
                _ => {}
            }
        }

        let keys: Vec<Keycode> = event_pump.keyboard_state().pressed_scancodes().filter_map(Keycode::from_scancode).collect();
        player.move_player(&keys);

        bullets.iter_mut().for_each(Bullet::update);
        bullets.retain(|b| b.active);
        enemies.iter_mut().for_each(|enemy| enemy.update(player.x, player.y));
        enemies.retain(|e| e.alive);

        bullets.iter_mut().for_each(|bullet| {
            enemies.iter_mut().for_each(|enemy| {
                if bullet.active && enemy.alive && (bullet.x - enemy.x).abs() < 20 && (bullet.y - enemy.y).abs() < 20 {
                    enemy.alive = false;
                    bullet.active = false;
                    player.ammo += 2;
                }
            });
        });

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        canvas.set_draw_color(Color::RGB(0, 255, 0));
        canvas.fill_rect(Rect::new(player.x, player.y, 20, 20)).unwrap();

        canvas.set_draw_color(Color::RGB(255, 0, 0));
        for enemy in &enemies {
            canvas.fill_rect(Rect::new(enemy.x, enemy.y, 20, 20)).unwrap();
        }

        canvas.set_draw_color(Color::RGB(255, 255, 0));
        for bullet in &bullets {
            canvas.fill_rect(Rect::new(bullet.x, bullet.y, 5, 10)).unwrap();
        }

        // Draw the crosshair cursor
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.draw_line((mouse_x - 10, mouse_y), (mouse_x + 10, mouse_y)).unwrap();
        canvas.draw_line((mouse_x, mouse_y - 10), (mouse_x, mouse_y + 10)).unwrap();

        render_hud(&mut canvas, &ttf_context, &player);

        canvas.present();
        ::std::thread::sleep(Duration::from_millis(16));
    }
}