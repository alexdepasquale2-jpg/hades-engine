use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tbc_engine::ecs::{Brain, BrainKind};
use tbc_engine::frame::Frame;
use tbc_engine::types::Vec3;

#[derive(Debug, Deserialize)]
struct PackManifest {
    entities: Vec<String>,
    placements: Option<Vec<Placement>>,
}

#[derive(Debug, Deserialize)]
struct Placement {
    entity: String,
    x: f32,
    y: f32,
    z: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct EntityDef {
    pub id: String,
    pub display_name: String,
    pub kind: String,
    pub interactable: Option<bool>,
    pub transform_default: Option<TransformDef>,
    pub avatar: Option<AvatarDef>,
    pub brain: Option<BrainDef>,
    pub sprite: Option<SpriteDef>,
}

#[derive(Debug, Deserialize)]
struct TransformDef {
    position: Pos,
    rotation: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct Pos {
    x: f32,
    y: f32,
    z: f32,
}

#[derive(Debug, Deserialize)]
struct AvatarDef {
    hp: f32,
    stamina: f32,
}

#[derive(Debug, Deserialize)]
struct BrainDef {
    kind: String,
    seed: u64,
}

#[derive(Debug, Deserialize)]
pub struct SpriteDef {
    pub path: String,
}

pub struct SpawnResult {
    pub player_spawn: Option<Vec3>,
    pub entity_sprite_paths: HashMap<String, PathBuf>,
}

pub fn populate_pack(frame: &mut Frame, pack_dir: &Path) -> Result<SpawnResult, String> {
    let manifest_path = pack_dir.join("manifest.json");
    let text = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("read manifest: {}", e))?;
    let manifest: PackManifest =
        serde_json::from_str(&text).map_err(|e| format!("parse manifest: {}", e))?;

    let placement_map: HashMap<String, Pos> = manifest
        .placements
        .unwrap_or_default()
        .into_iter()
        .map(|p| {
            (
                p.entity.replace('\\', "/"),
                Pos {
                    x: p.x,
                    y: p.y,
                    z: p.z.unwrap_or(0.0),
                },
            )
        })
        .collect();

    let mut player_spawn = None;
    let mut sprites = HashMap::new();

    for rel in &manifest.entities {
        let rel_norm = rel.replace('\\', "/");
        let ent_path = pack_dir.join(&rel_norm);
        let ent_text = std::fs::read_to_string(&ent_path)
            .map_err(|e| format!("read {}: {}", ent_path.display(), e))?;
        let def: EntityDef =
            serde_json::from_str(&ent_text).map_err(|e| format!("parse {}: {}", rel_norm, e))?;

        let base = def
            .transform_default
            .as_ref()
            .map(|t| t.position.clone())
            .unwrap_or(Pos {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        let pos = placement_map.get(&rel_norm).cloned().unwrap_or(base);
        let position = Vec3::new(pos.x, pos.y, pos.z);

        if def.kind == "player_spawn" {
            player_spawn = Some(position);
            continue;
        }

        let name = def.display_name.clone();
        let entity = frame.world.spawn(&name);
        if let Some(rec) = frame.world.get_mut(entity) {
            rec.transform.position = position;
            if let Some(rot) = def.transform_default.as_ref().and_then(|t| t.rotation) {
                rec.transform.rotation = rot;
            }
            if let Some(av) = &def.avatar {
                rec.avatar = Some(tbc_engine::ecs::AvatarState {
                    hp: av.hp,
                    stamina: av.stamina,
                    dead: false,
                    inventory: Vec::new(),
                });
            }
            let needs_brain = def.kind == "npc"
                || (def.interactable.unwrap_or(false) && def.kind != "pickup");
            if needs_brain {
                let kind = def
                    .brain
                    .as_ref()
                    .map(|b| b.kind.as_str())
                    .unwrap_or("Utility");
                let seed = def.brain.as_ref().map(|b| b.seed).unwrap_or(1);
                rec.brain = Some(Brain {
                    kind: match kind {
                        "BehaviorTree" => BrainKind::BehaviorTree,
                        "Ml" => BrainKind::Ml,
                        _ => BrainKind::Utility,
                    },
                    seed,
                });
            }
            rec.sleep.awake = true;
            rec.dirty = true;
        }
        frame.grid.insert_entity(entity, position, 1);

        if let Some(sp) = &def.sprite {
            sprites.insert(def.id.clone(), pack_dir.join(&sp.path));
        }
    }

    Ok(SpawnResult {
        player_spawn,
        entity_sprite_paths: sprites,
    })
}
