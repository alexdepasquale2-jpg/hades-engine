use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct LevelInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub ruleset_id: String,
    pub pack_dir: Option<PathBuf>,
    pub builtin_demo_npcs: usize,
}

#[derive(Debug, Deserialize)]
struct PackManifest {
    id: String,
    title: String,
    description: Option<String>,
    ruleset_id: String,
}

const BUILTIN: &[(&str, &str, &str, usize)] = &[
    ("builtin.pmr", "pmr.v1", "PMR-Prime sandbox (engine demo NPCs)", 12),
    (
        "builtin.npmr_academy",
        "npmr.academy.v1",
        "NPMR Academy sandbox",
        8,
    ),
    (
        "builtin.npmr_dream",
        "npmr.dream.v1",
        "NPMR Dream sandbox",
        8,
    ),
];

pub fn repo_root() -> PathBuf {
    if let Ok(p) = std::env::var("HADES_REPO_ROOT") {
        let path = PathBuf::from(p);
        if path.join("assets").join("packs").is_dir() {
            return path;
        }
    }
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..8 {
        if dir.join("assets").join("packs").is_dir() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    dir
}

pub fn discover_levels() -> Vec<LevelInfo> {
    let root = repo_root();
    let mut levels = Vec::new();

    for (id, ruleset, desc, npcs) in BUILTIN {
        levels.push(LevelInfo {
            id: id.to_string(),
            title: ruleset_title(&root, ruleset),
            description: desc.to_string(),
            ruleset_id: ruleset.to_string(),
            pack_dir: None,
            builtin_demo_npcs: *npcs,
        });
    }

    let packs_dir = root.join("assets").join("packs");
    if packs_dir.is_dir() {
        for entry in std::fs::read_dir(&packs_dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&manifest_path).unwrap_or_default();
            if let Ok(m) = serde_json::from_str::<PackManifest>(&text) {
                levels.push(LevelInfo {
                    id: m.id.clone(),
                    title: m.title.clone(),
                    description: m.description.unwrap_or_default(),
                    ruleset_id: m.ruleset_id.clone(),
                    pack_dir: Some(path),
                    builtin_demo_npcs: 0,
                });
            }
        }
    }

    levels.sort_by(|a, b| a.title.cmp(&b.title));
    levels
}

fn ruleset_title(root: &Path, id: &str) -> String {
    let path = root.join("rulesets").join(format!("{}.json", id));
    if let Ok(text) = std::fs::read_to_string(&path) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
                return t.to_string();
            }
        }
    }
    id.to_string()
}
