//! M9 — archive hydration across process restarts.

use tbc_engine::iuoc::IuocRegistry;
use tbc_engine::persist::SoulArchive;
use tbc_engine::types::{FrameId, FwauId, IuocId, QualityScalar, Tick};

#[test]
fn hydrate_after_restart() {
    let dir = std::env::temp_dir().join(format!("tbc-hydrate-{}", ulid::Ulid::new()));
    let path = dir.join("archive.db");

    let archive = SoulArchive::open(&path).expect("open");
    let mut reg = IuocRegistry::with_archive(archive).expect("hydrate empty");

    let iuoc = reg.create_soul();
    reg.bind_fwau(
        iuoc,
        FwauId(7),
        tbc_engine::iuoc::EntityRef {
            index: 1,
            generation: 0,
        },
        Tick(5),
    )
    .unwrap();
    reg.merge_fwau(
        FwauId(7),
        FrameId(2),
        Tick(50),
        "restart-test",
        QualityScalar::clamped(0.35),
    );

    let stats = reg.archive_stats().expect("stats");
    assert_eq!(stats.souls, 1);
    assert_eq!(stats.packets, 1);

    let archive2 = SoulArchive::open(&path).expect("reopen");
    let reg2 = IuocRegistry::with_archive(archive2).expect("rehydrate");
    let soul = reg2.get(iuoc).expect("soul");
    assert_eq!(soul.incarnations, 1);
    assert!(soul.bound_fwau.is_none());
    assert_eq!(reg2.packets_for(iuoc).len(), 1);
    assert_eq!(reg2.packets_for(iuoc)[0].death_cause, "restart-test");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resume_uses_archived_quality() {
    let dir = std::env::temp_dir().join(format!("tbc-resume-{}", ulid::Ulid::new()));
    let path = dir.join("archive.db");

    let archive = SoulArchive::open(&path).expect("open");
    let mut reg = IuocRegistry::with_archive(archive).expect("hydrate");
    let iuoc = reg.create_soul();
    if let Some(s) = reg.get_mut(iuoc) {
        s.quality = QualityScalar::clamped(0.25);
    }
    SoulArchive::open(&path)
        .expect("reopen")
        .upsert_soul(reg.get(iuoc).unwrap())
        .unwrap();

    let archive2 = SoulArchive::open(&path).expect("reopen read");
    let reg2 = IuocRegistry::with_archive(archive2).expect("rehydrate");
    let soul = reg2.get(iuoc).unwrap();
    assert!(soul.quality.value() < 0.3);

    std::fs::remove_dir_all(&dir).ok();
}
