//! Optional NATS JetStream integration test (requires `nats` feature + running NATS with -js).

#[cfg(feature = "nats")]
use tbc_engine::rww::{RwwBus, RwwConfig};

#[cfg(feature = "nats")]
#[test]
#[ignore = "requires NATS at TBC_NATS_URL"]
fn nats_publish_roundtrip() {
    let url = std::env::var("TBC_NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into());
    let bus = RwwBus::open(RwwConfig {
        nats_url: Some(url),
        stream_name: format!("TBC_RWW_TEST_{}", ulid::Ulid::new()),
    });
    assert_eq!(bus.status().backend, "nats");
    assert!(bus.status().connected);

    bus.publish("rww.test.nats", b"jetstream", 42, true);
    std::thread::sleep(std::time::Duration::from_millis(800));
    let msgs = bus.recent("rww.test.nats", 5);
    assert!(!msgs.is_empty());
}
