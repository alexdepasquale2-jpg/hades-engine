//! M12 — health, readiness, and Prometheus metrics for production ops.

use crate::aum::AumCore;
use crate::guardrails::GuardrailReport;
use crate::islands::IslandProfiler;
use crate::persist::ArchiveStats;
use crate::rww::RwwStatus;
use crate::shard::ShardNodeStatus;
use serde::Serialize;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize)]
pub struct HealthCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct FrameOps {
    pub name: String,
    pub ruleset: String,
    pub tick: u64,
    pub entities: usize,
    pub fwau_count: usize,
    pub island_profiler: Option<IslandProfiler>,
    pub guardrails: Option<GuardrailReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OpsSnapshot {
    pub version: String,
    pub status: String,
    pub ready: bool,
    pub node: ShardNodeStatus,
    pub archive: Option<ArchiveStats>,
    pub rww: RwwStatus,
    pub active_sessions: usize,
    pub frames: Vec<FrameOps>,
    pub checks: Vec<HealthCheck>,
}

impl OpsSnapshot {
    pub fn from_aum(aum: &AumCore, active_sessions: usize) -> Self {
        let archive = aum.archive_stats();
        let rww = aum.rww.status();
        let node = aum.node_status();

        let frames: Vec<FrameOps> = aum
            .frames
            .iter()
            .map(|f| {
                let within_budget = f.island_mgr.profiler.within_budget;
                FrameOps {
                    name: f.spec.name.clone(),
                    ruleset: f.spec.ruleset.id.clone(),
                    tick: f.now().0,
                    entities: f.world.all_entities().len(),
                    fwau_count: f.intent_queues.len(),
                    island_profiler: Some(f.island_mgr.profiler.clone()),
                    guardrails: Some(f.guardrails.report(within_budget)),
                }
            })
            .collect();

        let mut checks = Vec::new();

        checks.push(HealthCheck {
            name: "frames_booted".into(),
            ok: !aum.frames.is_empty(),
            detail: format!("{} frame(s)", aum.frames.len()),
        });

        if let Some(stats) = &archive {
            checks.push(HealthCheck {
                name: "archive".into(),
                ok: true,
                detail: format!(
                    "{} souls, {} packets at {}",
                    stats.souls, stats.packets, stats.path
                ),
            });
        } else {
            checks.push(HealthCheck {
                name: "archive".into(),
                ok: true,
                detail: "in-memory souls (no archive path)".into(),
            });
        }

        let rww_ok = if rww.nats_url.is_some() {
            rww.connected
        } else {
            true
        };
        checks.push(HealthCheck {
            name: "rww".into(),
            ok: rww_ok,
            detail: if rww.nats_url.is_some() {
                format!("backend={} connected={}", rww.backend, rww.connected)
            } else {
                format!("backend={} (dev in-memory)", rww.backend)
            },
        });

        let guardrail_ok = frames.iter().all(|f| {
            f.guardrails
                .as_ref()
                .map(|g| g.within_budget)
                .unwrap_or(true)
        });
        checks.push(HealthCheck {
            name: "guardrails".into(),
            ok: guardrail_ok,
            detail: if guardrail_ok {
                "within beam budget".into()
            } else {
                "beam budget overrun detected".into()
            },
        });

        let ready = !aum.frames.is_empty()
            && rww_ok
            && checks.iter().filter(|c| c.name != "rww").all(|c| c.ok);
        let status = if ready && guardrail_ok {
            "ok"
        } else if ready {
            "degraded"
        } else {
            "unavailable"
        };

        Self {
            version: VERSION.into(),
            status: status.into(),
            ready,
            node,
            archive,
            rww,
            active_sessions,
            frames,
            checks,
        }
    }

    pub fn prometheus_lines(&self) -> String {
        let mut out = String::new();
        append_metric(
            &mut out,
            "tbc_info",
            &format!("version=\"{}\"", self.version),
            1.0,
        );
        append_metric(
            &mut out,
            "tbc_ready",
            "",
            if self.ready { 1.0 } else { 0.0 },
        );
        append_metric(
            &mut out,
            "tbc_sessions_active",
            "",
            self.active_sessions as f64,
        );
        append_metric(&mut out, "tbc_frames", "", self.frames.len() as f64);
        append_metric(
            &mut out,
            "tbc_rww_connected",
            "",
            if self.rww.connected { 1.0 } else { 0.0 },
        );

        if let Some(stats) = &self.archive {
            append_metric(&mut out, "tbc_archive_souls", "", stats.souls as f64);
            append_metric(&mut out, "tbc_archive_packets", "", stats.packets as f64);
        }

        for frame in &self.frames {
            let labels = format!(
                "frame=\"{}\",ruleset=\"{}\"",
                escape_label(&frame.name),
                escape_label(&frame.ruleset)
            );
            append_metric(&mut out, "tbc_tick", &labels, frame.tick as f64);
            append_metric(&mut out, "tbc_entities", &labels, frame.entities as f64);
            append_metric(&mut out, "tbc_fwau_count", &labels, frame.fwau_count as f64);

            if let Some(p) = &frame.island_profiler {
                append_metric(&mut out, "tbc_island_count", &labels, p.island_count as f64);
                append_metric(
                    &mut out,
                    "tbc_beam_steps_last_tick",
                    &labels,
                    p.total_steps_last_tick as f64,
                );
            }

            if let Some(g) = &frame.guardrails {
                append_metric(
                    &mut out,
                    "tbc_guardrail_stalls",
                    &labels,
                    g.stall_count as f64,
                );
                append_metric(
                    &mut out,
                    "tbc_guardrail_overruns",
                    &labels,
                    g.budget_overruns as f64,
                );
                append_metric(
                    &mut out,
                    "tbc_guardrail_rate_limited",
                    &labels,
                    g.rate_limited as f64,
                );
                append_metric(
                    &mut out,
                    "tbc_guardrail_throttled_ticks",
                    &labels,
                    g.throttled_ticks as f64,
                );
                append_metric(
                    &mut out,
                    "tbc_guardrail_within_budget",
                    &labels,
                    if g.within_budget { 1.0 } else { 0.0 },
                );
            }
        }

        out
    }
}

fn append_metric(out: &mut String, name: &str, labels: &str, value: f64) {
    if labels.is_empty() {
        out.push_str(&format!("{} {}\n", name, value));
    } else {
        out.push_str(&format!("{}{{{}}} {}\n", name, labels, value));
    }
}

fn escape_label(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aum::AumCore;

    #[test]
    fn ops_snapshot_cluster_ok() {
        let genesis = *blake3::hash(b"ops-test").as_bytes();
        let aum = AumCore::boot_cluster(genesis);
        let snap = OpsSnapshot::from_aum(&aum, 3);
        assert_eq!(snap.status, "ok");
        assert!(snap.ready);
        assert!(!snap.prometheus_lines().is_empty());
        assert!(snap.prometheus_lines().contains("tbc_tick"));
    }
}
