use eframe::egui;
use smallvec::SmallVec;

use super::{GuiApp, theme};
use crate::arena::{FileArenaSnapshot, NO_INDEX};

impl GuiApp {
    pub fn flatten_visible_tree(
        &mut self,
        snapshot: &FileArenaSnapshot,
        node_idx: u32,
        indent_level: usize,
        out: &mut Vec<(u32, usize)>,
    ) {
        let node = &snapshot.nodes[node_idx as usize];
        let name = snapshot.string_pool.get(node.name_id).unwrap_or("unknown");

        // Filter search query
        if !self.search_query.is_empty() {
            let matches_query = name
                .to_lowercase()
                .contains(&self.search_query.to_lowercase());
            // If it's a file and doesn't match, skip
            if !node.is_directory() && !matches_query {
                return;
            }
        }

        out.push((node_idx, indent_level));

        let is_expanded = self.expanded_nodes.contains(&node_idx);
        let has_children = node.is_directory() && node.first_child != NO_INDEX;

        if is_expanded && has_children {
            let mut sorted_child_indices = SmallVec::<[u32; 16]>::new();
            let mut curr = node.first_child;
            while curr != NO_INDEX {
                sorted_child_indices.push(curr);
                curr = snapshot.nodes[curr as usize].next_sibling;
            }
            // Sort immediate children by size descending dynamically for 100% correct tree views
            sorted_child_indices.sort_by(|&a, &b| {
                snapshot.nodes[b as usize]
                    .size
                    .cmp(&snapshot.nodes[a as usize].size)
            });

            for &child_idx in &sorted_child_indices {
                self.flatten_visible_tree(snapshot, child_idx, indent_level + 1, out);
            }
        }
    }

    pub fn render_tree_node_row(
        &mut self,
        ui: &mut egui::Ui,
        snapshot: &FileArenaSnapshot,
        node_idx: u32,
        indent_level: usize,
    ) {
        let node = &snapshot.nodes[node_idx as usize];
        let name = snapshot.string_pool.get(node.name_id).unwrap_or("unknown");

        let is_expanded = self.expanded_nodes.contains(&node_idx);
        let has_children = node.is_directory() && node.first_child != NO_INDEX;
        let is_selected = self.selected_node_idx == Some(node_idx);

        let horizontal_res = ui.horizontal(|ui| {
            // Indent padding
            #[allow(clippy::cast_precision_loss)]
            ui.add_space(indent_level as f32 * 16.0);

            // Icon & Expand Arrow
            let icon_text = if node.is_symlink() {
                "🔗"
            } else if node.is_directory() {
                "📁"
            } else {
                "📄"
            };

            if has_children {
                let arrow = if is_expanded { "[-]" } else { "[+]" };
                let rich_arrow = egui::RichText::new(arrow).monospace();
                let label = ui.selectable_label(is_expanded, rich_arrow);
                if label.clicked() {
                    if is_expanded {
                        self.expanded_nodes.remove(&node_idx);
                    } else {
                        self.expanded_nodes.insert(node_idx);
                    }
                }
            } else {
                ui.add_space(22.0); // Arrow placeholder alignment space matching "[+]"
            }

            ui.label(icon_text);

            // Node Name / Label with automatic left-aligned truncation
            let mut rich_name = egui::RichText::new(name);
            if self.monospace_paths {
                rich_name = rich_name.monospace();
            }
            if is_selected {
                rich_name = rich_name
                    .strong()
                    .color(ui.visuals().selection.stroke.color);
            }

            // Allocate exactly the remaining width minus space for the size column (72px subtracted)
            let name_width = (ui.available_width() - 72.0).max(50.0);

            ui.allocate_ui(egui::vec2(name_width, ui.spacing().interact_size.y), |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.label(rich_name);
            });

            // Muted size details (far right aligned)
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    prettier_bytes::ByteFormatter::new()
                        .format(node.size)
                        .to_string(),
                );
            });
        });

        // Get the bounding box of the whole row
        let rect = horizontal_res.response.rect;

        // --- Offset the interaction hitbox strictly to the right of the expand button ---
        let mut interactive_rect = rect;
        #[allow(clippy::cast_precision_loss)]
        let expand_button_width = (indent_level as f32).mul_add(16.0, 24.0);
        interactive_rect.min.x += expand_button_width;

        let row_id = ui.id().with(("tree_row", node_idx));
        let response = ui.interact(interactive_rect, row_id, egui::Sense::click());

        // Draw professional background selection / hover highlights over the FULL row (for seamless visual style)
        if is_selected {
            let fill_color = ui.visuals().selection.bg_fill.linear_multiply(0.12);
            ui.painter().rect_filled(rect, 4.0, fill_color);
        } else if response.hovered() {
            let hover_color = ui.visuals().widgets.hovered.bg_fill.linear_multiply(0.04);
            ui.painter().rect_filled(rect, 4.0, hover_color);
        }

        // Handle selection on Left-Click or Right-Click (only outside of the expand button)
        if response.clicked() || response.secondary_clicked() {
            self.selected_node_idx = Some(node_idx);
        }

        // Render the context menu on Right-Click
        response.context_menu(|ui| {
            self.draw_file_menu_contents(ui, snapshot);
        });

        // Draw vertical indentation guidelines to visually track nested guidelines
        let painter = ui.painter();
        let stroke = egui::Stroke::new(1.0, theme::INDENT_GUIDELINE);
        for i in 0..indent_level {
            #[allow(clippy::cast_precision_loss)]
            let x = (i as f32).mul_add(16.0, rect.min.x) + 8.0;

            // Draw a dashed vertical line
            let dash_length = 2.0;
            let gap_length = 2.0;
            let step = dash_length + gap_length;
            let total_height = rect.max.y - rect.min.y;
            if total_height > 0.0 {
                let num_steps = (total_height / step).ceil() as usize;
                for step_idx in 0..num_steps {
                    #[allow(clippy::cast_precision_loss)]
                    let segment_y = (step_idx as f32).mul_add(step, rect.min.y);

                    let next_y = (segment_y + dash_length).min(rect.max.y);
                    painter.line_segment([egui::pos2(x, segment_y), egui::pos2(x, next_y)], stroke);
                }
            }
        }
    }
}
