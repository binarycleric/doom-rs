use sdl2::{pixels::Color, event::Event, keyboard::Keycode, mouse::MouseButton, rect::Rect, render::Canvas, video::Window, EventPump, ttf::Sdl2TtfContext};
use std::time::{Duration, Instant};
use rand::Rng;
use rand::seq::SliceRandom;

const SCREEN_WIDTH: u32 = 800;
const SCREEN_HEIGHT: u32 = 600;
const PLAYER_SPEED: i32 = 5;
const BULLET_SPEED: i32 = 10;
const ENEMY_SPEED: i32 = 2;
const FIRE_RATE: Duration = Duration::from_secs(1);

fn generate_maze() -> Vec<(i32, i32, u32, u32)> {
    let cell_size: i32 = 40;
    let w: i32 = (SCREEN_WIDTH as i32) / cell_size; // 800 / 40 = 20
    let h: i32 = (SCREEN_HEIGHT as i32) / cell_size; // 600 / 40 = 15
    let mut rng = rand::thread_rng();

    // Mark all cells as walls initially
    let mut grid = vec![vec![true; w as usize]; h as usize];

    // Carve a path using DFS from (0,0)
    let mut stack = vec![(0, 0)];
    grid[0][0] = false;

    while let Some((cx, cy)) = stack.pop() {
        let mut neighbors = vec![];
        if cx > 0 { neighbors.push((cx - 1, cy)); }
        if cx < w - 1 { neighbors.push((cx + 1, cy)); }
        if cy > 0 { neighbors.push((cx, cy - 1)); }
        if cy < h - 1 { neighbors.push((cx, cy + 1)); }

        neighbors.shuffle(&mut rng);
        for (nx, ny) in neighbors {
            if grid[ny as usize][nx as usize] {
                grid[ny as usize][nx as usize] = false;
                stack.push((nx, ny));
            }
        }
    }

    // Convert walls in 'grid' to rectangles
    let mut maze = vec![];
    for row in 0..h {
        for col in 0..w {
            if grid[row as usize][col as usize] {
                let x = col * cell_size;
                let y = row * cell_size;
                maze.push((x, y, cell_size as u32, cell_size as u32));
            }
        }
    }

    // If the maze somehow ends up empty, force at least one wall in the center
    if maze.is_empty() {
        let x = (w / 2) * cell_size;
        let y = (h / 2) * cell_size;
        maze.push((x, y, cell_size as u32, cell_size as u32));
    }

    // Add bounding walls around the maze
    let total_width = w * cell_size;  // i32
    let total_height = h * cell_size; // i32
    maze.push((0,               0,                total_width as u32, 20));  // top
    maze.push((0,               total_height - 20, total_width as u32, 20));  // bottom
    maze.push((0,               0,                20, total_height as u32));  // left
    maze.push((total_width - 20, 0,        20, total_height as u32));  // right

    println!("{:?}", grid);

    maze
}

#[derive(Clone, PartialEq)]
struct Enemy {
    x: i32,
    y: i32,
    health: i32,
    alive: bool,
}

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

struct AmmoDrop {
    x: i32,
    y: i32,
    active: bool,
}

impl Player {
    fn move_player(&mut self, keys: &[Keycode], maze: &[(i32, i32, u32, u32)]) {
        let mut new_x = self.x;
        let mut new_y = self.y;

        if keys.contains(&Keycode::A) { new_x -= PLAYER_SPEED; }
        if keys.contains(&Keycode::D) { new_x += PLAYER_SPEED; }
        if keys.contains(&Keycode::W) { new_y -= PLAYER_SPEED; }
        if keys.contains(&Keycode::S) { new_y += PLAYER_SPEED; }

        if !self.collides_with_maze(new_x, new_y, maze) {
            self.x = new_x;
            self.y = new_y;
        }
    }

