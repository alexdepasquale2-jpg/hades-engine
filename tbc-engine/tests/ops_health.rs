//! M12 — ops health and readiness snapshots

use tbc_engine::aum::AumCore;
use tbc_engine::persist::SoulArchive;

#[test]
fn health_ready_cluster() {
    let dir = std::env::temp_dir().join(format!("tbc-ops-{}", ulid::Ulid::new()));
    let path = dir.join("archive.db");
    let archive = SoulArchive::open(&path).expect("open");
    let genesis = *blake3::hash(b"ops-health").as_bytes();
    let aum = AumCore::boot_cluster_with_archive(genesis, archive).expect("boot");
    let snap = aum.ops_snapshot(2);
    assert!(snap.ready);
    assert_eq!(snap.status, "ok");
    assert!(snap
        .checks
        .iter()
        .any(|c| c.name == "frames_booted" && c.ok));
    assert!(snap.prometheus_lines().contains("tbc_ready"));
    std::fs::remove_dir_all(&dir).ok();
}
