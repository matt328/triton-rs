use std::sync::Arc;

use egui_winit_vulkano::egui::load::SizedTexture;
use egui_winit_vulkano::egui::{Context, ImageSource, Visuals};
use egui_winit_vulkano::{egui, Gui};
use vulkano::format::Format;
use vulkano::image::view::ImageView;

/// Example struct to contain the state of the UI
pub struct GuiState {
    show_texture_window1: bool,
    show_texture_window2: bool,
    show_scene_window: bool,
    image_texture_id1: egui::TextureId,
    image_texture_id2: egui::TextureId,
    scene_texture_id: egui::TextureId,
    scene_view_size: [u32; 2],
}

impl GuiState {
    pub fn new(gui: &mut Gui, scene_image: Arc<ImageView>, scene_view_size: [u32; 2]) -> GuiState {
        // tree.png asset is from https://github.com/sotrh/learn-wgpu/tree/master/docs/beginner/tutorial5-textures
        let image_texture_id1 = gui.register_user_image(
            include_bytes!("./assets/tree.png"),
            Format::R8G8B8A8_SRGB,
            Default::default(),
        );
        let image_texture_id2 = gui.register_user_image(
            include_bytes!("./assets/doge2.png"),
            Format::R8G8B8A8_SRGB,
            Default::default(),
        );

        GuiState {
            show_texture_window1: true,
            show_texture_window2: true,
            show_scene_window: true,
            image_texture_id1,
            image_texture_id2,
            scene_texture_id: gui.register_user_image_view(scene_image, Default::default()),
            scene_view_size,
        }
    }

    /// Defines the layout of our UI
    pub fn layout(&mut self, egui_context: Context, window_size: [f32; 2], fps: f32) {
        let GuiState {
            show_texture_window1,
            show_texture_window2,
            show_scene_window,
            image_texture_id1,
            image_texture_id2,
            scene_view_size,
            scene_texture_id,
            ..
        } = self;
        egui_context.set_visuals(Visuals::dark());
        egui::SidePanel::left("Side Panel")
            .default_width(150.0)
            .show(&egui_context, |ui| {
                ui.heading("Hello Tree");
                ui.separator();
                ui.checkbox(show_texture_window1, "Show Tree");
                ui.checkbox(show_texture_window2, "Show Doge");
                ui.checkbox(show_scene_window, "Show Scene");
            });

        egui::Window::new("Mah Tree")
            .resizable(true)
            .vscroll(true)
            .open(show_texture_window1)
            .show(&egui_context, |ui| {
                ui.image(ImageSource::Texture(SizedTexture::new(
                    *image_texture_id1,
                    [256.0, 256.0],
                )));
            });
        egui::Window::new("Mah Doge")
            .resizable(true)
            .vscroll(true)
            .open(show_texture_window2)
            .show(&egui_context, |ui| {
                ui.image(ImageSource::Texture(SizedTexture::new(
                    *image_texture_id2,
                    [300.0, 200.0],
                )));
            });
        egui::Window::new("Scene")
            .resizable(true)
            .vscroll(true)
            .open(show_scene_window)
            .show(&egui_context, |ui| {
                ui.image(ImageSource::Texture(SizedTexture::new(
                    *scene_texture_id,
                    [scene_view_size[0] as f32, scene_view_size[1] as f32],
                )));
            });
        egui::Area::new("fps")
            .fixed_pos(egui::pos2(window_size[0] - 0.05 * window_size[0], 10.0))
            .show(&egui_context, |ui| {
                ui.label(format!("{fps:.2}"));
            });
    }
}
