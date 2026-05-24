use std::collections::HashMap;

use egui_plot::BoxSpread;

use crate::arena::FileArenaSnapshot;

pub struct ExtensionBoxplotChart {
    pub top_extensions: Vec<String>,
    pub computed_spreads: Vec<(String, BoxSpread)>, // (ext, spread)
}

impl ExtensionBoxplotChart {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            top_extensions: Vec::new(),
            computed_spreads: Vec::new(),
        }
    }
}

impl Default for ExtensionBoxplotChart {
    fn default() -> Self {
        Self::new()
    }
}

impl super::StatsChart for ExtensionBoxplotChart {
    type Output = ();

    fn compute(&mut self, snapshot: &FileArenaSnapshot) -> Self::Output {
        self.top_extensions.clear();
        self.computed_spreads.clear();

        if snapshot.nodes.is_empty() {
            return;
        }

        // 1. Map extension names to log10(sizes) vector
        let mut ext_files: HashMap<String, Vec<f64>> = HashMap::new();

        for node in snapshot.nodes.iter() {
            if node.is_directory() {
                continue;
            }
            if let Some(name) = snapshot.string_pool.get(node.name_id) {
                let ext = std::path::Path::new(name).extension().map_or_else(
                    || "(no extension)".to_string(),
                    |s| s.to_string_lossy().to_ascii_lowercase(),
                );
                if node.size > 0 {
                    #[allow(clippy::cast_precision_loss)]
                    let log_size = (node.size as f64).log10();

                    ext_files.entry(ext).or_default().push(log_size);
                }
            }
        }

        if ext_files.is_empty() {
            return;
        }

        // 2. Sort the list of extensions descending by file sample counts to ensure statistical significance
        let mut sorted_exts: Vec<(String, Vec<f64>)> = ext_files.into_iter().collect();
        sorted_exts.sort_by_key(|b| std::cmp::Reverse(b.1.len()));
        sorted_exts.truncate(6);

        // 3. Compute box spread parameters (min, Q1, median, Q3, max)
        for (ext, mut sizes) in sorted_exts {
            if sizes.len() < 4 {
                // Ignore categories with fewer than 4 files to guarantee box structural integrity
                continue;
            }
            sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let len = sizes.len();
            let min = sizes[0];
            let q1 = sizes[len / 4];
            let median = sizes[len / 2];
            let q3 = sizes[(len * 3) / 4];
            let max = sizes[len - 1];

            let spread = BoxSpread::new(min, q1, median, q3, max);
            self.top_extensions.push(ext.clone());
            self.computed_spreads.push((ext, spread));
        }
    }
}
