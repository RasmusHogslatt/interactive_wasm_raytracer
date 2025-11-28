mod app;
mod camera;
mod math;
mod primitives;
mod raytracer;
mod renderer_3d;
mod scene;
mod ui;

use app::RaytracerApp;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init();
    
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "Interactive WASM Raytracer",
        native_options,
        Box::new(|cc| {
            crate::apply_custom_style(&cc.egui_ctx);
            Ok(Box::new(RaytracerApp::new(cc)))
        }),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("no global `window` exists")
            .document()
            .expect("should have a document on window");

        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("document should have an element with id 'the_canvas_id'")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("element should be an HtmlCanvasElement");

        eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    crate::apply_custom_style(&cc.egui_ctx);
                    Ok(Box::new(RaytracerApp::new(cc)))
                }),
            )
            .await
            .expect("failed to start eframe");
    });
}

pub fn apply_custom_style(ctx: &egui::Context) {
    use egui::{Color32, Rounding, Stroke, Visuals};
    let mut fonts = egui::FontDefinitions::default();
    
    // Add emoji support by prioritizing system fonts with emoji
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("emoji-icon-font".to_owned());
    
    fonts.font_data.insert(
        "emoji-icon-font".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/NotoEmoji-Regular.ttf"))
            .tweak(egui::FontTweak {
                scale: 1.0,
                y_offset_factor: 0.0,
                y_offset: 0.0,
                baseline_offset_factor: 0.0,
            }).into(),
    );
    
    ctx.set_fonts(fonts);
    let mut style = (*ctx.style()).clone();
    
    // Custom colors
    let orange_primary = Color32::from_rgb(0xFE, 0x58, 0x00);  // #FE5800
    let orange_bright = Color32::from_rgb(0xFF, 0x73, 0x00);   // #FF7300
    let gray = Color32::from_rgb(0x67, 0x67, 0x67);            // #676767
    let dark_bg = Color32::from_rgb(0x1a, 0x1a, 0x1a);
    let darker_bg = Color32::from_rgb(0x0f, 0x0f, 0x0f);
    
    // Set up dark theme with custom colors
    let mut visuals = Visuals::dark();
    
    // Background colors
    visuals.panel_fill = dark_bg;
    visuals.window_fill = dark_bg;
    visuals.extreme_bg_color = darker_bg;
    
    // Widget colors
    visuals.widgets.noninteractive.bg_fill = gray;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    
    visuals.widgets.inactive.bg_fill = gray;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.inactive.weak_bg_fill = gray;
    
    visuals.widgets.hovered.bg_fill = orange_primary;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    visuals.widgets.hovered.weak_bg_fill = orange_primary;
    
    visuals.widgets.active.bg_fill = orange_bright;
    visuals.widgets.active.fg_stroke = Stroke::new(2.0, Color32::WHITE);
    visuals.widgets.active.weak_bg_fill = orange_bright;
    
    visuals.widgets.open.bg_fill = orange_bright;
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.open.weak_bg_fill = orange_bright;
    
    // Selection colors
    visuals.selection.bg_fill = orange_primary;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    
    // Hyperlink color
    visuals.hyperlink_color = orange_bright;
    
    // Window and panel styling
    visuals.window_rounding = Rounding::same(8.0);
    visuals.window_stroke = Stroke::new(1.0, gray);
    
    // Apply rounding to widgets
    style.visuals = visuals;
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.slider_width = 180.0;
    
    ctx.set_style(style);
}
