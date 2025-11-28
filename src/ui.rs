use crate::camera::Camera;
use crate::raytracer::Raytracer;
use egui::Ui;
use glam::Quat;

#[derive(PartialEq)]
pub enum ExplanationTab {
    HowToUse,
    RaytracingVsPathtracing,
    AuthorNote,
}

pub struct UiState {
    pub auto_update: bool,
    pub show_rays: bool,
    pub ray_count: usize,
    pub camera_pitch: f32,
    pub camera_yaw: f32,
    pub explanation_tab: ExplanationTab,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            auto_update: true,
            show_rays: true,
            ray_count: 25,
            camera_pitch: 0.0,
            camera_yaw: 0.0,
            explanation_tab: ExplanationTab::HowToUse,
        }
    }
}

use crate::scene::Scene;
use crate::primitives::{Light, LightType};
use glam::Vec3;

pub fn render_controls(
    ui: &mut Ui,
    ui_state: &mut UiState,
    camera: &mut Camera,
    raytracer: &mut Raytracer,
    scene: &mut Scene,
    trigger_render: &mut bool,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
    ui.heading(egui::RichText::new("Ray- & Pathtracing Visualization").underline());
    ui.heading("Camera Controls");
    
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("Position:");
        changed |= ui.add(egui::DragValue::new(&mut camera.transform.position.x).speed(0.1).prefix("X: ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut camera.transform.position.y).speed(0.1).prefix("Y: ")).changed();
        changed |= ui.add(egui::DragValue::new(&mut camera.transform.position.z).speed(0.1).prefix("Z: ")).changed();
    });

    ui.horizontal(|ui| {
        ui.label("Rotation:");
        if ui.add(egui::DragValue::new(&mut ui_state.camera_pitch).speed(1.0).prefix("Pitch: ").suffix("°")).changed() {
            changed = true;
        }
        if ui.add(egui::DragValue::new(&mut ui_state.camera_yaw).speed(1.0).prefix("Yaw: ").suffix("°")).changed() {
            changed = true;
        }
    });

    // Update camera rotation from pitch/yaw
    if changed {
        let pitch = ui_state.camera_pitch.to_radians();
        let yaw = ui_state.camera_yaw.to_radians();
        camera.transform.rotation = Quat::from_euler(glam::EulerRot::YXZ, yaw, pitch, 0.0);
    }

    if ui.button("Reset View").clicked() {
        camera.look_at(glam::Vec3::ZERO);
        let (yaw, pitch, _) = camera.transform.rotation.to_euler(glam::EulerRot::YXZ);
        ui_state.camera_yaw = yaw.to_degrees();
        ui_state.camera_pitch = pitch.to_degrees();
        *trigger_render = true;
    }

    ui.horizontal(|ui| {
        ui.label("FOV:");
        changed |= ui.add(egui::Slider::new(&mut camera.fov, 10.0..=120.0).suffix("°")).changed();
    });

    ui.separator();
    ui.heading("Raytracer Settings");

    ui.horizontal(|ui| {
        ui.label("Resolution:");
        // Simple resolution scaling
        let mut scale = raytracer.width as f32 / 100.0;
        if ui.add(egui::Slider::new(&mut scale, 0.5..=4.0).text("Scale")).changed() {
            raytracer.width = (100.0 * scale) as u32;
            raytracer.height = (75.0 * scale) as u32;
            *trigger_render = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Mode:");
        egui::ComboBox::from_id_source("render_mode")
            .selected_text(format!("{:?}", raytracer.mode))
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut raytracer.mode, crate::raytracer::RenderMode::Raytracing, "Raytracing").changed() {
                    *trigger_render = true;
                }
                if ui.selectable_value(&mut raytracer.mode, crate::raytracer::RenderMode::Pathtracing, "Pathtracing").changed() {
                    *trigger_render = true;
                }
            });
    });

    if ui.add(egui::Slider::new(&mut raytracer.max_bounces, 0..=10).text("Max Bounces")).changed() {
        *trigger_render = true;
    }

    if ui.add(egui::Slider::new(&mut raytracer.samples_per_pixel, 1..=16).text("Samples/Pixel")).changed() {
        *trigger_render = true;
    }

    ui.checkbox(&mut ui_state.auto_update, "Auto Update");

    if ui.button("Render Now").clicked() {
        *trigger_render = true;
    }

    ui.separator();
    ui.heading("Visualization");

    if ui.checkbox(&mut ui_state.show_rays, "Show Ray Paths").changed() {
        // Trigger render to update paths if we turned it on
        if ui_state.show_rays {
            *trigger_render = true;
        }
    }

    if ui_state.show_rays {
        if ui.add(egui::Slider::new(&mut ui_state.ray_count, 1..=200).text("Ray Count")).changed() {
            *trigger_render = true;
        }
    }

    if changed && ui_state.auto_update {
        *trigger_render = true;
    }

    ui.separator();
    ui.heading("Lighting");

    // Sun Control
    if let Some(sun) = scene.lights.iter_mut().find(|l| l.light_type == LightType::Directional) {
        ui.label("Sun (Directional)");
        let mut sun_changed = false;
        
        // Convert direction to pitch/yaw for intuitive control
        // Direction is normalized vector. 
        // We can just control the vector components directly for now or use angles.
        // Let's use simple sliders for direction components for simplicity and robustness first.
        ui.horizontal(|ui| {
            ui.label("Dir:");
            sun_changed |= ui.add(egui::DragValue::new(&mut sun.direction.x).speed(0.1).prefix("X: ")).changed();
            sun_changed |= ui.add(egui::DragValue::new(&mut sun.direction.y).speed(0.1).prefix("Y: ")).changed();
            sun_changed |= ui.add(egui::DragValue::new(&mut sun.direction.z).speed(0.1).prefix("Z: ")).changed();
        });
        if sun_changed {
            sun.direction = sun.direction.normalize();
            *trigger_render = true;
        }

        ui.horizontal(|ui| {
            ui.label("Intensity:");
            if ui.add(egui::Slider::new(&mut sun.intensity, 0.0..=5.0)).changed() {
                *trigger_render = true;
            }
        });
        
        ui.horizontal(|ui| {
            ui.label("Color:");
            let mut rgb = [sun.color.x, sun.color.y, sun.color.z];
            if ui.color_edit_button_rgb(&mut rgb).changed() {
                sun.color = Vec3::from_array(rgb);
                *trigger_render = true;
            }
        });
    }

    ui.separator();
    ui.label("Point Lights");
    
    if ui.button("Add Point Light").clicked() {
        scene.lights.push(Light {
            light_type: LightType::Point,
            position: Vec3::new(0.0, 2.0, 0.0),
            direction: Vec3::ZERO,
            color: Vec3::new(1.0, 1.0, 1.0),
            intensity: 1.0,
        });
        *trigger_render = true;
    }

    let mut remove_index = None;
    for (i, light) in scene.lights.iter_mut().enumerate() {
        if light.light_type == LightType::Point {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("Light {}", i));
                    if ui.button("Remove").clicked() {
                        remove_index = Some(i);
                    }
                });

                let mut light_changed = false;
                ui.horizontal(|ui| {
                    ui.label("Pos:");
                    light_changed |= ui.add(egui::DragValue::new(&mut light.position.x).speed(0.1).prefix("X: ")).changed();
                    light_changed |= ui.add(egui::DragValue::new(&mut light.position.y).speed(0.1).prefix("Y: ")).changed();
                    light_changed |= ui.add(egui::DragValue::new(&mut light.position.z).speed(0.1).prefix("Z: ")).changed();
                });

                ui.horizontal(|ui| {
                    ui.label("Int:");
                    light_changed |= ui.add(egui::Slider::new(&mut light.intensity, 0.0..=5.0)).changed();
                });

                ui.horizontal(|ui| {
                    ui.label("Col:");
                    let mut rgb = [light.color.x, light.color.y, light.color.z];
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        light.color = Vec3::from_array(rgb);
                        light_changed = true;
                    }
                });

                if light_changed {
                    *trigger_render = true;
                }
            });
        }
    }

    if let Some(index) = remove_index {
        scene.lights.remove(index);
        *trigger_render = true;
    }
    });
}

