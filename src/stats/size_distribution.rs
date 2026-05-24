use egui_plot::{Bar, BarChart};

pub struct SizeDistributionChart;

impl super::StatsChart for SizeDistributionChart {
    type Output = BarChart;

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
            if size < 10_000 {
                counts[0] += 1;
            } else if size < 100_000 {
                counts[1] += 1;
            } else if size < 1_000_000 {
                counts[2] += 1;
            } else if size < 10_000_000 {
                counts[3] += 1;
            } else if size < 100_000_000 {
                counts[4] += 1;
            } else if size < 1_000_000_000 {
                counts[5] += 1;
            } else if size < 10_000_000_000 {
                counts[6] += 1;
            } else {
                counts[7] += 1;
            }
        }

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

        BarChart::new("Size Distribution", bars).width(0.6)
    }
}
