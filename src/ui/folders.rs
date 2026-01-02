use crate::app::ImageViewerApp;
use std::path::PathBuf;
use egui::{self, Color32, RichText, Vec2, Rounding, Stroke, Rect};

// Lightroom-inspired color scheme
const LR_BG_PANEL: Color32 = Color32::from_rgb(51, 51, 51);
const LR_BORDER: Color32 = Color32::from_rgb(28, 28, 28);
const LR_TEXT_PRIMARY: Color32 = Color32::from_rgb(200, 200, 200);
const LR_TEXT_SECONDARY: Color32 = Color32::from_rgb(140, 140, 140);
const LR_HEADER_BG: Color32 = Color32::from_rgb(45, 45, 45);

pub fn render_folders_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    lr_collapsible_panel(ui, "Folders", true, |ui| {
        egui::ScrollArea::vertical()
            .id_salt("folder_tree")
            .auto_shrink([false, true])
            .max_height(300.0)
            .show(ui, |ui| {
                #[cfg(windows)]
                {
                    use std::path::Path;
                    for drive in b'A'..=b'Z' {
                        let drive_str = format!("{}:\\", drive as char);
                        let drive_path = Path::new(&drive_str);
                        if drive_path.exists() {
                            render_folder_node(app, ui, drive_path.to_path_buf(), 0);
                        }
                    }
                }

                #[cfg(unix)]
                {
                    render_folder_node(app, ui, PathBuf::from("/"), 0);
                }

                #[cfg(not(any(windows, unix)))]
                {
                    if let Ok(current) = std::env::current_dir() {
                        if let Some(parent) = current.parent() {
                            render_folder_node(app, ui, parent.to_path_buf(), 0);
                        }
                    }
                }
            });
    });
}

pub fn render_folder_node(app: &mut ImageViewerApp, ui: &mut egui::Ui, path: PathBuf, depth: usize) {
    if depth > 10 {
        return;
    }

    let is_expanded = app.expanded_dirs.contains(&path);
    let is_current_folder = app.current_folder.as_ref() == Some(&path);

    let name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let indent = depth as f32 * 12.0;

    ui.horizontal(|ui| {
        ui.add_space(indent);

        // Expand triangle (like Lightroom)
        if path.is_dir() {
            let icon = if is_expanded { "▼" } else { "▶" };
            if ui.add(egui::Button::new(RichText::new(icon).size(8.0).color(LR_TEXT_SECONDARY))
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .min_size(Vec2::new(14.0, 14.0)))
                .clicked() {
                if is_expanded {
                    app.expanded_dirs.remove(&path);
                } else {
                    app.expanded_dirs.insert(path.clone());
                }
            }
        } else {
            ui.add_space(14.0);
        }

        // Folder icon and name
        let folder_color = if is_current_folder {
            Color32::from_rgb(200, 170, 100)
        } else {
            LR_TEXT_SECONDARY
        };

        let bg_color = if is_current_folder {
            Color32::from_rgba_unmultiplied(200, 170, 100, 30)
        } else {
            Color32::TRANSPARENT
        };

        if ui.add(egui::Button::new(RichText::new(format!("📁 {}", name)).size(10.0).color(folder_color))
            .fill(bg_color)
            .stroke(Stroke::NONE)
            .wrap())
            .clicked() {
            if path.is_dir() {
                app.load_folder(path.clone());
            } else if path.is_file() {
                app.load_image_file(path.clone());
            }
        }
    });

    // Render children if expanded
    if is_expanded && path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            let mut dirs: Vec<PathBuf> = entries
                .flatten()
                .filter_map(|entry| {
                    let p = entry.path();
                    if p.is_dir() && !p.file_name()
                        .map(|n| n.to_string_lossy().starts_with('.'))
                        .unwrap_or(false) {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect();

            dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

            for dir_path in dirs {
                render_folder_node(app, ui, dir_path, depth + 1);
            }
        }
    }
}

// Lightroom-style collapsible panel
fn lr_collapsible_panel<R>(
    ui: &mut egui::Ui,
    title: &str,
    default_open: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::CollapsingResponse<R> {
    // Panel header background
    let header_rect = ui.available_rect_before_wrap();
    let header_rect = Rect::from_min_size(
        header_rect.min,
        Vec2::new(ui.available_width(), 24.0)
    );

    ui.painter().rect_filled(header_rect, Rounding::ZERO, LR_HEADER_BG);
    ui.painter().hline(
        header_rect.x_range(),
        header_rect.bottom(),
        Stroke::new(1.0, LR_BORDER)
    );

    let response = egui::CollapsingHeader::new(RichText::new(title).size(11.0).color(LR_TEXT_PRIMARY).strong())
        .default_open(default_open)
        .show(ui, |ui| {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(LR_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                .show(ui, |ui| {
                add_contents(ui)
            }).inner
        });

    // Bottom border
    ui.painter().hline(
        ui.available_rect_before_wrap().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, LR_BORDER)
    );

    response
}