//! M11 — distributed PMR shard nodes share handoffs via RWW.

use std::sync::Arc;
use tbc_engine::aum::AumCore;
use tbc_engine::persist::SoulArchive;
use tbc_engine::rww::{MemoryStore, RwwConfig, RwwBus};
use tbc_engine::shard::ShardNodeConfig;
use tbc_engine::types::Vec3;

#[test]
fn distributed_shard_cross_via_shared_rww() {
  let dir = std::env::temp_dir().join(format!("tbc-shard-{}", ulid::Ulid::new()));
  let path = dir.join("archive.db");

  let store = Arc::new(MemoryStore::new());
  let config = RwwConfig {
    nats_url: None,
    stream_name: "TBC_RWW".into(),
  };
  let rww0 = RwwBus::with_store(config.clone(), store.clone());
  let rww1 = RwwBus::with_store(config, store);

  let genesis = *blake3::hash(b"TBC-GENESIS-M11").as_bytes();

  let mut node0 = AumCore::boot_node_with_archive_rww(
    genesis,
    SoulArchive::open(&path).expect("open archive"),
    ShardNodeConfig::single(0),
    rww0,
  )
  .expect("boot shard-00");

  let mut node1 = AumCore::boot_node_with_archive_rww(
    genesis,
    SoulArchive::open(&path).expect("reopen archive"),
    ShardNodeConfig::single(1),
    rww1,
  )
  .expect("boot shard-01");

  node0.frames[0].spawn_demo_world(4);
  node1.frames[0].spawn_demo_world(4);

  let iuoc = node0.iuoc.create_soul();
  let fwau = node0
    .bind_player(0, iuoc, Vec3::new(40.0, 0.0, 0.0))
    .expect("bind west");

  for _ in 0..4 {
    node0.run_frame_ticks(0, 1);
    node0.process_shard_handoffs();
  }

  assert!(
    node0.iuoc.get(iuoc).unwrap().bound_fwau.is_none(),
    "source shard should unbind after cross"
  );
  assert!(!node0.rww.recent("rww.shard.cross", 4).is_empty());

  let inbound = node1.process_inbound_shard_crosses();
  assert_eq!(inbound.len(), 1);
  assert_eq!(inbound[0].1, iuoc);
  assert_ne!(inbound[0].0, fwau);
  assert!(node1.iuoc.get(iuoc).unwrap().bound_fwau.is_some());

  std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cluster_mode_handoff_stays_in_process() {
  let genesis = *blake3::hash(b"TBC-GENESIS-M11-CLUSTER").as_bytes();
  let mut cluster = AumCore::boot_cluster(genesis);
  cluster.frames[0].spawn_demo_world(4);
  cluster.frames[1].spawn_demo_world(4);

  let iuoc = cluster.iuoc.create_soul();
  let fwau = cluster
    .bind_player(0, iuoc, Vec3::new(40.0, 0.0, 0.0))
    .expect("bind");

  for _ in 0..4 {
    cluster.run_frame_ticks(0, 1);
    cluster.process_shard_handoffs();
  }

  let soul = cluster.iuoc.get(iuoc).unwrap();
  assert!(soul.bound_fwau.is_some());
  assert_ne!(soul.bound_fwau.unwrap(), fwau);
}
