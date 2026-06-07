use egui_plot::{Bar, BarChart};

pub struct SizeDistributionChart {
    pub cached_counts: Option<[u64; 8]>,
    pub last_snapshot_ptr: usize,
}

impl SizeDistributionChart {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cached_counts: None,
            last_snapshot_ptr: 0,
        }
    }
}

impl Default for SizeDistributionChart {
    fn default() -> Self {
        Self::new()
    }
}

impl super::StatsChart for SizeDistributionChart {
    type Output = [u64; 8];

    fn compute(&mut self, snapshot: &crate::arena::FileArenaSnapshot) -> Self::Output {
        // Bucket allocations:
        // 0: < 10 KB
        // 1: 10 KB - 100 KB
        // 2: 100 KB - 1 MB
        // 3: 1 MB - 10 MB
        // 4: 10 MB - 100 MB
        // 5: 100 MB - 1 GB
        // 6: 1 GB - 10 GB
        // 7: > 10 GB
        let mut counts = [0u64; 8];

        for node in snapshot.nodes.iter() {
            if node.is_directory() {
                continue;
            }
            let size = node.size;

            // Branchless comparison flags
            let b1 = (size >= 10_000) as usize;
            let b2 = (size >= 100_000) as usize;
            let b3 = (size >= 1_000_000) as usize;
            let b4 = (size >= 10_000_000) as usize;
            let b5 = (size >= 100_000_000) as usize;
            let b6 = (size >= 1_000_000_000) as usize;
            let b7 = (size >= 10_000_000_000) as usize;

            // The sum of satisfied thresholds yields the precise destination bucket index
            let bucket_idx = b1 + b2 + b3 + b4 + b5 + b6 + b7;
            counts[bucket_idx] += 1;
        }
        counts
    }
}

impl super::StatComponent for SizeDistributionChart {
    fn render(
        &mut self,
        ui: &mut eframe::egui::Ui,
        snapshot: &crate::arena::FileArenaSnapshot,
        _context: &mut super::StatContext,
    ) {
        use super::StatsChart;
        let snapshot_ptr = std::sync::Arc::as_ptr(&snapshot.nodes) as usize;
        let needs_rebuild = self.cached_counts.is_none() || self.last_snapshot_ptr != snapshot_ptr;

        if needs_rebuild {
            self.cached_counts = Some(self.compute(snapshot));
            self.last_snapshot_ptr = snapshot_ptr;
        }

        if let Some(counts) = &self.cached_counts {
            let labels = [
                "< 10 KB",
                "10 KB - 100 KB",
                "100 KB - 1 MB",
                "1 MB - 10 MB",
                "10 MB - 100 MB",
                "100 MB - 1 GB",
                "1 GB - 10 GB",
                "> 10 GB",
            ];

            let bars: Vec<Bar> = counts
                .iter()
                .enumerate()
                .map(|(i, &count)| {
                    #[allow(clippy::cast_precision_loss)]
                    let index = i as f64;
                    #[allow(clippy::cast_precision_loss)]
                    let count = count as f64;

                    Bar::new(index, count)
                        .name(labels[i])
                        .fill(crate::colors::COLOR_SCANNING)
                })
                .collect();

            let bar_chart = BarChart::new("Size Distribution", bars).width(0.6);

            let formatter = |mark: egui_plot::GridMark, _range: &std::ops::RangeInclusive<f64>| {
                let labels = [
                    "< 10 KB",
                    "10 KB - 100 KB",
                    "100 KB - 1 MB",
                    "1 MB - 10 MB",
                    "10 MB - 100 MB",
                    "100 MB - 1 GB",
                    "1 GB - 10 GB",
                    "> 10 GB",
                ];
                let val = mark.value.round() as usize;
                if val < labels.len() {
                    labels[val].to_string()
                } else {
                    String::new()
                }
            };

            let x_grid = |_input: egui_plot::GridInput| {
                let mut marks = vec![];
                for i in 0..8 {
                    marks.push(egui_plot::GridMark {
                        value: i as f64,
                        step_size: 1.0,
                    });
                }
                marks
            };

            let x_axes = vec![
                egui_plot::AxisHints::new_x()
                    .label("File Size Bracket")
                    .formatter(formatter),
            ];

            let plot = egui_plot::Plot::new("size_dist_plot")
                .height(ui.available_height() - 10.0)
                .custom_x_axes(x_axes)
                .x_grid_spacer(x_grid)
                .y_axis_label("File Count")
                .allow_zoom(false)
                .allow_drag(false)
                .allow_scroll(false);

            plot.show(ui, |plot_ui| {
                plot_ui.bar_chart(bar_chart);
            });
        }
    }
}
