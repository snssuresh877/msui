use crate::monitor::{BatteryInfo, PowerProfile};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    None,
    SetProfile(PowerProfile),
    NotifyLowBattery { percent: u8, is_critical: bool },
    EmergencySuspend,
}

#[derive(Debug, Clone)]
pub struct PowerPolicyEngine {
    /// Consecutive 2-second samples with CPU usage >= 80% on AC
    high_load_ticks: u32,
    /// Consecutive 2-second samples with CPU usage < 50% on AC (recovery cooldown)
    low_load_ticks: u32,
    /// Whether temporary workload boost to Performance is currently active
    workload_boost_active: bool,
    /// Last battery percentage where a low-battery notification was sent
    last_notified_low_bat: Option<u8>,
    /// Currently applied policy profile
    current_profile: Option<PowerProfile>,
}

impl Default for PowerPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerPolicyEngine {
    pub fn new() -> Self {
        Self {
            high_load_ticks: 0,
            low_load_ticks: 0,
            workload_boost_active: false,
            last_notified_low_bat: None,
            current_profile: None,
        }
    }

    pub fn current_profile(&self) -> Option<PowerProfile> {
        self.current_profile
    }

    pub fn set_current_profile(&mut self, profile: PowerProfile) {
        self.current_profile = Some(profile);
    }

    pub fn is_boost_active(&self) -> bool {
        self.workload_boost_active
    }

