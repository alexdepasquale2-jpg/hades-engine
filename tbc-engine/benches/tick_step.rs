//! Release-profile smoke bench: `cargo bench -p tbc-engine --bench tick_step`

use std::time::Instant;
use tbc_engine::aum::AumCore;
use tbc_engine::types::Vec3;

fn main() {
    let genesis = *blake3::hash(b"bench-tick").as_bytes();
    let mut aum = AumCore::boot_cluster(genesis);
    aum.frames[0].spawn_demo_world(200);
    let iuoc = aum.iuoc.create_soul();
    let _fwau = aum
        .bind_player(0, iuoc, Vec3::new(-100.0, 0.0, 0.0))
        .unwrap();

    for _ in 0..50 {
        aum.frames[0].clock.tick.0 += 1;
        aum.run_frame_ticks(0, 1);
    }

    let start = Instant::now();
    let steps = 600u32;
    for _ in 0..steps {
        aum.frames[0].clock.tick.0 += 1;
        aum.run_frame_ticks(0, 1);
    }
    let elapsed = start.elapsed();
    let ms_per_tick = elapsed.as_secs_f64() * 1000.0 / steps as f64;
    println!(
        "600 ticks (200 AI + 1 player): {:.2} ms/tick ({:.1} tps)",
        ms_per_tick,
        1000.0 / ms_per_tick
    );
}
