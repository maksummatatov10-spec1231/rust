//! Forest Bloom 3D — first-person sword-combat core.
//!
//! Rust owns all gameplay state: player movement, stamina, sword hit detection,
//! enemy AI, waves, pickups and scoring. The browser only renders the memory
//! exposed through `player_ptr()` / `foes_ptr()` and forwards raw input.
//!
//! No external crates: only `std` and the offline wasm32 target.

use core::ptr::addr_of;

const MAX_FOES: usize = 96;
const MAX_EVENTS: usize = 64;
const EYE: f32 = 1.7;
const GRAVITY: f32 = 16.0;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Player {
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub vx: f32,
    pub vz: f32,
    pub vy: f32,
    pub health: f32,
    pub stamina: f32,
    pub attack: f32,
    pub combo: f32,
}

impl Player {
    const fn new() -> Self {
        Self {
            x: 0.0, z: 0.0, y: EYE, yaw: 0.0, pitch: 0.0,
            vx: 0.0, vz: 0.0, vy: 0.0,
            health: 100.0, stamina: 100.0, attack: 0.0, combo: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Foe {
    pub kind: i32,
    pub x: f32,
    pub z: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub phase: f32,
    pub flash: f32,
    pub yaw: f32,
}

impl Foe {
    const fn empty() -> Self {
        Self { kind: 0, x: 0.0, z: 0.0, y: 0.0, hp: 0.0, max_hp: 0.0, phase: 0.0, flash: 0.0, yaw: 0.0 }
    }
}

struct Game {
    mode: i32,             // 0 menu, 1 playing, 2 paused, 3 game over
    score: i32,
    wave: i32,
    kills: i32,
    time: f32,
    spawn: f32,
    attack_cd: f32,
    invuln: f32,
    sensitivity: f32,
    shake: f32,
    hit_marker: f32,
    damage_flash: f32,
    picked: i32,
}

static mut P: Player = Player::new();
static mut FOES: [Foe; MAX_FOES] = [Foe::empty(); MAX_FOES];
static mut NFOES: i32 = 0;
static mut EVENTS: [i32; MAX_EVENTS] = [0; MAX_EVENTS];
static mut NEVENTS: i32 = 0;
static mut SEED: u64 = 0x2545_f491_4f6c_dd1d;
static mut G: Game = Game {
    mode: 0, score: 0, wave: 1, kills: 0, time: 0.0, spawn: 1.2,
    attack_cd: 0.0, invuln: 0.0, sensitivity: 0.0022, shake: 0.0,
    hit_marker: 0.0, damage_flash: 0.0, picked: 0,
};

fn rnd() -> u32 {
    unsafe {
        SEED ^= SEED << 13;
        SEED ^= SEED >> 7;
        SEED ^= SEED << 17;
        SEED as u32
    }
}
fn rndf() -> f32 { ((rnd() & 0xffff) as f32) / 65535.0 }

fn event(id: i32) {
    unsafe {
        if NEVENTS < MAX_EVENTS as i32 {
            EVENTS[NEVENTS as usize] = id;
            NEVENTS += 1;
        }
    }
}

fn add_foe(kind: i32, x: f32, z: f32) {
    let hp = match kind { 1 => 45.0, 2 => 130.0, _ => 70.0 };
    unsafe {
        if NFOES < MAX_FOES as i32 {
            FOES[NFOES as usize] = Foe {
                kind, x, z, y: 0.0, hp, max_hp: hp,
                phase: rndf() * 6.283, flash: 0.0, yaw: 0.0,
            };
            NFOES += 1;
        }
    }
}

#[no_mangle]
pub extern "C" fn init_game() {
    unsafe {
        P = Player::new();
        NFOES = 0;
        NEVENTS = 0;
        G.mode = 0;
        G.score = 0;
        G.wave = 1;
        G.kills = 0;
        G.time = 0.0;
        G.spawn = 1.0;
        G.attack_cd = 0.0;
        G.invuln = 0.0;
        G.shake = 0.0;
        G.hit_marker = 0.0;
        G.damage_flash = 0.0;
        G.picked = 0;
    }
}

#[no_mangle]
pub extern "C" fn reset_game() {
    unsafe {
        P = Player::new();
        NFOES = 0;
        NEVENTS = 0;
        G.mode = 1;
        G.score = 0;
        G.wave = 1;
        G.kills = 0;
        G.time = 0.0;
        G.spawn = 1.0;
        G.attack_cd = 0.0;
        G.invuln = 1.0;
        G.shake = 0.0;
        G.hit_marker = 0.0;
        G.damage_flash = 0.0;
        G.picked = 0;
        SEED ^= rnd() as u64;
        event(0);
    }
}

#[no_mangle]
pub extern "C" fn set_mode(mode: i32) {
    unsafe {
        G.mode = mode;
    }
}

#[no_mangle]
pub extern "C" fn set_sensitivity(v: f32) {
    unsafe { G.sensitivity = v.clamp(0.0004, 0.012); }
}

#[no_mangle]
pub extern "C" fn mode() -> i32 { unsafe { G.mode } }

/// 0 health, 1 stamina, 2 score, 3 wave, 4 kills, 5 time,
/// 6 attack, 7 combo, 8 shake, 9 hit marker, 10 damage flash,
/// 11 invuln, 12 foes alive, 13 picked, 14 max health
#[no_mangle]
pub extern "C" fn state(code: i32) -> f32 {
    unsafe {
        match code {
            0 => P.health,
            1 => P.stamina,
            2 => G.score as f32,
            3 => G.wave as f32,
            4 => G.kills as f32,
            5 => G.time,
            6 => P.attack,
            7 => P.combo,
            8 => G.shake,
            9 => G.hit_marker,
            10 => G.damage_flash,
            11 => G.invuln,
            12 => NFOES as f32,
            13 => G.picked as f32,
            14 => 100.0,
            _ => 0.0,
        }
    }
}

#[no_mangle]
pub extern "C" fn player_ptr() -> *const Player { addr_of!(P).cast::<Player>() }
#[no_mangle]
pub extern "C" fn foes_ptr() -> *const Foe { addr_of!(FOES).cast::<Foe>() }
#[no_mangle]
pub extern "C" fn foes_count() -> i32 { unsafe { NFOES } }
#[no_mangle]
pub extern "C" fn events_ptr() -> *const i32 { addr_of!(EVENTS).cast::<i32>() }
#[no_mangle]
pub extern "C" fn event_count() -> i32 { unsafe { NEVENTS } }
#[no_mangle]
pub extern "C" fn clear_events() { unsafe { NEVENTS = 0 } }

/// input bits: 1 forward, 2 back, 4 left, 8 right, 16 sprint, 32 attack, 64 jump
#[no_mangle]
pub extern "C" fn step(dt: f32, input: i32, mdx: f32, mdy: f32) {
    let dt = dt.clamp(0.0, 0.05);
    unsafe {
        if G.mode != 1 || dt <= 0.0 { return; }

        // Look.
        P.yaw -= mdx * G.sensitivity;
        P.pitch = (P.pitch - mdy * G.sensitivity).clamp(-1.42, 1.42);
        if P.yaw > 6.2831853 { P.yaw -= 6.2831853; }
        if P.yaw < 0.0 { P.yaw += 6.2831853; }

        G.time += dt;
        G.wave = 1 + (G.time / 32.0) as i32;
        if G.invuln > 0.0 { G.invuln -= dt; }
        if G.attack_cd > 0.0 { G.attack_cd -= dt; }
        if G.shake > 0.0 { G.shake -= dt; }
        if G.hit_marker > 0.0 { G.hit_marker -= dt; }
        if G.damage_flash > 0.0 { G.damage_flash -= dt; }
        if P.attack > 0.0 { P.attack -= dt * 2.1; }
        if P.combo > 0.0 { P.combo -= dt * 0.55; }

        // Movement in camera-relative plane.
        let fw: f32 = if input & 1 != 0 { 1.0 } else if input & 2 != 0 { -1.0 } else { 0.0 };
        let st: f32 = if input & 8 != 0 { 1.0 } else if input & 4 != 0 { -1.0 } else { 0.0 };
        let sin_y = P.yaw.sin();
        let cos_y = P.yaw.cos();
        let mut dx = -sin_y * fw + cos_y * st;
        let mut dz = -cos_y * fw - sin_y * st;
        let n = (dx * dx + dz * dz).sqrt().max(1.0);
        dx /= n; dz /= n;

        let sprinting = input & 16 != 0 && P.stamina > 5.0 && fw > 0.0;
        let speed = if sprinting { 7.4 } else { 4.3 };
        if sprinting { P.stamina = (P.stamina - 20.0 * dt).max(0.0); }
        else { P.stamina = (P.stamina + 14.0 * dt).min(100.0); }
        P.vx = P.vx * 0.82 + dx * speed * 0.18;
        P.vz = P.vz * 0.82 + dz * speed * 0.18;
        P.x += P.vx * dt;
        P.z += P.vz * dt;
        let limit = 46.0;
        if P.x < -limit { P.x = -limit; }
        if P.x > limit { P.x = limit; }
        if P.z < -limit { P.z = -limit; }
        if P.z > limit { P.z = limit; }

        // Jump / gravity.
        if input & 64 != 0 && P.y <= EYE + 0.01 {
            P.vy = 5.4;
        }
        P.vy -= GRAVITY * dt;
        P.y += P.vy * dt;
        if P.y < EYE { P.y = EYE; P.vy = 0.0; }

        // Sword swing.
        if input & 32 != 0 && G.attack_cd <= 0.0 && P.stamina >= 18.0 {
            P.stamina -= 18.0;
            P.attack = 1.0;
            G.attack_cd = 0.52;
            event(2);
            let reach = 3.3;
            let fx = -sin_y;
            let fz = -cos_y;
            for j in 0..NFOES as usize {
                let f = &mut FOES[j];
                let ex = f.x - P.x;
                let ez = f.z - P.z;
                let d = (ex * ex + ez * ez).sqrt();
                if d > reach || d < 0.001 { continue; }
                let dot = (ex / d) * fx + (ez / d) * fz;
                if dot < 0.42 { continue; }
                let dmg = 34.0 + P.combo.min(6.0) * 5.0 + if f.kind == 2 { -8.0 } else { 0.0 };
                f.hp -= dmg;
                f.flash = 0.28;
                f.x += fx * 0.6;
                f.z += fz * 0.6;
                G.score += 15;
                G.hit_marker = 0.22;
                G.shake = 0.10;
                P.combo = (P.combo + 1.0).min(8.0);
                event(3);
                if f.hp <= 0.0 {
                    let kind = f.kind;
                    FOES[j] = FOES[(NFOES - 1) as usize];
                    NFOES -= 1;
                    G.kills += 1;
                    G.score += if kind == 2 { 220 } else { 110 };
                    event(4);
                    if G.kills % 8 == 0 && NFOES < MAX_FOES as i32 {
                        FOES[NFOES as usize] = Foe {
                            kind: 3,
                            x: P.x + (rndf() - 0.5) * 10.0,
                            z: P.z + (rndf() - 0.5) * 10.0,
                            y: 0.55,
                            hp: 1.0, max_hp: 1.0, phase: 0.0, flash: 0.0, yaw: 0.0,
                        };
                        NFOES += 1;
                    }
                }
            }
        }

        // Spawner.
        G.spawn -= dt;
        if G.spawn <= 0.0 {
            let cap = (10 + G.wave * 2).min(MAX_FOES as i32 - 4);
            if NFOES < cap {
                let a = rndf() * 6.2831853;
                let dist = 17.0 + rndf() * 10.0;
                let r = rnd();
                let kind = if r % 11 == 0 { 2 } else if r % 3 == 0 { 1 } else { 0 };
                add_foe(kind, P.x + a.cos() * dist, P.z + a.sin() * dist);
            }
            G.spawn = (2.4 - G.wave as f32 * 0.12).max(0.55);
        }

        // Foes.
        for i in 0..NFOES as usize {
            let f = &mut FOES[i];
            f.phase += dt;
            if f.flash > 0.0 { f.flash -= dt; }
            if f.kind == 3 {
                f.y = 0.55 + (f.phase * 2.0).sin() * 0.16;
                let d2 = (f.x - P.x).powi(2) + (f.z - P.z).powi(2);
                if d2 < 3.0 {
                    P.health = (P.health + 28.0).min(100.0);
                    G.score += 75;
                    G.picked += 1;
                    event(6);
                    FOES[i] = FOES[(NFOES - 1) as usize];
                    NFOES -= 1;
                    return;
                }
                continue;
            }
            let ex = P.x - f.x;
            let ez = P.z - f.z;
            let d = (ex * ex + ez * ez).sqrt().max(0.001);
            f.yaw = ez.atan2(ex);
            let speed = match f.kind { 1 => 3.1, 2 => 1.35, _ => 2.0 };
            let wobble = (f.phase * 1.7).sin() * 0.35;
            if d > 1.5 {
                f.x += (ex / d + wobble * -ez / d) * speed * dt;
                f.z += (ez / d + wobble * ex / d) * speed * dt;
            } else if G.invuln <= 0.0 {
                let dmg = match f.kind { 1 => 6.0, 2 => 15.0, _ => 9.0 };
                P.health -= dmg;
                G.invuln = 0.95;
                G.damage_flash = 0.5;
                G.shake = 0.28;
                P.combo = 0.0;
                event(5);
                if P.health <= 0.0 {
                    P.health = 0.0;
                    G.mode = 3;
                    event(7);
                    return;
                }
            }
        }
    }
}