    /// Evaluates power policy on a 2-second sampling interval.
    /// Returns a list of actions to execute (profile change, notifications, or emergency suspend).
    pub fn evaluate(&mut self, bat: Option<&BatteryInfo>, cpu_percent: f32) -> Vec<PolicyAction> {
        let mut actions = Vec::new();
        let is_ac = bat.map_or(true, |b| b.is_ac_online);
        let bat_percent = bat.map(|b| b.percentage).unwrap_or(100);

        if is_ac {
            // Reset battery low notification state when connected to AC
            self.last_notified_low_bat = None;

            // Heavy workload detection: sustained 80%+ for >= 10 seconds (5 ticks at 2s/tick)
            if cpu_percent >= 80.0 {
                self.high_load_ticks = self.high_load_ticks.saturating_add(1);
                self.low_load_ticks = 0;
                if self.high_load_ticks >= 5 && !self.workload_boost_active {
                    self.workload_boost_active = true;
                    self.current_profile = Some(PowerProfile::Performance);
                    actions.push(PolicyAction::SetProfile(PowerProfile::Performance));
                }
            } else if cpu_percent < 50.0 {
                self.low_load_ticks = self.low_load_ticks.saturating_add(1);
                if self.low_load_ticks >= 3 {
                    self.high_load_ticks = 0;
                    if self.workload_boost_active {
                        self.workload_boost_active = false;
                        self.current_profile = Some(PowerProfile::Balanced);
                        actions.push(PolicyAction::SetProfile(PowerProfile::Balanced));
                    }
                }
            }

            let target_profile = if self.workload_boost_active {
                PowerProfile::Performance
            } else {
                PowerProfile::Balanced
            };

            if self.current_profile != Some(target_profile) {
                self.current_profile = Some(target_profile);
                actions.push(PolicyAction::SetProfile(target_profile));
            }
        } else {
            // On Battery power:
            // Workload boost is strictly disabled on battery to protect runtime
            self.workload_boost_active = false;
            self.high_load_ticks = 0;
            self.low_load_ticks = 0;

            // 1. Safety threshold: Emergency Suspend (<= 3%)
            if bat_percent <= 3 {
                actions.push(PolicyAction::EmergencySuspend);
            }

            // 2. Critical notification (<= 5%)
            if bat_percent <= 5 {
                let should_notify = match self.last_notified_low_bat {
                    Some(last) => last > 5 || bat_percent < last,
                    None => true,
                };
                if should_notify {
                    self.last_notified_low_bat = Some(bat_percent);
                    actions.push(PolicyAction::NotifyLowBattery {
                        percent: bat_percent,
                        is_critical: true,
                    });
                }
            }
            // 3. Low Battery warning (<= 15%)
            else if bat_percent <= 15 {
                let should_notify = match self.last_notified_low_bat {
                    Some(last) => last > 15 || (last > 10 && bat_percent <= 10),
                    None => true,
                };
                if should_notify {
                    self.last_notified_low_bat = Some(bat_percent);
                    actions.push(PolicyAction::NotifyLowBattery {
                        percent: bat_percent,
                        is_critical: false,
                    });
                }
            }

            // 4. Power Profile resolution based on battery level
            let target_profile = if bat_percent > 30 {
                PowerProfile::Balanced
            } else {
                // 15–30% -> PowerSaver
                // <= 15% -> PowerSaver (Extreme Saver)
                PowerProfile::PowerSaver
            };

            if self.current_profile != Some(target_profile) {
                self.current_profile = Some(target_profile);
                actions.push(PolicyAction::SetProfile(target_profile));
            }
        }

        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::BatteryStatus;

    fn mock_battery(percentage: u8, is_ac: bool) -> BatteryInfo {
        BatteryInfo {
            percentage,
            status: if is_ac { BatteryStatus::Charging } else { BatteryStatus::Discharging },
            is_charging: is_ac,
            is_ac_online: is_ac,
            power_watts: 8.0,
            voltage_volts: 11.5,
            energy_now_wh: 20.0,
            energy_full_wh: 40.0,
            energy_full_design_wh: 50.0,
            health_percent: 80.0,
            cycle_count: Some(500),
            technology: "Li-ion".into(),
            time_remaining_mins: Some(150),
        }
    }

    #[test]
    fn test_ac_normal_and_heavy_workload() {
        let mut engine = PowerPolicyEngine::new();
        let bat = mock_battery(80, true);

        // Initially on AC, load is normal (20%) -> Balanced
        let actions = engine.evaluate(Some(&bat), 20.0);
        assert!(actions.contains(&PolicyAction::SetProfile(PowerProfile::Balanced)));
        assert_eq!(engine.current_profile(), Some(PowerProfile::Balanced));

        // High load for 4 ticks (8s) -> Not yet boosted
        for _ in 0..4 {
            let actions = engine.evaluate(Some(&bat), 85.0);
            assert!(!actions.contains(&PolicyAction::SetProfile(PowerProfile::Performance)));
        }

        // 5th tick (10s total sustained >= 80%) -> Boosts to Performance!
        let actions = engine.evaluate(Some(&bat), 85.0);
        assert!(actions.contains(&PolicyAction::SetProfile(PowerProfile::Performance)));
        assert!(engine.is_boost_active());
        assert_eq!(engine.current_profile(), Some(PowerProfile::Performance));

        // Load drops to 20% for 2 ticks -> Still in cooldown
        for _ in 0..2 {
            let actions = engine.evaluate(Some(&bat), 20.0);
            assert!(!actions.contains(&PolicyAction::SetProfile(PowerProfile::Balanced)));
        }

        // 3rd low-load tick -> Reverts back to Balanced!
        let actions = engine.evaluate(Some(&bat), 20.0);
        assert!(actions.contains(&PolicyAction::SetProfile(PowerProfile::Balanced)));
        assert!(!engine.is_boost_active());
        assert_eq!(engine.current_profile(), Some(PowerProfile::Balanced));
    }

    #[test]
    fn test_battery_threshold_profiles() {
        let mut engine = PowerPolicyEngine::new();

        // Battery > 30% -> Balanced
        let bat_50 = mock_battery(50, false);
        let actions = engine.evaluate(Some(&bat_50), 20.0);
        assert!(actions.contains(&PolicyAction::SetProfile(PowerProfile::Balanced)));
        assert_eq!(engine.current_profile(), Some(PowerProfile::Balanced));

        // Battery in 15..=30% -> PowerSaver
        let bat_25 = mock_battery(25, false);
        let actions = engine.evaluate(Some(&bat_25), 20.0);
        assert!(actions.contains(&PolicyAction::SetProfile(PowerProfile::PowerSaver)));
        assert_eq!(engine.current_profile(), Some(PowerProfile::PowerSaver));

        // Battery <= 15% -> Extreme Saver (PowerSaver + Low battery alert)
        let bat_14 = mock_battery(14, false);
        let actions = engine.evaluate(Some(&bat_14), 20.0);
        assert!(actions.contains(&PolicyAction::NotifyLowBattery { percent: 14, is_critical: false }));

        // Battery <= 5% -> Critical alert
        let bat_5 = mock_battery(5, false);
        let actions = engine.evaluate(Some(&bat_5), 20.0);
        assert!(actions.contains(&PolicyAction::NotifyLowBattery { percent: 5, is_critical: true }));

        // Battery <= 3% -> Emergency suspend
        let bat_2 = mock_battery(2, false);
        let actions = engine.evaluate(Some(&bat_2), 20.0);
        assert!(actions.contains(&PolicyAction::EmergencySuspend));
    }
}
