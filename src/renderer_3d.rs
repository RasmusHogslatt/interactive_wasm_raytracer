use crate::camera::Camera;
use crate::scene::Scene;
use crate::raytracer::RayPath;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
}

pub struct Renderer3D {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

impl Renderer3D {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some("depth_texture"),
            view_formats: &[],
        });
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
            depth_texture,
            depth_view,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.depth_texture = device.create_texture(&wgpu::TextureDescriptor {
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                label: Some("depth_texture"),
                view_formats: &[],
            });
            self.depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        scene: &Scene,
        camera: &Camera,
        ray_paths: &[RayPath],
        view_camera: &Camera, // The camera we are looking THROUGH to see the 3D scene
    ) {
        // Update uniforms
        let view_proj = view_camera.projection_matrix() * view_camera.view_matrix();
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[Uniforms {
            view_proj: view_proj.to_cols_array_2d(),
        }]));

        // Generate geometry
        let mut vertices: Vec<Vertex> = Vec::new();

        // Grid
        let grid_color = [0.2, 0.2, 0.2];
        for i in -10..=10 {
            let x = i as f32;
            vertices.push(Vertex { position: [x, 0.0, -10.0], color: grid_color });
            vertices.push(Vertex { position: [x, 0.0, 10.0], color: grid_color });
            vertices.push(Vertex { position: [-10.0, 0.0, x], color: grid_color });
            vertices.push(Vertex { position: [10.0, 0.0, x], color: grid_color });
        }

        // Scene Objects
        for sphere in &scene.spheres {
            self.add_sphere_wireframe(&mut vertices, sphere.center, sphere.radius, sphere.material.color.into());
        }
        for cube in &scene.cubes {
            self.add_cube_wireframe(&mut vertices, cube.min, cube.max, cube.material.color.into());
        }
        for _plane in &scene.planes {
            // Visualize plane as a large quad
            // TODO: Better plane visualization
        }
        for light in &scene.lights {
            self.add_sphere_wireframe(&mut vertices, light.position, 0.2, light.color.into());
        }

        // Camera Frustum
        self.add_camera_frustum(&mut vertices, camera);

        // Ray Paths - Color coded by segment type
        for path in ray_paths {
            if path.points.len() < 2 { continue; }
            for i in 0..path.points.len() - 1 {
                let color = if i < path.segment_types.len() {
                    match path.segment_types[i] {
                        crate::raytracer::RaySegmentType::Primary => [1.0, 1.0, 0.0],      // Yellow - primary ray
                        crate::raytracer::RaySegmentType::Reflection => [0.0, 1.0, 1.0],   // Cyan - reflection
                        crate::raytracer::RaySegmentType::Refraction => [1.0, 0.0, 1.0],   // Magenta - refraction
                        crate::raytracer::RaySegmentType::Diffuse => [0.5, 0.5, 1.0],      // Light blue - diffuse
                    }
                } else {
                    [1.0, 1.0, 0.0] // Fallback to yellow
                };
                vertices.push(Vertex { position: path.points[i].into(), color });
                vertices.push(Vertex { position: path.points[i+1].into(), color });
            }
        }

        if vertices.is_empty() {
            return;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.05,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            render_pass.draw(0..vertices.len() as u32, 0..1);
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    fn add_sphere_wireframe(&self, vertices: &mut Vec<Vertex>, center: Vec3, radius: f32, color: [f32; 3]) {
        let segments = 16;
        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * std::f32::consts::TAU;
            let angle2 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
            
            // XY circle
            vertices.push(Vertex { position: (center + Vec3::new(angle1.cos() * radius, angle1.sin() * radius, 0.0)).into(), color });
            vertices.push(Vertex { position: (center + Vec3::new(angle2.cos() * radius, angle2.sin() * radius, 0.0)).into(), color });
            
            // XZ circle
            vertices.push(Vertex { position: (center + Vec3::new(angle1.cos() * radius, 0.0, angle1.sin() * radius)).into(), color });
            vertices.push(Vertex { position: (center + Vec3::new(angle2.cos() * radius, 0.0, angle2.sin() * radius)).into(), color });
            
            // YZ circle
            vertices.push(Vertex { position: (center + Vec3::new(0.0, angle1.cos() * radius, angle1.sin() * radius)).into(), color });
            vertices.push(Vertex { position: (center + Vec3::new(0.0, angle2.cos() * radius, angle2.sin() * radius)).into(), color });
        }
    }

    fn add_cube_wireframe(&self, vertices: &mut Vec<Vertex>, min: Vec3, max: Vec3, color: [f32; 3]) {
        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ];

        let edges = [
            (0, 1), (1, 2), (2, 3), (3, 0), // Front face
            (4, 5), (5, 6), (6, 7), (7, 4), // Back face
            (0, 4), (1, 5), (2, 6), (3, 7), // Connecting edges
        ];

        for (start, end) in edges {
            vertices.push(Vertex { position: corners[start].into(), color });
            vertices.push(Vertex { position: corners[end].into(), color });
        }
    }

    fn add_camera_frustum(&self, vertices: &mut Vec<Vertex>, camera: &Camera) {
        let color = [0.0, 1.0, 0.0]; // Green camera
        let pos = camera.transform.position;
        
        // Draw camera position
        self.add_sphere_wireframe(vertices, pos, 0.1, color);

        // Draw frustum cone
        let forward = camera.transform.forward();
        let right = camera.transform.right();
        let up = camera.transform.up();
        
        let dist = 1.0;
        let h = (camera.fov.to_radians() / 2.0).tan() * dist;
        let w = h * camera.aspect_ratio;
        
        let tl = pos + forward * dist - right * w + up * h;
        let tr = pos + forward * dist + right * w + up * h;
        let bl = pos + forward * dist - right * w - up * h;
        let br = pos + forward * dist + right * w - up * h;

        // Lines from eye to corners
        vertices.push(Vertex { position: pos.into(), color }); vertices.push(Vertex { position: tl.into(), color });
        vertices.push(Vertex { position: pos.into(), color }); vertices.push(Vertex { position: tr.into(), color });
        vertices.push(Vertex { position: pos.into(), color }); vertices.push(Vertex { position: bl.into(), color });
        vertices.push(Vertex { position: pos.into(), color }); vertices.push(Vertex { position: br.into(), color });

        // Rectangle at distance
        vertices.push(Vertex { position: tl.into(), color }); vertices.push(Vertex { position: tr.into(), color });
        vertices.push(Vertex { position: tr.into(), color }); vertices.push(Vertex { position: br.into(), color });
        vertices.push(Vertex { position: br.into(), color }); vertices.push(Vertex { position: bl.into(), color });
        vertices.push(Vertex { position: bl.into(), color }); vertices.push(Vertex { position: tl.into(), color });
    }
}