    fn collides_with_maze(&self, x: i32, y: i32, maze: &[(i32, i32, u32, u32)]) -> bool {
        for &(mx, my, mw, mh) in maze {
            if x < mx + mw as i32 && x + 20 > mx && y < my + mh as i32 && y + 20 > my {
                return true;
            }
        }
        false
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
    fn update(&mut self, player_x: i32, player_y: i32, other_enemies: &[Enemy], maze: &[(i32, i32, u32, u32)]) {
        if !self.alive {
            return;
        }

        let dx = player_x - self.x;
        let dy = player_y - self.y;
        let distance = ((dx * dx + dy * dy) as f64).sqrt();
        let vx = (ENEMY_SPEED as f64 * dx as f64 / distance) as i32;
        let vy = (ENEMY_SPEED as f64 * dy as f64 / distance) as i32;

        let mut new_x = self.x + vx;
        let mut new_y = self.y + vy;

        if !self.collides_with_maze(new_x, new_y, maze) && !self.collides_with_enemies(new_x, new_y, other_enemies) {
            self.x = new_x;
            self.y = new_y;
        }
    }

    fn collides_with_maze(&self, x: i32, y: i32, maze: &[(i32, i32, u32, u32)]) -> bool {
        for &(mx, my, mw, mh) in maze {
            if x < mx + mw as i32 && x + 20 > mx && y < my + mh as i32 && y + 20 > my {
                return true;
            }
        }
        false
    }

    fn collides_with_enemies(&self, x: i32, y: i32, other_enemies: &[Enemy]) -> bool {
        for enemy in other_enemies {
            if enemy.alive && enemy.x != self.x && enemy.y != self.y && x < enemy.x + 20 && x + 20 > enemy.x && y < enemy.y + 20 && y + 20 > enemy.y {
                return true;
            }
        }
        false
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

fn render_enemy_health(canvas: &mut Canvas<Window>, ttf_context: &Sdl2TtfContext, enemy: &Enemy) {
    let font = ttf_context.load_font("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 16).unwrap();
    let texture_creator = canvas.texture_creator();
    let surface = font.render(&format!("{}", enemy.health))
        .blended(Color::WHITE).unwrap();
    let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
    canvas.copy(&texture, None, Rect::new(enemy.x, enemy.y - 20, 20, 20)).unwrap();
}

fn render_ammo_drop(canvas: &mut Canvas<Window>, ttf_context: &Sdl2TtfContext, ammo_drop: &AmmoDrop) {
    let font = ttf_context.load_font("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 20).unwrap();
    let texture_creator = canvas.texture_creator();
    let surface = font.render("B")
        .blended(Color::WHITE).unwrap();
    let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
    canvas.copy(&texture, None, Rect::new(ammo_drop.x, ammo_drop.y, 20, 20)).unwrap();
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

    let mut player = Player { x: 120, y: 120, health: 100, ammo: 50, last_shot: Instant::now() };
    let mut bullets = vec![];
    let mut enemies = vec![
        Enemy { x: 400, y: 120, health: 3, alive: true },
        Enemy { x: 450, y: 120, health: 3, alive: true },
        Enemy { x: 500, y: 120, health: 3, alive: true },
        Enemy { x: 550, y: 120, health: 3, alive: true }
    ];
    let mut ammo_drops = vec![];

    let maze = generate_maze();
    println!("{:?}", maze);

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
        player.move_player(&keys, &maze);

        bullets.iter_mut().for_each(Bullet::update);
        bullets.retain(|b| b.active);

        let other_enemies: Vec<_> = enemies.iter().cloned().collect();
        enemies.iter_mut().for_each(|enemy| {
            enemy.update(player.x, player.y, &other_enemies, &maze);
        });
        enemies.retain(|e| e.alive);

        bullets.iter_mut().for_each(|bullet| {
            enemies.iter_mut().for_each(|enemy| {
                if bullet.active && enemy.alive && (bullet.x - enemy.x).abs() < 20 && (bullet.y - enemy.y).abs() < 20 {
                    enemy.health -= 1;
                    bullet.active = false;
                    if enemy.health <= 0 {
                        enemy.alive = false;
                        ammo_drops.push(AmmoDrop { x: enemy.x, y: enemy.y, active: true });
                    }
                }
            });
        });

        // Check for player picking up ammo drops
        ammo_drops.iter_mut().for_each(|ammo_drop| {
            if ammo_drop.active && (player.x - ammo_drop.x).abs() < 20 && (player.y - ammo_drop.y).abs() < 20 {
                player.ammo += 5;
                ammo_drop.active = false;
            }
        });

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // Draw the maze
        canvas.set_draw_color(Color::RGB(255, 255, 255));

        for &(x, y, w, h) in &maze {
            println!("{} {} {} {}", x, y, w, h);
            canvas.fill_rect(Rect::new(x, y, w, h)).unwrap();
        }

        canvas.set_draw_color(Color::RGB(0, 255, 0));
        canvas.fill_rect(Rect::new(player.x, player.y, 20, 20)).unwrap();

        for enemy in &enemies {
            if enemy.alive {
                match enemy.health {
                    3 => canvas.set_draw_color(Color::RGB(0, 255, 0)),
                    2 => canvas.set_draw_color(Color::RGB(255, 255, 0)),
                    1 => canvas.set_draw_color(Color::RGB(255, 0, 0)),
                    _ => {}
                }
                canvas.fill_rect(Rect::new(enemy.x, enemy.y, 20, 20)).unwrap();
                render_enemy_health(&mut canvas, &ttf_context, enemy);
            } else {
                // Draw explosion effect for dead enemies
                canvas.set_draw_color(Color::RGB(255, 0, 0));
                canvas.fill_rect(Rect::new(enemy.x - 10, enemy.y - 10, 40, 40)).unwrap();
            }
        }

        for ammo_drop in &ammo_drops {
            if ammo_drop.active {
                render_ammo_drop(&mut canvas, &ttf_context, ammo_drop);
            }
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