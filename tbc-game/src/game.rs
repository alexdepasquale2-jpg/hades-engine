use crate::content::populate_pack;
use crate::levels::LevelInfo;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use tbc_engine::aum::{AumConfig, AumCore};
use tbc_engine::frame::{FrameSnapshot, FrameSpec};
use tbc_engine::intent::{Intent, Verb};
use tbc_engine::ruleset::{Ruleset, RulesetRegistry};
use tbc_engine::types::{FrameId, FwauId, Vec3};

const WORLD_SCALE: f32 = 6.0;

pub struct PlaySession {
    pub level: LevelInfo,
    pub aum: AumCore,
    pub fwau: FwauId,
    pub sprite_paths: HashMap<String, PathBuf>,
    pub textures: HashMap<String, Texture2D>,
    pub message: String,
    pub message_ttl: f32,
}

pub fn start_level(level: LevelInfo) -> Result<PlaySession, String> {
    let root = crate::levels::repo_root();
    let rulesets_path = root.join("rulesets");
    let reg = RulesetRegistry::load_dir(&rulesets_path).unwrap_or_else(|_| {
        RulesetRegistry::boot_defaults()
    });
    let ruleset = reg
        .get(&level.ruleset_id)
        .cloned()
        .unwrap_or_else(Ruleset::pmr_prime);

    let genesis = [7u8; 32];
    let mut aum = AumCore::boot(AumConfig {
        genesis_hash: genesis,
        ruleset,
    });

    let frame = aum.primary_frame();
    frame.spec = FrameSpec {
        id: FrameId(1),
        name: level.title.clone(),
        ruleset: frame.spec.ruleset.clone(),
        genesis_hash: genesis,
    };

    let mut sprite_paths = HashMap::new();
    let mut spawn_pos = Vec3::new(0.0, 0.0, 0.0);

    if let Some(pack_dir) = &level.pack_dir {
        let spawned = populate_pack(frame, pack_dir)?;
        if let Some(p) = spawned.player_spawn {
            spawn_pos = p;
        }
        sprite_paths = spawned.entity_sprite_paths;
    } else if level.builtin_demo_npcs > 0 {
        frame.spawn_demo_world(level.builtin_demo_npcs);
        spawn_pos = Vec3::new(-20.0, 0.0, 0.0);
    }

    let iuoc = aum.iuoc.create_soul();
    let fwau = aum.bind_player(0, iuoc, spawn_pos)?;

    Ok(PlaySession {
        level,
        aum,
        fwau,
        sprite_paths,
        textures: HashMap::new(),
        message: "WASD move · E interact · Space attack · Esc menu".into(),
        message_ttl: 6.0,
    })
}

impl PlaySession {
    pub async fn ensure_textures(&mut self) {
        for (id, path) in &self.sprite_paths {
            if self.textures.contains_key(id) {
                continue;
            }
            if path.is_file() {
                if let Ok(tex) = load_texture(&path.to_string_lossy()).await {
                    tex.set_filter(FilterMode::Nearest);
                    self.textures.insert(id.clone(), tex);
                }
            }
        }
    }

    pub fn tick(&mut self, dt: f32, move_dir: Vec2) {
        self.message_ttl -= dt;
        let frame_idx = 0;
        let tick = self.aum.frames[frame_idx].now();
        let seq = tick.0 as u32;

        if move_dir.length() > 0.01 {
            let n = move_dir.normalize();
            let payload = [n.x.to_le_bytes(), n.y.to_le_bytes()].concat();
            let intent = Intent {
                fwau: self.fwau,
                tick,
                seq,
                verb: Verb::Move,
                payload,
                consent: None,
                checksum: 0,
            };
            let _ = self.aum.submit_intent(frame_idx, intent);
        } else {
            let intent = Intent {
                fwau: self.fwau,
                tick,
                seq,
                verb: Verb::Move,
                payload: 0f32.to_le_bytes().into_iter().chain(0f32.to_le_bytes()).collect(),
                consent: None,
                checksum: 0,
            };
            let _ = self.aum.submit_intent(frame_idx, intent);
        }

        self.aum.run_frame_ticks(frame_idx, 1);
        self.aum.process_player_deaths(frame_idx);
    }

    pub fn interact(&mut self) {
        let res = self.aum.interact(0, self.fwau);
        self.message = res.message.clone();
        self.message_ttl = 4.0;
    }

    pub fn attack(&mut self) {
        let res = self.aum.attack(0, self.fwau, None);
        self.message = res.message.clone();
        self.message_ttl = 4.0;
    }

    pub fn snapshot(&mut self) -> FrameSnapshot {
        self.aum.frames[0].build_snapshot()
    }

