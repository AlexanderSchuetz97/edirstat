use std::path::Path;

use eframe::egui::{Color32, Rect, pos2};
use smallvec::SmallVec;

use super::StatsChart;
use crate::arena::{FileNode, NO_INDEX, StringPool};

const NO_EXTENSION: &str = "(no extension)";

pub struct TreemapBlock {
    pub rect: Rect,
    pub node_idx: u32,
    pub color: Color32,
}

pub struct TreemapChart {
    pub rect: Rect,
}

impl TreemapChart {
    #[must_use]
    pub const fn new(rect: Rect) -> Self {
        Self { rect }
    }
}

impl StatsChart for TreemapChart {
    type Output = Vec<TreemapBlock>;

    fn compute(&mut self, snapshot: &crate::arena::FileArenaSnapshot) -> Self::Output {
        let mut blocks = Vec::new();
        if snapshot.nodes.is_empty() {
            return blocks;
        }

        let config = TreemapConfig {
            nodes: &snapshot.nodes,
            string_pool: &snapshot.string_pool,
            max_depth: 20,
        };

        build_treemap(&config, 0, self.rect, 0, &mut blocks);
        blocks
    }
}

struct TreemapConfig<'a> {
    nodes: &'a [FileNode],
    string_pool: &'a StringPool,
    max_depth: usize,
}

fn worst_aspect_ratio(row: &[f64], w: f64) -> f64 {
    if row.is_empty() || w <= 0.0 {
        return f64::INFINITY;
    }
    let sum: f64 = row.iter().sum();
    if sum <= 0.0 {
        return f64::INFINITY;
    }
    let sum_sq = sum * sum;
    let w_sq = w * w;

    let mut max_ratio = 0.0;
    for &area in row {
        if area <= 0.0 {
            continue;
        }
        let ratio1 = (w_sq * area) / sum_sq;
        let ratio2 = sum_sq / (w_sq * area);
        let ratio = ratio1.max(ratio2);
        if ratio > max_ratio {
            max_ratio = ratio;
        }
    }
    max_ratio
}

fn recurse_child(
    config: &TreemapConfig,
    child_idx: u32,
    child_rect: Rect,
    depth: usize,
    blocks: &mut Vec<TreemapBlock>,
) {
    const MIN_PIXEL_DIM: f32 = 12.0;

    if child_rect.width() <= 0.0 || child_rect.height() <= 0.0 {
        return;
    }

    let child = &config.nodes[child_idx as usize];

    let is_leaf_or_too_small = !child.is_directory()
        || depth >= config.max_depth
        || child_rect.width() < MIN_PIXEL_DIM
        || child_rect.height() < MIN_PIXEL_DIM;

    if is_leaf_or_too_small {
        let name = config.string_pool.get(child.name_id).unwrap_or("");
        let ext = Path::new(name).extension().map_or_else(
            || NO_EXTENSION.to_string(),
            |s| s.to_string_lossy().to_ascii_lowercase(),
        );
        let color = get_color_for_extension(&ext);
        blocks.push(TreemapBlock {
            rect: child_rect,
            node_idx: child_idx,
            color,
        });
        return;
    }

    build_treemap(config, child_idx, child_rect, depth + 1, blocks);
}

/// Walks up the parent chain of a node to determine if it is a descendant of a target ancestor.
#[must_use]
pub fn is_descendant(nodes: &[FileNode], child_idx: u32, ancestor_idx: u32) -> bool {
    let mut curr = Some(child_idx);
    while let Some(idx) = curr {
        if idx == ancestor_idx {
            return true;
        }
        if let Some(node) = nodes.get(idx as usize) {
            curr = node.parent_opt();
        } else {
            break;
        }
    }
    false
}

