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

/// A faster A* pathfinding method to replace the slower BFS.
fn find_path(maze: &[(i32, i32, u32, u32)], start: (i32, i32), end: (i32, i32)) -> Option<Vec<(i32, i32)>> {
    use std::collections::{BinaryHeap, HashMap, HashSet};
    #[derive(Copy, Clone, Eq, PartialEq)]
    struct Node {
        pos: (i32, i32),
        f_score: i32,
    }
    impl Ord for Node {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            other.f_score.cmp(&self.f_score)
        }
    }
    impl PartialOrd for Node {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut blocked = HashSet::new();
    for &(mx, my, mw, mh) in maze {
        for ry in my..my + mh as i32 {
            for rx in mx..mx + mw as i32 {
                blocked.insert((rx, ry));
            }
        }
    }

    let heuristic = |p: (i32, i32)| -> i32 {
        let dx = (p.0 - end.0).abs();
        let dy = (p.1 - end.1).abs();
        dx + dy
    };

    let mut open_set = BinaryHeap::new();
    let mut came_from = HashMap::new();
    let mut g_score = HashMap::new();

    g_score.insert(start, 0);
    open_set.push(Node { pos: start, f_score: heuristic(start) });

    while let Some(Node { pos, .. }) = open_set.pop() {
        if pos == end {
            let mut path = vec![pos];
            let mut current = pos;
            while let Some(&prev) = came_from.get(&current) {
                path.push(prev);
                current = prev;
            }
            path.reverse();
            return Some(path);
        }
        for &(dx, dy) in &[(0,1),(0,-1),(1,0),(-1,0)] {
            let nx = pos.0 + dx;
            let ny = pos.1 + dy;
            if nx >= 0 && ny >= 0 && !blocked.contains(&(nx, ny)) {
                let tentative_g = g_score.get(&pos).unwrap_or(&i32::MAX) + 1;
                if tentative_g < *g_score.get(&(nx, ny)).unwrap_or(&i32::MAX) {
                    came_from.insert((nx, ny), pos);
                    g_score.insert((nx, ny), tentative_g);
                    let f_score = tentative_g + heuristic((nx, ny));
                    open_set.push(Node { pos: (nx, ny), f_score });
                }
            }
        }
    }
    None
}

impl Enemy {
    fn update(&mut self, player_x: i32, player_y: i32, other_enemies: &[Enemy], maze: &[(i32, i32, u32, u32)]) {
        if !self.alive {
            return;
        }

        // Instead of directly charging the player, use BFS to find a path.
        let start = (self.x, self.y);
        let end = (player_x, player_y);
        if let Some(path) = find_path(maze, start, end) {
            // Move only a small step along the path
            if path.len() > 1 {
                let next = path[1];
                // ...existing collision checks...
                if !self.collides_with_maze(next.0, next.1, maze)
                    && !self.collides_with_enemies(next.0, next.1, other_enemies) {
                    self.x = next.0;
                    self.y = next.1;
                }
            }
        } else {
            // Fallback to original tracking if path not found
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

fn render_hud(
    canvas: &mut Canvas<Window>,
    ttf_context: &Sdl2TtfContext,
    player: &Player,
    fps: f64
) {
    let font = ttf_context.load_font("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf", 24).unwrap();
    let texture_creator = canvas.texture_creator();
    let surface = font.render(&format!("Health: {}  Ammo: {}  FPS: {:.1}", player.health, player.ammo, fps))
        .blended(Color::YELLOW)
        .unwrap();
    let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
    canvas.copy(&texture, None, Rect::new(10, 10, 300, 30)).unwrap();
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

    let mut last_time = Instant::now();
    let mut frames = 0;
    let mut fps = 0.0;

    'running: loop {
        let frame_start = Instant::now();
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

        frames += 1;
        let now = Instant::now();
        let dt = now.duration_since(last_time).as_secs_f64();
        if dt >= 1.0 {
            fps = frames as f64 / dt;
            frames = 0;
            last_time = now;
        }

        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();

        // Draw the maze
        for (i, &(x, y, w, h)) in maze.iter().enumerate() {
            // The last 4 are the outer walls
            if i >= maze.len() - 4 {
                canvas.set_draw_color(Color::RGB(128, 0, 128)); // purple
            } else {
                canvas.set_draw_color(Color::RGB(255, 255, 255));
            }
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

        render_hud(&mut canvas, &ttf_context, &player, fps);

        canvas.present();

        // Lock to 60 FPS
        let frame_time = frame_start.elapsed();
        let target_frame_time = Duration::from_millis(16);
        if frame_time < target_frame_time {
            std::thread::sleep(target_frame_time - frame_time);
        }
    }
}