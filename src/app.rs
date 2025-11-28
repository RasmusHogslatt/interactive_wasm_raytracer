use crate::camera::Camera;
use crate::raytracer::{Raytracer, RayPath};
use crate::renderer_3d::Renderer3D;
use crate::scene::Scene;
use crate::ui::{UiState, render_controls};
use eframe::egui;
use glam::{Vec3, Quat};


pub struct RaytracerApp {
    scene: Scene,
    camera: Camera, // The camera used for raytracing
    view_camera: Camera, // The camera used to view the 3D scene
    raytracer: Raytracer,
    renderer_3d: Renderer3D,
    ui_state: UiState,
    
    raytraced_texture: Option<egui::TextureHandle>,
    ray_paths: Vec<RayPath>,
    
    // 3D View Texture
    view_texture: Option<wgpu::Texture>,
    view_texture_view: Option<wgpu::TextureView>,
    view_texture_id: Option<egui::TextureId>,
    view_width: u32,
    view_height: u32,
}

impl RaytracerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let wgpu_render_state = cc.wgpu_render_state.as_ref().expect("WGPU enabled");
        let device = &wgpu_render_state.device;
        
        let format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let renderer_3d = Renderer3D::new(device, format);

        let mut camera = Camera::default();
        camera.transform.position = Vec3::new(0.0, 2.5, 6.0);
        camera.look_at(Vec3::ZERO);

        let mut view_camera = Camera::new(
            Vec3::new(10.0, 10.0, 10.0),
            Vec3::ZERO,
            60.0,
            16.0 / 9.0,
        );
        view_camera.look_at(Vec3::ZERO);

        let raytracer = Raytracer::default();
        let scene = Scene::default();

        let mut ui_state = UiState::default();
        let (yaw, pitch, _) = camera.transform.rotation.to_euler(glam::EulerRot::YXZ);
        ui_state.camera_yaw = yaw.to_degrees();
        ui_state.camera_pitch = pitch.to_degrees();

        let mut app = Self {
            scene,
            camera,
            view_camera,
            raytracer,
            renderer_3d,
            ui_state,
            raytraced_texture: None,
            ray_paths: Vec::new(),
            view_texture: None,
            view_texture_view: None,
            view_texture_id: None,
            view_width: 0,
            view_height: 0,
        };
        
        app.update_raytrace(cc.egui_ctx.clone());
        
        app
    }

    fn update_raytrace(&mut self, ctx: egui::Context) {
        let pixels = self.raytracer.render(&self.scene, &self.camera);
        
        let image = egui::ColorImage::from_rgba_unmultiplied(
            [self.raytracer.width as usize, self.raytracer.height as usize],
            &pixels,
        );

        self.raytraced_texture = Some(ctx.load_texture(
            "raytraced_output",
            image,
            egui::TextureOptions::NEAREST,
        ));

        if self.ui_state.show_rays {
            self.ray_paths = self.raytracer.trace_paths(&self.scene, &self.camera, self.ui_state.ray_count);
        } else {
            self.ray_paths.clear();
        }
    }

    fn update_3d_view(&mut self, frame: &mut eframe::Frame, width: u32, height: u32) {
        if width == 0 || height == 0 { return; }

        let wgpu_state = if let Some(state) = frame.wgpu_render_state() {
            state
        } else {
            return;
        };
        let device = &wgpu_state.device;
        let queue = &wgpu_state.queue;

        // Resize if needed
        if self.view_width != width || self.view_height != height || self.view_texture.is_none() {
            // Free old texture ID if exists
            if let Some(id) = self.view_texture_id {
                wgpu_state.renderer.write().free_texture(&id);
                self.view_texture_id = None;
            }

            self.view_width = width;
            self.view_height = height;
            
            let texture_desc = wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                label: Some("3d_view_texture"),
                view_formats: &[],
            };
            
            let texture = device.create_texture(&texture_desc);
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            
            // Register new texture
            let texture_id = wgpu_state.renderer.write().register_native_texture(
                device,
                &view,
                wgpu::FilterMode::Linear
            );

            self.view_texture = Some(texture);
            self.view_texture_view = Some(view);
            self.view_texture_id = Some(texture_id);
            
            self.renderer_3d.resize(device, width, height);
            self.view_camera.aspect_ratio = width as f32 / height as f32;
        }

        // Render
        if let Some(view) = &self.view_texture_view {
            self.renderer_3d.render(
                device,
                queue,
                view,
                &self.scene,
                &self.camera,
                &self.ray_paths,
                &self.view_camera,
            );
        }
    }
}

impl eframe::App for RaytracerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Apply custom styling
        crate::apply_custom_style(ctx);
        
        let mut trigger_render = false;

        egui::SidePanel::left("controls_panel").show(ctx, |ui| {
            render_controls(
                ui, 
                &mut self.ui_state, 
                &mut self.camera, 
                &mut self.raytracer,
                &mut self.scene,
                &mut trigger_render
            );
            
            ui.separator();
            ui.label("3D View Controls:");
            ui.label("Right-click + Drag to rotate");
            ui.label("Scroll to zoom");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                // 3D Scene View (Left Column)
                let ui_left = &mut columns[0];
                let view_size = ui_left.available_size();
                
                // Update texture size and render
                self.update_3d_view(frame, view_size.x as u32, view_size.y as u32);
                
                let response = if let Some(texture_id) = self.view_texture_id {
                    ui_left.add(egui::Image::new(egui::load::SizedTexture::new(texture_id, view_size))
                        .sense(egui::Sense::drag()))
                } else {
                    ui_left.allocate_response(view_size, egui::Sense::drag())
                };

                // Handle 3D view input
                if response.dragged_by(egui::PointerButton::Secondary) {
                    let delta = response.drag_delta();
                    let sensitivity = 0.01;
                    
                    let yaw = Quat::from_rotation_y(-delta.x * sensitivity);
                    let pitch = Quat::from_axis_angle(self.view_camera.transform.right(), -delta.y * sensitivity);
                    
                    self.view_camera.transform.position = yaw * pitch * self.view_camera.transform.position;
                    self.view_camera.look_at(Vec3::ZERO);
                }
                
                if response.hovered() {
                    let zoom_delta = ctx.input(|i| i.raw_scroll_delta.y);
                    if zoom_delta != 0.0 {
                        let forward = self.view_camera.transform.forward();
                        self.view_camera.transform.position += forward * zoom_delta * 0.01;
                    }
                }

                // Raytraced View (Right Column)
                let ui_right = &mut columns[1];
                
                ui_right.vertical(|ui| {
                    // Rendered image
                    let available_height = ui.available_height();
                    let image_height = available_height * 0.6; // 60% for image
                    let image_size = egui::vec2(ui.available_width(), image_height);
                    
                    if let Some(texture) = &self.raytraced_texture {
                        ui.add(egui::Image::new(texture).fit_to_exact_size(image_size));
                    } else {
                        ui.allocate_space(image_size);
                        ui.label("Rendering...");
                    }
                    
                    ui.separator();
                    
                    // Explanation panel
                    crate::ui::render_explanation(ui, &mut self.ui_state);
                });
            });
        });

        if trigger_render {
            self.update_raytrace(ctx.clone());
        }
    }
}
