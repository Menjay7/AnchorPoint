//! Token Vesting Contract
//!
//! This vesting module is intended to be used as a foundation for a time-locked
//! token release mechanism with optional cliff and linear vesting.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VestingSchedule {
    pub start_timestamp: u64,
    pub cliff_seconds: u64,
    pub duration_seconds: u64,
    pub total_amount: u128,
}

impl VestingSchedule {
    pub fn new(start_timestamp: u64, cliff_seconds: u64, duration_seconds: u64, total_amount: u128) -> Self {
        assert!(duration_seconds > 0, "duration must be positive");
        assert!(total_amount > 0, "amount must be positive");
        assert!(cliff_seconds <= duration_seconds, "cliff cannot be longer than duration");

        Self {
            start_timestamp,
            cliff_seconds,
            duration_seconds,
            total_amount,
        }
    }

    pub fn vested_amount(&self, current_timestamp: u64) -> u128 {
        if current_timestamp < self.start_timestamp + self.cliff_seconds {
            return 0;
        }

        if current_timestamp >= self.start_timestamp + self.duration_seconds {
            return self.total_amount;
        }

        let elapsed = current_timestamp.saturating_sub(self.start_timestamp);
        let vested = (self.total_amount as u128)
            .saturating_mul(elapsed as u128)
            .checked_div(self.duration_seconds as u128)
            .unwrap_or(0);

        vested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vesting_before_start_is_zero() {
        let schedule = VestingSchedule::new(1_000, 100, 1_000, 1_000_000);
        assert_eq!(schedule.vested_amount(999), 0);
    }

    #[test]
    fn vesting_before_cliff_is_zero() {
        let schedule = VestingSchedule::new(1_000, 200, 1_000, 1_000_000);
        assert_eq!(schedule.vested_amount(1_100), 0);
    }

    #[test]
    fn vesting_at_cliff_starts_linear() {
        let schedule = VestingSchedule::new(1_000, 200, 1_000, 1_000_000);
        let cliff_time = 1_200;
        // cliff reached, elapsed is 200, 20% of duration -> 200_000
        assert_eq!(schedule.vested_amount(cliff_time), 200_000);
    }

    #[test]
    fn vesting_mid_duration() {
        let schedule = VestingSchedule::new(1_000, 100, 1_000, 500_000);
        let current = 1_500; // 500 elapsed, 50% duration
        assert_eq!(schedule.vested_amount(current), 250_000);
    }

    #[test]
    fn vesting_after_duration_is_total() {
        let schedule = VestingSchedule::new(1_000, 100, 1_000, 500_000);
        assert_eq!(schedule.vested_amount(2_100), 500_000);
        assert_eq!(schedule.vested_amount(3_000), 500_000);
    }

    #[test]
    #[should_panic(expected = "duration must be positive")]
    fn new_zero_duration_panics() {
        VestingSchedule::new(1_000, 0, 0, 1_000);
    }

    #[test]
    #[should_panic(expected = "amount must be positive")]
    fn new_zero_amount_panics() {
        VestingSchedule::new(1_000, 0, 1_000, 0);
    }

    #[test]
    #[should_panic(expected = "cliff cannot be longer than duration")]
    fn new_cliff_longer_than_duration_panics() {
        VestingSchedule::new(1_000, 2_000, 1_000, 1_000);
    }

    #[test]
    fn zero_cliff_vesting_starts_immediately() {
        let schedule = VestingSchedule::new(1_000, 0, 1_000, 1_000_000);
        // At start, elapsed = 0, vested = 0
        assert_eq!(schedule.vested_amount(1_000), 0);
        // After 1 second (cliff = 0, so immediately vesting)
        assert_eq!(schedule.vested_amount(1_001), 1_000);
    }

    #[test]
    fn cliff_equals_duration() {
        let schedule = VestingSchedule::new(1_000, 500, 500, 500_000);
        // Before cliff: 0
        assert_eq!(schedule.vested_amount(1_499), 0);
        // At cliff (cliff == duration): all tokens
        assert_eq!(schedule.vested_amount(1_500), 500_000);
    }

    #[test]
    fn just_before_cliff() {
        let schedule = VestingSchedule::new(1_000, 100, 1_000, 1_000_000);
        let just_before = 1_099;
        assert_eq!(schedule.vested_amount(just_before), 0);
    }

    #[test]
    fn just_after_cliff() {
        let schedule = VestingSchedule::new(1_000, 100, 1_000, 1_000_000);
        let just_after = 1_101;
        // elapsed = 101, duration = 1000, 101/1000 = 101_000
        assert_eq!(schedule.vested_amount(just_after), 101_000);
    }

    #[test]
    fn exact_cliff_timestamp() {
        let schedule = VestingSchedule::new(1_000, 100, 1_000, 1_000_000);
        let exact_cliff = 1_100;
        // elapsed = 100, duration = 1000, 100/1000 = 100_000
        assert_eq!(schedule.vested_amount(exact_cliff), 100_000);
    }

    #[test]
    fn three_quarter_vesting() {
        let schedule = VestingSchedule::new(0, 0, 1_000, 1_000_000);
        let three_qtr = 750;
        // elapsed = 750, duration = 1000, 750/1000 = 750_000
        assert_eq!(schedule.vested_amount(three_qtr), 750_000);
    }

    #[test]
    fn one_quarter_vesting() {
        let schedule = VestingSchedule::new(0, 0, 1_000, 4_000_000);
        let one_qtr = 250;
        // elapsed = 250, duration = 1000, 250/1000 = 1_000_000
        assert_eq!(schedule.vested_amount(one_qtr), 1_000_000);
    }

    #[test]
    fn large_numbers() {
        let schedule = VestingSchedule::new(0, 0, u64::MAX / 2, u128::MAX / 4);
        let mid_time = u64::MAX / 4;
        // Should not overflow or panic
        let vested = schedule.vested_amount(mid_time);
        assert!(vested > 0);
        assert!(vested < u128::MAX / 4);
    }

    #[test]
    fn start_timestamp_zero() {
        let schedule = VestingSchedule::new(0, 100, 1_000, 500_000);
        assert_eq!(schedule.vested_amount(0), 0);
        assert_eq!(schedule.vested_amount(100), 50_000);
        assert_eq!(schedule.vested_amount(1_000), 500_000);
    }

    #[test]
    fn one_token_over_long_duration() {
        let schedule = VestingSchedule::new(0, 0, 1_000_000, 1);
        assert_eq!(schedule.vested_amount(500_000), 0); // rounding down
        assert_eq!(schedule.vested_amount(1_000_000), 1);
    }

    #[test]
    fn precise_half_vesting() {
        let schedule = VestingSchedule::new(0, 0, 2_000, 1_000_000);
        // At exactly half duration
        assert_eq!(schedule.vested_amount(1_000), 500_000);
    }

    #[test]
    fn saturating_behavior_before_start() {
        let schedule = VestingSchedule::new(u64::MAX - 1, 0, 100, 1_000_000);
        // Current time before start -> saturating_sub gives 0
        assert_eq!(schedule.vested_amount(0), 0);
        assert_eq!(schedule.vested_amount(u64::MAX - 2), 0);
    }

    #[test]
    fn multiple_schedules_independent() {
        let sched1 = VestingSchedule::new(1_000, 100, 1_000, 500_000);
        let sched2 = VestingSchedule::new(2_000, 200, 2_000, 1_000_000);

        assert_eq!(sched1.vested_amount(1_500), 250_000);
        assert_eq!(sched2.vested_amount(2_500), 250_000);
        assert_eq!(sched1.vested_amount(1_100), 50_000);
        assert_eq!(sched2.vested_amount(2_200), 100_000);
    }

    #[test]
    fn copy_clone_behavior() {
        let sched = VestingSchedule::new(1_000, 100, 1_000, 500_000);
        let sched_copy = sched;
        assert_eq!(sched, sched_copy);
        assert_eq!(sched.vested_amount(1_500), sched_copy.vested_amount(1_500));
    }
}
