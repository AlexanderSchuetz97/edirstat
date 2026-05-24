use std::collections::HashMap;

use egui_plot::{Bar, BarChart};

use crate::arena::{FileArenaSnapshot, FileNode, NO_INDEX, StringPool};

pub struct DirCompositionChart {
    pub parent_idx: u32,
    pub top_extensions: Vec<String>,
    // Holds (child_name, child_extension_map, total_bytes) for the top 8 children
    pub children_composition: Vec<(String, HashMap<String, u64>, u64)>,
}

impl DirCompositionChart {
    #[must_use]
    pub const fn new(parent_idx: u32) -> Self {
        Self {
            parent_idx,
            top_extensions: Vec::new(),
            children_composition: Vec::new(),
        }
    }
}

impl super::StatsChart for DirCompositionChart {
    type Output = Vec<BarChart>;

    fn compute(&mut self, snapshot: &FileArenaSnapshot) -> Self::Output {
        self.top_extensions.clear();
        self.children_composition.clear();

        if snapshot.nodes.is_empty() || self.parent_idx as usize >= snapshot.nodes.len() {
            return Vec::new();
        }

        let parent_node = &snapshot.nodes[self.parent_idx as usize];
        if !parent_node.is_directory() {
            return Vec::new();
        }

        // 1. Gather all immediate children of the parent directory
        let mut immediate_children = Vec::new();
        let mut curr = parent_node.first_child;
        while curr != NO_INDEX {
            immediate_children.push(curr);
            curr = snapshot.nodes[curr as usize].next_sibling;
        }

        if immediate_children.is_empty() {
            return Vec::new();
        }

        // Sort immediate children descending by size
        immediate_children.sort_by(|&a, &b| {
            snapshot.nodes[b as usize]
                .size
                .cmp(&snapshot.nodes[a as usize].size)
        });

        // Restrict to the top 8 largest children for clean, readable layout spacing
        immediate_children.truncate(8);

        // 2. Compute extension composition for each child
        let mut overall_ext_sizes: HashMap<String, u64> = HashMap::new();

        for &child_idx in &immediate_children {
            let child_node = &snapshot.nodes[child_idx as usize];
            let name = snapshot
                .string_pool
                .get(child_node.name_id)
                .unwrap_or("unknown")
                .to_string();

            let mut ext_map = HashMap::new();
            if child_node.is_directory() {
                // Recursively gather file extension profiles of the subdirectory
                gather_dir_extensions(
                    &snapshot.nodes,
                    &snapshot.string_pool,
                    child_idx,
                    &mut ext_map,
                );
            } else {
                let ext = std::path::Path::new(&name).extension().map_or_else(
                    || "(no extension)".to_string(),
                    |s| s.to_string_lossy().to_ascii_lowercase(),
                );
                ext_map.insert(ext, child_node.size);
            }

            // Aggregate overall sizes to identify dominant extension groups
            for (ext, size) in &ext_map {
                *overall_ext_sizes.entry(ext.clone()).or_insert(0) += size;
            }

            self.children_composition
                .push((name, ext_map, child_node.size));
        }

        // Sort globally observed extensions to choose the top 5 largest formats
        let mut sorted_exts: Vec<(String, u64)> = overall_ext_sizes.into_iter().collect();
        sorted_exts.sort_by_key(|b| std::cmp::Reverse(b.1));
        sorted_exts.truncate(5);

        let top_exts: Vec<String> = sorted_exts.into_iter().map(|(ext, _)| ext).collect();
        self.top_extensions.clone_from(&top_exts);

        // 3. Build stacked BarCharts
        let mut unstacked_charts = Vec::new();

        // Color mapping helper matching the global layout extensions
        let color_for_ext = |ext: &str| -> eframe::egui::Color32 {
            match ext {
                "rs" => crate::colors::EXT_RUST,
                "toml" => crate::colors::EXT_TOML,
                "git" | "gitignore" => crate::colors::EXT_GIT,
                "js" | "ts" => crate::colors::EXT_JS_TS,
                "json" | "yaml" => crate::colors::EXT_CONFIG,
                "html" | "css" => crate::colors::EXT_WEB,
                "py" => crate::colors::EXT_PYTHON,
                "c" | "cpp" | "h" => crate::colors::EXT_CPP,
                "zip" | "tar" | "gz" => crate::colors::EXT_COMPRESSED,
                "mp3" | "wav" | "flac" => crate::colors::EXT_AUDIO,
                "mp4" | "mkv" | "avi" => crate::colors::EXT_VIDEO,
                "png" | "jpg" | "jpeg" | "gif" => crate::colors::EXT_IMAGE,
                "(no extension)" => crate::colors::EXT_NONE,
                _ => {
                    let mut hash: u32 = 5381;
                    for c in ext.bytes() {
                        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(c as u32);
                    }
                    #[allow(clippy::cast_precision_loss)]
                    let hue = (hash % 360) as f32 / 360.0;
                    eframe::egui::Color32::from(eframe::egui::epaint::Hsva::new(
                        hue, 0.75, 0.55, 1.0,
                    ))
                }
            }
        };

        // Create individual BarChart bars for each top extension
        for ext in &top_exts {
            let mut bars = Vec::new();
            for (i, (_child_name, ext_map, _total_size)) in
                self.children_composition.iter().enumerate()
            {
                #[allow(clippy::cast_precision_loss)]
                let height = *ext_map.get(ext).unwrap_or(&0) as f64;
                #[allow(clippy::cast_precision_loss)]
                let index = i as f64;

                bars.push(Bar::new(index, height).name(ext));
            }
            let chart = BarChart::new(ext.clone(), bars)
                .width(0.5)
                .name(format!(".{ext} files"))
                .color(color_for_ext(ext));
            unstacked_charts.push(chart);
        }

        // Add remaining non-dominant extensions under the "Other" category
        let mut other_bars = Vec::new();
        for (i, (_child_name, ext_map, _total_size)) in self.children_composition.iter().enumerate()
        {
            let mut other_height = 0u64;
            for (ext, &size) in ext_map {
                if !top_exts.contains(ext) {
                    other_height += size;
                }
            }
            #[allow(clippy::cast_precision_loss)]
            let index = i as f64;
            #[allow(clippy::cast_precision_loss)]
            let height = other_height as f64;

            other_bars.push(Bar::new(index, height).name("Other"));
        }
        let other_chart = BarChart::new("Other".to_string(), other_bars)
            .width(0.5)
            .name("Other files")
            .color(crate::colors::TREEMAP_DIR_FALLBACK);
        unstacked_charts.push(other_chart);

        // 4. Transform unstacked series into a stacked vector
        let mut stacked_charts = Vec::new();
        for unstacked in unstacked_charts {
            let refs: Vec<&BarChart> = stacked_charts.iter().collect();
            let stacked = unstacked.stack_on(&refs);
            stacked_charts.push(stacked);
        }

        stacked_charts
    }
}

/// Accumulates file sizes of a directory subtree in a safe, stack-based non-recursive layout,
/// capped at 20,000 files to guarantee lightning-fast visual updates.
pub fn gather_dir_extensions<S: ::std::hash::BuildHasher>(
    nodes: &[FileNode],
    string_pool: &StringPool,
    start_idx: u32,
    ext_sizes: &mut HashMap<String, u64, S>,
) {
    let mut stack = vec![start_idx];
    let mut visited_count = 0;

    while let Some(idx) = stack.pop() {
        visited_count += 1;
        if visited_count > 20000 {
            break;
        }

        let node = &nodes[idx as usize];
        if node.is_directory() {
            let mut curr = node.first_child;
            while curr != NO_INDEX {
                stack.push(curr);
                curr = nodes[curr as usize].next_sibling;
            }
        } else {
            let name = string_pool.get(node.name_id).unwrap_or("");
            let ext = std::path::Path::new(name).extension().map_or_else(
                || "(no extension)".to_string(),
                |s| s.to_string_lossy().to_ascii_lowercase(),
            );
            *ext_sizes.entry(ext).or_insert(0) += node.size;
        }
    }
}