    pub fn draw(&self, snap: &FrameSnapshot) {
        let sw = screen_width();
        let sh = screen_height();
        clear_background(Color::from_rgba(14, 18, 26, 255));

        let player = snap.entities.iter().find(|e| e.is_player);
        let cam = if let Some(p) = player {
            vec2(p.x, p.y)
        } else {
            vec2(0.0, 0.0)
        };

        draw_grid(cam, sw, sh);

        for ent in &snap.entities {
            let screen = world_to_screen(vec2(ent.x, ent.y), cam, sw, sh);
            let color = entity_color(ent);
            let size = if ent.is_player { 14.0 } else { 11.0 };

            draw_circle(screen.x, screen.y, size, color);
            if ent.is_player {
                draw_circle_lines(screen.x, screen.y, size + 3.0, 2.0, WHITE);
            }

            let label = if ent.name.len() > 14 {
                &ent.name[..14]
            } else {
                &ent.name
            };
            draw_text(label, screen.x + 12.0, screen.y - 6.0, 16.0, LIGHTGRAY);

            if let Some(hp) = ent.hp {
                let w = 36.0;
                let ratio = (hp / 120.0).clamp(0.0, 1.0);
                draw_rectangle(screen.x - w / 2.0, screen.y + 14.0, w, 4.0, Color::from_rgba(30, 30, 40, 200));
                draw_rectangle(
                    screen.x - w / 2.0,
                    screen.y + 14.0,
                    w * ratio,
                    4.0,
                    Color::from_rgba(80, 200, 120, 255),
                );
            }
        }

        draw_hud(self, snap, player);
    }
}

fn entity_color(ent: &tbc_engine::frame::EntitySnapshot) -> Color {
    if ent.is_player {
        return Color::from_rgba(100, 200, 255, 255);
    }
    match ent.name.to_lowercase() {
        n if n.contains("mentor") => Color::from_rgba(120, 160, 240, 255),
        n if n.contains("gate") => Color::from_rgba(180, 140, 90, 255),
        n if n.contains("shard") => Color::from_rgba(240, 200, 80, 255),
        _ if ent.is_ai => Color::from_rgba(200, 100, 100, 255),
        _ => Color::from_rgba(160, 160, 180, 255),
    }
}

fn world_to_screen(world: Vec2, cam: Vec2, sw: f32, sh: f32) -> Vec2 {
    let offset = world - cam;
    vec2(sw * 0.5 + offset.x * WORLD_SCALE, sh * 0.5 - offset.y * WORLD_SCALE)
}

fn draw_grid(cam: Vec2, sw: f32, sh: f32) {
    let step = 32.0;
    let world_step = step / WORLD_SCALE;
    let left = cam.x - sw / (2.0 * WORLD_SCALE);
    let right = cam.x + sw / (2.0 * WORLD_SCALE);
    let bottom = cam.y - sh / (2.0 * WORLD_SCALE);
    let top = cam.y + sh / (2.0 * WORLD_SCALE);
    let x0 = (left / world_step).floor() * world_step;
    let y0 = (bottom / world_step).floor() * world_step;
    let grid_color = Color::from_rgba(40, 50, 70, 120);
    let mut x = x0;
    while x <= right {
        let a = world_to_screen(vec2(x, bottom), cam, sw, sh);
        let b = world_to_screen(vec2(x, top), cam, sw, sh);
        draw_line(a.x, a.y, b.x, b.y, 1.0, grid_color);
        x += world_step;
    }
    let mut y = y0;
    while y <= top {
        let a = world_to_screen(vec2(left, y), cam, sw, sh);
        let b = world_to_screen(vec2(right, y), cam, sw, sh);
        draw_line(a.x, a.y, b.x, b.y, 1.0, grid_color);
        y += world_step;
    }
}

fn draw_hud(session: &PlaySession, snap: &FrameSnapshot, player: Option<&tbc_engine::frame::EntitySnapshot>) {
    draw_rectangle(0.0, 0.0, screen_width(), 56.0, Color::from_rgba(0, 0, 0, 140));
    draw_text(
        &format!("{} · {}", session.level.title, snap.ruleset_id),
        16.0,
        22.0,
        24.0,
        WHITE,
    );
    draw_text(
        &format!("tick {}", snap.tick),
        screen_width() - 120.0,
        22.0,
        20.0,
        LIGHTGRAY,
    );

    if let Some(p) = player {
        let hp = p.hp.unwrap_or(0.0);
        let st = p.stamina.unwrap_or(0.0);
        draw_text(
            &format!("HP {:.0}  STA {:.0}", hp, st),
            16.0,
            44.0,
            18.0,
            Color::from_rgba(180, 220, 180, 255),
        );
        if let Some(inv) = &p.inventory {
            if !inv.is_empty() {
                draw_text(
                    &format!("Inventory: {}", inv.join(", ")),
                    220.0,
                    44.0,
                    18.0,
                    YELLOW,
                );
            }
        }
    }

    if session.message_ttl > 0.0 {
        let alpha = (session.message_ttl.min(1.0) * 255.0) as u8;
        draw_rectangle(16.0, screen_height() - 72.0, screen_width() - 32.0, 48.0, Color::from_rgba(0, 0, 0, 160));
        draw_text(
            &session.message,
            28.0,
            screen_height() - 44.0,
            22.0,
            Color::from_rgba(230, 230, 240, alpha),
        );
    }

    draw_text(
        "Esc · level select",
        16.0,
        screen_height() - 16.0,
        16.0,
        DARKGRAY,
    );
}
