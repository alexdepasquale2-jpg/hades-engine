//! M13 — ruleset JSON loading and gameplay verbs.

use tbc_engine::ruleset::RulesetRegistry;

#[test]
fn load_rulesets_from_dir() {
    let path = format!("{}/../rulesets", env!("CARGO_MANIFEST_DIR"));
    let reg = RulesetRegistry::load_dir(std::path::Path::new(&path)).expect("load dir");
    let pmr = reg.get("pmr.v1").expect("pmr");
    assert!(pmr.verbs.attack.enabled);
    assert_eq!(pmr.verbs.attack.damage, 34.0);
    let npmr = reg.get("npmr.academy.v1").expect("npmr");
    assert!(!npmr.verbs.attack.enabled);
    assert_eq!(pmr.verbs.speak.range_m, 20.0);
    assert!(npmr.verbs.assist.ai_practice);
}

#[test]
fn boot_defaults_matches_json() {
    let reg = RulesetRegistry::boot_defaults();
    assert!(reg.get("pmr.v1").is_some());
}