fn build_treemap(
    config: &TreemapConfig,
    node_idx: u32,
    rect: Rect,
    depth: usize,
    blocks: &mut Vec<TreemapBlock>,
) {
    const MIN_AVG_CHILD_AREA: f64 = 16.0;

    let node = &config.nodes[node_idx as usize];
    if node.size == 0 || rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }

    if !node.is_directory() || depth >= config.max_depth {
        let name = config.string_pool.get(node.name_id).unwrap_or("");
        let ext = Path::new(name).extension().map_or_else(
            || NO_EXTENSION.to_string(),
            |s| s.to_string_lossy().to_ascii_lowercase(),
        );
        let color = get_color_for_extension(&ext);

        blocks.push(TreemapBlock {
            rect,
            node_idx,
            color,
        });
        return;
    }

    let mut children = SmallVec::<[u32; 16]>::new();
    let mut curr = node.first_child;
    while curr != NO_INDEX {
        children.push(curr);
        curr = config.nodes[curr as usize].next_sibling;
    }

    if children.is_empty() {
        let color = crate::colors::TREEMAP_DIR_FALLBACK;
        blocks.push(TreemapBlock {
            rect,
            node_idx,
            color,
        });
        return;
    }

    let area = (rect.width() * rect.height()) as f64;

    #[allow(clippy::cast_precision_loss)]
    let avg_area_per_child = area / children.len() as f64;

    if avg_area_per_child < MIN_AVG_CHILD_AREA {
        let name = config.string_pool.get(node.name_id).unwrap_or("");
        let ext = Path::new(name).extension().map_or_else(
            || NO_EXTENSION.to_string(),
            |s| s.to_string_lossy().to_ascii_lowercase(),
        );
        let color = get_color_for_extension(&ext);
        blocks.push(TreemapBlock {
            rect,
            node_idx,
            color,
        });
        return;
    }

    children.sort_by(|&a, &b| {
        config.nodes[b as usize]
            .size
            .cmp(&config.nodes[a as usize].size)
    });

    let active_children: Vec<u32> = children
        .into_iter()
        .filter(|&idx| config.nodes[idx as usize].size > 0)
        .collect();

    if active_children.is_empty() {
        return;
    }

    #[allow(clippy::cast_precision_loss)]
    let total_size = active_children
        .iter()
        .map(|&idx| config.nodes[idx as usize].size)
        .sum::<u64>() as f64;

    if total_size == 0.0 {
        return;
    }

    let total_area = (rect.width() * rect.height()) as f64;
    let child_areas: Vec<f64> = active_children
        .iter()
        .map(|&idx| {
            #[allow(clippy::cast_precision_loss)]
            let size = config.nodes[idx as usize].size as f64;
            (size / total_size) * total_area
        })
        .collect();

    let mut remaining_rect = rect;
    let mut i = 0;

    while i < active_children.len() {
        let w = (remaining_rect.width().min(remaining_rect.height())) as f64;
        if w <= 0.0 {
            break;
        }

        let mut current_row = Vec::new();
        current_row.push(child_areas[i]);
        let mut j = i + 1;

        while j < active_children.len() {
            let next_area = child_areas[j];
            let mut test_row = current_row.clone();
            test_row.push(next_area);

            let worst_before = worst_aspect_ratio(&current_row, w);
            let worst_after = worst_aspect_ratio(&test_row, w);

            if worst_after <= worst_before {
                current_row.push(next_area);
                j += 1;
            } else {
                break;
            }
        }

        let row_sum: f64 = current_row.iter().sum();
        let vertical_layout = remaining_rect.width() >= remaining_rect.height();

        if vertical_layout {
            let h = remaining_rect.height() as f64;
            let thickness = if h > 0.0 { row_sum / h } else { 0.0 };
            let mut current_y = remaining_rect.min.y;

            for (k, &area) in current_row.iter().enumerate() {
                let child_idx = active_children[i + k];
                let item_height = if row_sum > 0.0 {
                    h * (area / row_sum)
                } else {
                    0.0
                };

                let child_rect = Rect::from_min_max(
                    pos2(remaining_rect.min.x, current_y),
                    pos2(
                        (remaining_rect.min.x + thickness as f32).min(remaining_rect.max.x),
                        (current_y + item_height as f32).min(remaining_rect.max.y),
                    ),
                );

                recurse_child(config, child_idx, child_rect, depth, blocks);
                current_y += item_height as f32;
            }

            remaining_rect.min.x =
                (remaining_rect.min.x + thickness as f32).min(remaining_rect.max.x);
        } else {
            let width = remaining_rect.width() as f64;
            let thickness = if width > 0.0 { row_sum / width } else { 0.0 };
            let mut current_x = remaining_rect.min.x;

            for (k, &area) in current_row.iter().enumerate() {
                let child_idx = active_children[i + k];
                let item_width = if row_sum > 0.0 {
                    width * (area / row_sum)
                } else {
                    0.0
                };

                let child_rect = Rect::from_min_max(
                    pos2(current_x, remaining_rect.min.y),
                    pos2(
                        (current_x + item_width as f32).min(remaining_rect.max.x),
                        (remaining_rect.min.y + thickness as f32).min(remaining_rect.max.y),
                    ),
                );

                recurse_child(config, child_idx, child_rect, depth, blocks);
                current_x += item_width as f32;
            }

            remaining_rect.min.y =
                (remaining_rect.min.y + thickness as f32).min(remaining_rect.max.y);
        }

        i = j;
    }
}

fn get_color_for_extension(ext: &str) -> Color32 {
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
        NO_EXTENSION => crate::colors::EXT_NONE,
        _ => {
            let mut hash: u32 = 5381;
            for c in ext.bytes() {
                hash = ((hash << 5).wrapping_add(hash)).wrapping_add(c as u32);
            }
            #[allow(clippy::cast_precision_loss)]
            let hue = (hash % 360) as f32 / 360.0;

            let color = eframe::epaint::Hsva::new(hue, 0.75, 0.55, 1.0);
            Color32::from(color)
        }
    }
}