pub fn render_explanation(ui: &mut Ui, ui_state: &mut UiState) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.explanation_tab, ExplanationTab::HowToUse, "How to Use");
        ui.selectable_value(&mut ui_state.explanation_tab, ExplanationTab::RaytracingVsPathtracing, "Raytracing vs Pathtracing");
        ui.selectable_value(&mut ui_state.explanation_tab, ExplanationTab::AuthorNote, "Author's note");
    });
    
    ui.separator();
    
    egui::ScrollArea::vertical().show(ui, |ui| {
        match ui_state.explanation_tab {
            ExplanationTab::HowToUse => {
                ui.heading("How to Use");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("3D View (Left)").underline());
                ui.label("• Right-click + drag to rotate the camera");
                ui.label("• Scroll to zoom in/out");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Rendered View (Right)").underline());
                ui.label("• Shows the raytraced/pathtraced output");
                ui.label("• Adjust camera position/rotation in the controls");
                ui.label("• Click 'Reset View' to look at origin");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Ray Visualization").underline());
                ui.label("• Enable 'Show Ray Paths' to see rays in 3D view");
                ui.label("• Yellow: Primary rays from camera");
                ui.label("• Cyan: Reflections (mirrors, metals)");
                ui.label("• Magenta: Refractions (through glass)");
                ui.label("• Light Blue: Diffuse scattering");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Lighting").underline());
                ui.label("• Add/remove point lights dynamically");
                ui.label("• Adjust light position, color, and intensity");
            }
            ExplanationTab::RaytracingVsPathtracing => {
                ui.heading("Raytracing vs Pathtracing");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Raytracing (Whitted-Style)").underline());
                ui.label("• Fast and deterministic");
                ui.label("• Perfect reflections and refractions");
                ui.label("• Sharp, hard shadows");
                ui.label("• Direct lighting only (no global illumination)");
                ui.label("• Best for: Real-time previews, mirrors, glass");
                ui.add_space(10.0);
    
                ui.label(egui::RichText::new("Path Tracing (Monte Carlo)").underline());
                ui.label("• Physically accurate but slower");
                ui.label("• Soft shadows and color bleeding");
                ui.label("• Global illumination (light bounces everywhere)");
                ui.label("• Noisy with few samples (1 sample per pixel here)");
                ui.label("• Rough materials look more realistic");
                ui.label("• Best for: Photorealistic renders, complex lighting");
                ui.add_space(10.0);
    
                ui.label(egui::RichText::new("Key Difference").underline());
                ui.label("Raytracing traces specific rays (to lights, reflections).");
                ui.label("Path tracing randomly samples all directions, simulating");
                ui.label("how light actually behaves in the real world.");
            }
            ExplanationTab::AuthorNote => {
                ui.heading("Author's note");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Purpose").underline());
                ui.label("Hi! I'm Rasmus Hogslätt, the creator of this raytracing/pathtracing demo.");
                ui.label("I'm a software engineer interested in graphics programming and rendering techniques. As the use of AI-agents for coding become more prevalent, I created this demo to explore how to use them myself, a field that I am already familiar with.");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Thoughts on Agents").underline());
                ui.label("Google's Antigravity with the free access to Gemini Pro 3 was used.");
                ui.label("• Common boilerplate, like setting up window, UI, graphics pipeline was setup quickly. I used WGPU, which had a breaking change. I had to search for this and provide the link with the up to date docs to the agent.");
                ui.label("• It was easy to iterate on features so that I could make a decision on what to keep or remove.");
                ui.label("• It's important to specify the features I want, don't want, and what features I might want in the future to allow reusabilty of code. For example, I initially just started with Whitted raytracing, and had to specify I wanted the code to be extendable for pathtracing later.");
                ui.label("• It is important to occasionally pause, manually review, and have the agent study the codebase and refactor common patterns before the agent had added too much code to manage.");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Takeaway").underline());
                ui.label("• You need to know what you want. I did not care about the graphics pipeline setup, but I cared about getting a raytracer/pathtracer that I can experiment with. If I cared to have a more customizable graphics pipeline, I would need to specify that.");
                ui.label("• Afterall, specificity seems to be the key to achieve my wanted outcome.");
                ui.add_space(5.0);

                ui.label(egui::RichText::new("Rust 🦀").underline());
                ui.label("Of course, the project was done in Rust and compiled to WebAssembly because I like crabs 🦀🦀🦀...");
                ui.add_space(5.0);
    
                ui.label(egui::RichText::new("Contact").underline());
                ui.label("• If you want to contact me for any reason, consulting or otherwise, feel free to reach out at r.hogslatt@gmail.com");
            },
        }
    });
}

