use crate::app::ImageViewerApp;
use crate::ui::common;
use egui::{self, Color32, RichText, Stroke, Vec2};
use std::path::PathBuf;

pub fn render_folders_panel(app: &mut ImageViewerApp, ui: &mut egui::Ui) {
    common::lr_collapsible_panel(ui, "Folders", true, |ui| {
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

pub fn render_folder_node(
    app: &mut ImageViewerApp,
    ui: &mut egui::Ui,
    path: PathBuf,
    depth: usize,
) {
    if depth > 10 {
        return;
    }

    let is_expanded = app.expanded_dirs.contains(&path);
    let is_current_folder = app.current_folder.as_ref() == Some(&path);

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let indent = depth as f32 * 12.0;

    ui.horizontal(|ui| {
        ui.add_space(indent);

        if path.is_dir() {
            let icon = if is_expanded { "▼" } else { "▶" };
            if ui
                .add(
                    egui::Button::new(RichText::new(icon).size(8.0).color(common::LR_TEXT_SECONDARY))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(14.0, 14.0)),
                )
                .clicked()
            {
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
            common::LR_TEXT_SECONDARY
        };

        let bg_color = if is_current_folder {
            Color32::from_rgba_unmultiplied(200, 170, 100, 30)
        } else {
            Color32::TRANSPARENT
        };

        let button_resp = ui.add(
            egui::Button::new(
                RichText::new(format!("📁 {}", name))
                    .size(10.0)
                    .color(folder_color),
            )
            .fill(bg_color)
            .stroke(Stroke::NONE)
            .wrap(),
        );

        if button_resp.clicked() {
            if path.is_dir() {
                app.load_folder(path.clone());
            } else if path.is_file() {
                app.load_image_file(path.clone());
            }
        }

        // Context menu for folders
        if path.is_dir() {
            button_resp.context_menu(|ui| {
                if ui.button("Open Folder").clicked() {
                    app.load_folder(path.clone());
                    ui.close_menu();
                }
            });
        }
    });

    // Render children if expanded
    if is_expanded && path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&path) {
            let mut dirs: Vec<PathBuf> = entries
                .flatten()
                .filter_map(|entry| {
                    let p = entry.path();
                    if p.is_dir()
                        && !p
                            .file_name()
                            .map(|n| n.to_string_lossy().starts_with('.'))
                            .unwrap_or(false)
                    {
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
