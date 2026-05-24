use std::collections::HashMap;

use crate::arena::FileArenaSnapshot;

pub struct TemporalTimelineChart {
    pub sorted_days: Vec<i64>,
    pub daily_totals: HashMap<i64, (u64, u32)>, // Day_secs -> (size_sum, count)
}

impl TemporalTimelineChart {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sorted_days: Vec::new(),
            daily_totals: HashMap::new(),
        }
    }
}

impl Default for TemporalTimelineChart {
    fn default() -> Self {
        Self::new()
    }
}

impl super::StatsChart for TemporalTimelineChart {
    type Output = ();

    fn compute(&mut self, snapshot: &FileArenaSnapshot) -> Self::Output {
        self.sorted_days.clear();
        self.daily_totals.clear();

        if snapshot.nodes.is_empty() {
            return;
        }

        // 1. Bucket files by 24-hour day boundaries (86,400 seconds)
        for node in snapshot.nodes.iter() {
            if node.is_directory() {
                continue;
            }
            if node.modified_timestamp > 0 {
                let day_boundary = (node.modified_timestamp / 86400) * 86400;
                let entry = self.daily_totals.entry(day_boundary).or_insert((0, 0));
                entry.0 += node.size;
                entry.1 += 1;
            }
        }

        if self.daily_totals.is_empty() {
            return;
        }

        // 2. Sort key boundaries chronologically
        self.sorted_days = self.daily_totals.keys().copied().collect();
        self.sorted_days.sort_unstable();

        // 3. Keep dataset bounded to 5,000 active days to avoid visual clutter
        if self.sorted_days.len() > 5000 {
            let truncate_len = self.sorted_days.len() - 5000;
            self.sorted_days.drain(0..truncate_len);
        }
    }
}

/// Translates Unix Epoch seconds directly to a "YYYY-MM-DD" Gregorian calendar string.
#[must_use]
pub fn format_epoch_to_date(epoch_secs: i64) -> String {
    if epoch_secs <= 0 {
        return "Pre-1970".to_string();
    }
    let days_since_epoch = epoch_secs / 86400;

    let mut year = 1970;
    let mut days_left = days_since_epoch;

    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    let mut day_of_month = days_left + 1;
    for &days in &month_days {
        if day_of_month <= days {
            break;
        }
        day_of_month -= days;
        month += 1;
    }

    format!("{year:04}-{month:02}-{day_of_month:02}")
}
