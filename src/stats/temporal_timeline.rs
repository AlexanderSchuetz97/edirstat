use std::collections::HashMap;

use crate::arena::FileArenaSnapshot;

pub struct TemporalTimelineChart {
    pub sorted_days: Vec<i64>,
    pub daily_totals: HashMap<i64, (u64, u32)>, // Day_secs -> (size_sum, count)
    pub last_snapshot_ptr: usize,
}

impl TemporalTimelineChart {
    #[must_use]
    pub fn new() -> Self {
        Self {
            sorted_days: Vec::new(),
            daily_totals: HashMap::new(),
            last_snapshot_ptr: 0,
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

impl super::StatComponent for TemporalTimelineChart {
    fn render(
        &mut self,
        ui: &mut eframe::egui::Ui,
        snapshot: &crate::arena::FileArenaSnapshot,
        _context: &mut super::StatContext,
    ) {
        use super::StatsChart;
        let snapshot_ptr = std::sync::Arc::as_ptr(&snapshot.nodes) as usize;
        let needs_rebuild = self.last_snapshot_ptr != snapshot_ptr || self.sorted_days.is_empty();

        if needs_rebuild {
            self.compute(snapshot);
            self.last_snapshot_ptr = snapshot_ptr;
        }

        if self.sorted_days.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label("No file modification metadata available to construct timelines.");
            });
            return;
        }

        // Build Space Points (cumulative) and Activity Points (daily frequency)
        let mut space_points = Vec::new();
        let mut activity_points = Vec::new();

        let mut cumulative_size = 0u64;
        for &day in &self.sorted_days {
            let (size, count) = self.daily_totals[&day];
            cumulative_size += size;

            #[allow(clippy::cast_precision_loss)]
            let d = day as f64;
            #[allow(clippy::cast_precision_loss)]
            let count_d = count as f64;
            #[allow(clippy::cast_precision_loss)]
            let cumulative_size_d = cumulative_size as f64;

            space_points.push([d, cumulative_size_d]);
            activity_points.push([d, count_d]);
        }

        // Custom time-axis calendar formatter
        let x_formatter = |mark: egui_plot::GridMark, _range: &std::ops::RangeInclusive<f64>| {
            let val = mark.value.round() as i64;
            format_epoch_to_date(val)
        };

        let y_space_formatter =
            |mark: egui_plot::GridMark, _range: &std::ops::RangeInclusive<f64>| {
                let val = mark.value;
                if val <= 0.0 {
                    return String::new();
                }
                prettier_bytes::ByteFormatter::new()
                    .format(val as u64)
                    .to_string()
            };

        // Shared link structures
        let link_group_id = ui.id().with("linked_timeline_plots");
        let link_axis = eframe::egui::Vec2b::new(true, false); // link X only, do not scale Y together
        let link_cursor = eframe::egui::Vec2b::new(true, false);

        let space_line = egui_plot::Line::new("Space Progress", space_points)
            .color(crate::colors::COLOR_SCANNING)
            .width(2.0);

        let activity_line = egui_plot::Line::new("Activity Frequency", activity_points)
            .color(crate::colors::GLOW_INNER_CORE)
            .width(1.5);

        // Render dual layout
        let half_height = (ui.available_height() - 40.0) / 2.0;

        ui.label(
            "Timeline views are dynamically linked; zooming/panning one will scroll the other.",
        );
        ui.add_space(4.0);

        // 1. Top Plot: Cumulative Storage Growth
        let top_x = vec![egui_plot::AxisHints::new_x().formatter(x_formatter)];
        let top_y = vec![
            egui_plot::AxisHints::new_y()
                .label("Disk Space")
                .formatter(y_space_formatter),
        ];
        let plot_top = egui_plot::Plot::new("timeline_space_plot")
            .height(half_height)
            .custom_x_axes(top_x)
            .custom_y_axes(top_y)
            .link_axis(link_group_id, link_axis)
            .link_cursor(link_group_id, link_cursor)
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop));

        plot_top.show(ui, |plot_ui| {
            plot_ui.line(space_line);
        });

        ui.add_space(6.0);

        // 2. Bottom Plot: Activity frequency spikes
        let bottom_x = vec![egui_plot::AxisHints::new_x().formatter(x_formatter)];
        let bottom_y = vec![egui_plot::AxisHints::new_y().label("Files Modified")];
        let plot_bottom = egui_plot::Plot::new("timeline_activity_plot")
            .height(half_height)
            .custom_x_axes(bottom_x)
            .custom_y_axes(bottom_y)
            .link_axis(link_group_id, link_axis)
            .link_cursor(link_group_id, link_cursor)
            .legend(egui_plot::Legend::default().position(egui_plot::Corner::LeftTop));

        plot_bottom.show(ui, |plot_ui| {
            plot_ui.line(activity_line);
        });
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
