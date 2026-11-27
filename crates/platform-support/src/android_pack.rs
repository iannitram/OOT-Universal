// SPDX-License-Identifier: MIT
// Adapted from BananaNative's PackPolicy.java at
// ba6094da4407247cba31e546ea7d51a747a11a0a.
// See licenses/banananative-MIT.txt for the upstream copyright and permission notice.

const MIN_BATTERY_PERCENT: i32 = 20;
const SEVERE_THERMAL_STATUS: i32 = 3;
const SHUTDOWN_THERMAL_STATUS: i32 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackDecision {
    WaitForCharge,
    Run { workers: usize },
}

/// Decides whether and how to start the next local build step.
///
/// Readings outside Android's documented ranges and absent readings are
/// treated as unknown. An unknown battery level cannot trigger a charge wait,
/// and an unknown thermal status leaves the requested worker count in effect.
///
/// The caller supplies fresh readings at step boundaries. Android retains
/// control of service and process shutdown even when this policy returns Run.
pub fn decide_pack_step(
    battery_percent: Option<i32>,
    charging: bool,
    thermal_status: Option<i32>,
    requested_workers: usize,
) -> PackDecision {
    if !charging && battery_percent.is_some_and(|level| (0..MIN_BATTERY_PERCENT).contains(&level)) {
        return PackDecision::WaitForCharge;
    }
    let hot = thermal_status
        .is_some_and(|level| (SEVERE_THERMAL_STATUS..=SHUTDOWN_THERMAL_STATUS).contains(&level));
    PackDecision::Run {
        workers: if hot { 1 } else { requested_workers.max(1) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_on_low_battery_only_when_not_charging() {
        assert_eq!(
            decide_pack_step(Some(19), false, Some(3), 4),
            PackDecision::WaitForCharge
        );
        assert_eq!(
            decide_pack_step(Some(19), true, Some(3), 4),
            PackDecision::Run { workers: 1 }
        );
        assert_eq!(
            decide_pack_step(Some(20), false, Some(2), 4),
            PackDecision::Run { workers: 4 }
        );
    }

    #[test]
    fn reduces_worker_count_from_severe_through_shutdown() {
        for status in 3..=6 {
            assert_eq!(
                decide_pack_step(Some(90), false, Some(status), 8),
                PackDecision::Run { workers: 1 }
            );
        }
    }

    #[test]
    fn unknown_readings_do_not_trigger_a_false_pause_or_thermal_state() {
        for (battery, thermal) in [(None, None), (Some(-1), Some(-1)), (Some(101), Some(7))] {
            assert_eq!(
                decide_pack_step(battery, false, thermal, 4),
                PackDecision::Run { workers: 4 }
            );
        }
        assert_eq!(
            decide_pack_step(None, false, None, 0),
            PackDecision::Run { workers: 1 }
        );
    }
}
