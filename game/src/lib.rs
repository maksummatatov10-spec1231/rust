//! Forest Bloom — game logic compiled to WebAssembly.
//! No external crates: the bundled Rust toolchain is used offline.

use core::ptr::addr_of;

const WORLD_W: f32 = 960.0;
const WORLD_H: f32 = 600.0;
const MAX_ENTS: usize = 256;
const MAX_EVENTS: usize = 48;

const KIND_PLAYER: i32 = 1;
const KIND_ENEMY: i32 = 2;
const KIND_SHOT: i32 = 3;
const KIND_FLOWER: i32 = 4;
const KIND_MUSHROOM: i32 = 5;
const KIND_PARTICLE: i32 = 6;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entity {
    pub kind: i32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub r: f32,
    pub life: f32,
    pub data: f32,
}

impl Entity {
    const fn empty() -> Self {
        Self { kind: 0, x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, r: 0.0, life: 0.0, data: 0.0 }
    }
}

struct State {
    started: bool,
    over: bool,
    score: i32,
    lives: i32,
    level: i32,
    elapsed: f32,
    enemy_timer: f32,
    flower_timer: f32,
    mushroom_timer: f32,
    shoot_cd: f32,
    invuln: f32,
    shake: f32,
    kills: i32,
    facing_x: f32,
    facing_y: f32,
}

static mut ENTS: [Entity; MAX_ENTS] = [Entity::empty(); MAX_ENTS];
static mut NENTS: i32 = 0;
static mut EVENTS: [i32; MAX_EVENTS] = [0; MAX_EVENTS];
static mut NEVENTS: i32 = 0;
static mut SEED: u64 = 0x9e37_79b9_7f4a_7c15;
static mut ST: State = State {
    started: false, over: false, score: 0, lives: 3, level: 1,
    elapsed: 0.0, enemy_timer: 0.8, flower_timer: 0.4, mushroom_timer: 5.0,
    shoot_cd: 0.0, invuln: 0.0, shake: 0.0, kills: 0,
    facing_x: 1.0, facing_y: 0.0,
};

fn rnd() -> u32 {
    unsafe {
        SEED ^= SEED << 13;
        SEED ^= SEED >> 7;
        SEED ^= SEED << 17;
        SEED as u32
    }
}

fn rndf() -> f32 {
    ((rnd() & 0xffff) as f32) / 65535.0
}

fn event(id: i32) {
    unsafe {
        if NEVENTS < MAX_EVENTS as i32 {
            EVENTS[NEVENTS as usize] = id;
            NEVENTS += 1;
        }
    }
}

fn spawn(e: Entity) {
    unsafe {
        if NENTS < MAX_ENTS as i32 {
            ENTS[NENTS as usize] = e;
            NENTS += 1;
        }
    }
}

fn burst(x: f32, y: f32, color: f32, count: i32) {
    for _ in 0..count {
        let a = rndf() * 6.2831853;
        let s = 60.0 + rndf() * 180.0;
        spawn(Entity {
            kind: KIND_PARTICLE,
            x, y,
            vx: a.cos() * s,
            vy: a.sin() * s,
            r: 2.0 + rndf() * 3.0,
            life: 0.35 + rndf() * 0.4,
            data: color,
        });
    }
}

fn reset() {
    unsafe {
        NENTS = 0;
        NEVENTS = 0;
        SEED = 0x9e37_79b9_7f4a_7c15 ^ (rnd() as u64);
        ST.started = true;
        ST.over = false;
        ST.score = 0;
        ST.lives = 3;
        ST.level = 1;
        ST.elapsed = 0.0;
        ST.enemy_timer = 1.0;
        ST.flower_timer = 0.3;
        ST.mushroom_timer = 6.0;
        ST.shoot_cd = 0.0;
        ST.invuln = 1.0;
        ST.shake = 0.0;
        ST.kills = 0;
        ST.facing_x = 1.0;
        ST.facing_y = 0.0;
        ENTS[0] = Entity {
            kind: KIND_PLAYER,
            x: WORLD_W * 0.5,
            y: WORLD_H * 0.5,
            vx: 0.0,
            vy: 0.0,
            r: 17.0,
            life: 1.0,
            data: 0.0,
        };
        NENTS = 1;
    }
}

#[no_mangle]
pub extern "C" fn init_game() {
    unsafe {
        ST.started = false;
        ST.over = false;
        reset();
        ST.started = false;
    }
}

#[no_mangle]
pub extern "C" fn start_game() {
    reset();
    event(0);
}

