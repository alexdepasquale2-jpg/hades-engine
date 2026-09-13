use crate::types::QualityScalar;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PacketTemplate {
    pub id: String,
    pub title: String,
    pub situation: String,
    pub faction: String,
    pub start_shard: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReincarnationOffer {
    pub template_id: String,
    pub title: String,
    pub situation: String,
    pub faction: String,
    pub start_shard: u32,
    pub expected_delta_s: f32,
    pub score: f32,
    pub odds_label: String,
}

pub fn starter_pool() -> Vec<PacketTemplate> {
    vec![
        PacketTemplate {
            id: "pkt.merchant".into(),
            title: "The Debtor's Ledger".into(),
            situation: "You inherit a failing stall in the market quarter.".into(),
            faction: "traders".into(),
            start_shard: 0,
        },
        PacketTemplate {
            id: "pkt.watcher".into(),
            title: "Night Watch".into(),
            situation: "A border tower needs a soul willing to see what others ignore.".into(),
            faction: "wardens".into(),
            start_shard: 1,
        },
        PacketTemplate {
            id: "pkt.healer".into(),
            title: "Field Medic".into(),
            situation: "A clinic on the seam between shards asks for help without applause.".into(),
            faction: "healers".into(),
            start_shard: 0,
        },
        PacketTemplate {
            id: "pkt.hermit".into(),
            title: "Quiet Ridge".into(),
            situation: "A hermitage offers solitude and no audience.".into(),
            faction: "solitary".into(),
            start_shard: 1,
        },
        PacketTemplate {
            id: "pkt.scholar".into(),
            title: "Academy Audition".into(),
            situation: "NPMR faculty scouts PMR for one curious mind.".into(),
            faction: "academy".into(),
            start_shard: 0,
        },
        PacketTemplate {
            id: "pkt.guard".into(),
            title: "Strip Patrol".into(),
            situation: "The overlap strip between shards needs a walker.".into(),
            faction: "wardens".into(),
            start_shard: 1,
        },
    ]
}

/// Rank K=5 reincarnation offers (spec §11).
pub fn rank_offers(
    soul_quality: QualityScalar,
    _incarnation_count: u32,
    seen_templates: &[String],
) -> Vec<ReincarnationOffer> {
    let pool = starter_pool();
    let band = soul_quality.band().label();

    let mut offers: Vec<ReincarnationOffer> = pool
        .iter()
        .map(|t| {
            let base_delta: f32 = match band {
                "Turbulent" => -0.08,
                "Restless" => -0.05,
                "Settled" => -0.03,
                "Coherent" => -0.02,
                _ => -0.01,
            };
            let novelty = if seen_templates.contains(&t.id) {
                0.0
            } else {
                0.2
            };
            let stretch = base_delta.abs();
            let score = 0.5 * (-base_delta) + 0.3 * stretch + novelty;
            ReincarnationOffer {
                template_id: t.id.clone(),
                title: t.title.clone(),
                situation: t.situation.clone(),
                faction: t.faction.clone(),
                start_shard: t.start_shard,
                expected_delta_s: base_delta,
                score,
                odds_label: format!("~{}% toward quieter band", (stretch * 100.0) as i32),
            }
        })
        .collect();

    offers.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    offers.truncate(5);
    offers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_five_offers() {
        let offers = rank_offers(QualityScalar::INITIAL, 1, &[]);
        assert_eq!(offers.len(), 5);
    }
}