#[no_mangle]
pub extern "C" fn is_over() -> i32 {
    unsafe { if ST.over { 1 } else { 0 } }
}

#[no_mangle]
pub extern "C" fn entities_ptr() -> *const Entity {
    addr_of!(ENTS).cast::<Entity>()
}

#[no_mangle]
pub extern "C" fn entity_count() -> i32 {
    unsafe { NENTS }
}

#[no_mangle]
pub extern "C" fn events_ptr() -> *const i32 {
    addr_of!(EVENTS).cast::<i32>()
}

#[no_mangle]
pub extern "C" fn event_count() -> i32 {
    unsafe { NEVENTS }
}

#[no_mangle]
pub extern "C" fn clear_events() {
    unsafe { NEVENTS = 0 }
}

/// state codes: 0 score, 1 lives, 2 level, 3 elapsed seconds, 4 kills,
/// 5 shake, 6 invulnerability, 7 started
#[no_mangle]
pub extern "C" fn state(code: i32) -> i32 {
    unsafe {
        match code {
            0 => ST.score,
            1 => ST.lives,
            2 => ST.level,
            3 => ST.elapsed as i32,
            4 => ST.kills,
            5 => (ST.shake * 1000.0) as i32,
            6 => (ST.invuln * 1000.0) as i32,
            7 => if ST.started { 1 } else { 0 },
            _ => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn step(dt: f32, input: i32, pointer_x: f32, pointer_y: f32) {
    let dt = dt.clamp(0.0, 0.05);
    unsafe {
        if !ST.started || ST.over || dt <= 0.0 {
            return;
        }

        ST.elapsed += dt;
        ST.level = 1 + (ST.elapsed / 22.0) as i32;
        if ST.invuln > 0.0 { ST.invuln -= dt; }
        if ST.shake > 0.0 { ST.shake -= dt; }
        if ST.shoot_cd > 0.0 { ST.shoot_cd -= dt; }

        // Player movement (bit flags: 1 up, 2 down, 4 left, 8 right, 16 shoot).
        let px: f32 = if input & 8 != 0 { 1.0 } else if input & 4 != 0 { -1.0 } else { 0.0 };
        let py: f32 = if input & 2 != 0 { 1.0 } else if input & 1 != 0 { -1.0 } else { 0.0 };
        let len = (px * px + py * py).sqrt().max(1.0);
        let speed = 215.0;
        let p = &mut ENTS[0];
        p.vx = p.vx * 0.78 + (px / len * speed) * 0.22;
        p.vy = p.vy * 0.78 + (py / len * speed) * 0.22;
        p.x = (p.x + p.vx * dt).clamp(p.r, WORLD_W - p.r);
        p.y = (p.y + p.vy * dt).clamp(p.r, WORLD_H - p.r);

        // Aim and shoot toward the cursor.
        let mut dx = pointer_x - p.x;
        let mut dy = pointer_y - p.y;
        let d = (dx * dx + dy * dy).sqrt();
        if d > 1.0 {
            dx /= d;
            dy /= d;
            ST.facing_x = dx;
            ST.facing_y = dy;
        } else {
            dx = ST.facing_x;
            dy = ST.facing_y;
        }
        if input & 16 != 0 && ST.shoot_cd <= 0.0 {
            spawn(Entity {
                kind: KIND_SHOT,
                x: p.x + dx * 24.0,
                y: p.y + dy * 24.0,
                vx: dx * 520.0,
                vy: dy * 520.0,
                r: 7.0,
                life: 1.1,
                data: rndf(),
            });
            ST.shoot_cd = 0.16;
            event(2);
        }

        // Spawners.
        ST.enemy_timer -= dt;
        if ST.enemy_timer <= 0.0 {
            let edge = rnd() % 4;
            let (x, y) = match edge {
                0 => (rndf() * WORLD_W, -18.0),
                1 => (WORLD_W + 18.0, rndf() * WORLD_H),
                2 => (rndf() * WORLD_W, WORLD_H + 18.0),
                _ => (-18.0, rndf() * WORLD_H),
            };
            spawn(Entity {
                kind: KIND_ENEMY,
                x, y,
                vx: 0.0,
                vy: 0.0,
                r: 16.0 + (ST.level.min(8) as f32),
                life: 14.0,
                data: rndf() * 6.2831853,
            });
            ST.enemy_timer = (1.35 - (ST.level as f32) * 0.07).max(0.35);
        }
        ST.flower_timer -= dt;
        if ST.flower_timer <= 0.0 {
            spawn(Entity {
                kind: KIND_FLOWER,
                x: 40.0 + rndf() * (WORLD_W - 80.0),
                y: 40.0 + rndf() * (WORLD_H - 80.0),
                vx: 0.0,
                vy: 0.0,
                r: 15.0,
                life: 10.0,
                data: (rnd() % 5) as f32,
            });
            ST.flower_timer = 2.2 + rndf() * 1.8;
        }
        ST.mushroom_timer -= dt;
        if ST.mushroom_timer <= 0.0 {
            spawn(Entity {
                kind: KIND_MUSHROOM,
                x: 60.0 + rndf() * (WORLD_W - 120.0),
                y: 60.0 + rndf() * (WORLD_H - 120.0),
                vx: 0.0,
                vy: 0.0,
                r: 19.0,
                life: 9.0,
                data: 0.0,
            });
            ST.mushroom_timer = 11.0 + rndf() * 5.0;
        }

        // Update all non-player entities.
        let n = NENTS;
        for i in 1..n as usize {
            let e = &mut ENTS[i];
            if e.life <= 0.0 { continue; }
            match e.kind {
                KIND_ENEMY => {
                    let ex = p.x - e.x;
                    let ey = p.y - e.y;
                    let el = (ex * ex + ey * ey).sqrt().max(1.0);
                    let spd = 52.0 + ST.level as f32 * 9.0;
                    e.vx = ex / el * spd;
                    e.vy = ey / el * spd;
                    e.x += e.vx * dt;
                    e.y += e.vy * dt;
                    e.data += dt * 3.0;
                    let hit = (e.x - p.x).powi(2) + (e.y - p.y).powi(2);
                    if hit < (e.r + p.r) as f32 * (e.r + p.r) as f32 && ST.invuln <= 0.0 {
                        ST.lives -= 1;
                        ST.invuln = 1.4;
                        ST.shake = 0.35;
                        e.life = 0.0;
                        burst(p.x, p.y, 1.0, 12);
                        event(4);
                        if ST.lives <= 0 {
                            ST.over = true;
                            event(5);
                        }
                    }
                }
                KIND_SHOT => {
                    e.x += e.vx * dt;
                    e.y += e.vy * dt;
                    e.life -= dt;
                    if e.x < -20.0 || e.x > WORLD_W + 20.0 || e.y < -20.0 || e.y > WORLD_H + 20.0 {
                        e.life = 0.0;
                    }
                }
                KIND_FLOWER | KIND_MUSHROOM => {
                    e.life -= dt;
                    e.data += dt;
                    let hit = (e.x - p.x).powi(2) + (e.y - p.y).powi(2);
                    if hit < (e.r + p.r + 6.0) as f32 * (e.r + p.r + 6.0) as f32 {
                        e.life = 0.0;
                        if e.kind == KIND_FLOWER {
                            ST.score += 25 + ST.level * 5;
                            burst(e.x, e.y, 2.0, 10);
                            event(1);
                        } else {
                            ST.score += 100;
                            ST.lives = (ST.lives + 1).min(5);
                            burst(e.x, e.y, 3.0, 18);
                            event(6);
                        }
                    }
                }
                KIND_PARTICLE => {
                    e.x += e.vx * dt;
                    e.y += e.vy * dt;
                    e.vx *= 0.94;
                    e.vy *= 0.94;
                    e.life -= dt * 1.3;
                }
                _ => {}
            }
        }

        // Projectile → enemy hits.
        let n = NENTS;
        for i in 1..n as usize {
            if ENTS[i].kind != KIND_SHOT || ENTS[i].life <= 0.0 { continue; }
            for j in 1..n as usize {
                if ENTS[j].kind != KIND_ENEMY || ENTS[j].life <= 0.0 { continue; }
                let a = &ENTS[i];
                let b = &ENTS[j];
                let rr = a.r + b.r;
                if (a.x - b.x).powi(2) + (a.y - b.y).powi(2) < rr * rr {
                    let ax = a.x;
                    let ay = a.y;
                    ENTS[i].life = 0.0;
                    ENTS[j].life = 0.0;
                    ST.score += 10 * ST.level;
                    ST.kills += 1;
                    ST.shake = 0.12;
                    burst(ax, ay, 0.0, 12);
                    event(3);
                    break;
                }
            }
        }

        // Compact dead entities, keeping slot 0 for the player.
        let mut i = 1usize;
        while i < NENTS as usize {
            if ENTS[i].life <= 0.0 {
                let last = (NENTS - 1) as usize;
                if i != last {
                    ENTS[i] = ENTS[last];
                }
                NENTS -= 1;
            } else {
                i += 1;
            }
        }
    }
}
