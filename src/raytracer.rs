use crate::camera::Camera;
use crate::math::{Ray, random_unit_vector, reflect, refract, reflectance};
use crate::primitives::MaterialType;
use crate::scene::Scene;
use glam::Vec3;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RenderMode {
    Raytracing,
    Pathtracing,
}

pub struct Raytracer {
    pub width: u32,
    pub height: u32,
    pub max_bounces: u32,
    pub samples_per_pixel: u32,
    pub mode: RenderMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RaySegmentType {
    Primary,      // Initial ray from camera
    Reflection,   // Reflected ray
    Refraction,   // Refracted ray (through glass)
    Diffuse,      // Diffuse scatter
}

#[derive(Clone, Debug)]
pub struct RayPath {
    pub points: Vec<Vec3>,
    pub segment_types: Vec<RaySegmentType>, // Type of each segment (length = points.len() - 1)
    pub hit: bool,
}

impl Default for Raytracer {
    fn default() -> Self {
        Self {
            width: 200,
            height: 150,
            max_bounces: 3,
            samples_per_pixel: 1,
            mode: RenderMode::Raytracing,
        }
    }
}

impl Raytracer {
    pub fn render(&self, scene: &Scene, camera: &Camera) -> Vec<u8> {
        let mut buffer = vec![0; (self.width * self.height * 4) as usize];
        let mut rng = rand::thread_rng();

        for y in 0..self.height {
            for x in 0..self.width {
                let mut color = Vec3::ZERO;
                
                // Multi-sampling with random jittering
                for _ in 0..self.samples_per_pixel {
                    let random_u: f32 = rng.gen();
                    let random_v: f32 = rng.gen();
                    
                    let u = (x as f32 + random_u) / self.width as f32;
                    let v = 1.0 - (y as f32 + random_v) / self.height as f32; // Flip Y
                    
                    let ray = camera.get_ray(u, v);
                    
                    match self.mode {
                        RenderMode::Raytracing => {
                            color += self.trace_ray(ray, scene, self.max_bounces);
                        }
                        RenderMode::Pathtracing => {
                            color += self.trace_pathtrace(ray, scene, self.max_bounces);
                        }
                    }
                }
                
                // Average the samples
                color /= self.samples_per_pixel as f32;

                // Clamp and convert to RGBA
                let r = (color.x.clamp(0.0, 1.0) * 255.0) as u8;
                let g = (color.y.clamp(0.0, 1.0) * 255.0) as u8;
                let b = (color.z.clamp(0.0, 1.0) * 255.0) as u8;

                let index = ((y * self.width + x) * 4) as usize;
                buffer[index] = r;
                buffer[index + 1] = g;
                buffer[index + 2] = b;
                buffer[index + 3] = 255;
            }
        }

        buffer
    }

    pub fn trace_ray(&self, ray: Ray, scene: &Scene, depth: u32) -> Vec3 {
        if depth == 0 {
            return Vec3::ZERO;
        }

        if let Some(hit) = scene.intersect(&ray, 0.001, f32::INFINITY) {
            let mut color = Vec3::ZERO;
            let view_dir = -ray.direction;

            // Ambient
            color += hit.material.color * 0.1;

            // Diffuse and Specular
            for light in &scene.lights {
                let (light_dir, distance) = match light.light_type {
                    crate::primitives::LightType::Directional => (-light.direction.normalize(), f32::INFINITY),
                    crate::primitives::LightType::Point => {
                        let dir = light.position - hit.point;
                        (dir.normalize(), dir.length())
                    }
                };
                
                // Shadow ray
                let shadow_ray = Ray::new(hit.point, light_dir);
                if scene.intersect(&shadow_ray, 0.001, distance).is_none() {
                    // Diffuse
                    let diff = hit.normal.dot(light_dir).max(0.0);
                    color += hit.material.color * light.color * diff * light.intensity;

                    // Specular
                    let reflect_dir = (-light_dir).reflect(hit.normal);
                    let spec = view_dir.dot(reflect_dir).max(0.0).powf(hit.material.shininess);
                    color += light.color * hit.material.specular * spec * light.intensity;
                }
            }

            // Reflection (Whitted style)
            if hit.material.reflectivity > 0.0 {
                let reflected_ray = Ray::new(hit.point, ray.direction.reflect(hit.normal));
                color += self.trace_ray(reflected_ray, scene, depth - 1) * hit.material.reflectivity;
            }
            
            // Refraction (Whitted style) - Basic implementation for Dielectric
            if hit.material.mat_type == MaterialType::Dielectric {
                let unit_direction = ray.direction.normalize();
                let dot = unit_direction.dot(hit.normal);
                let (normal, refraction_ratio) = if dot < 0.0 {
                    (hit.normal, 1.0 / hit.material.ior)
                } else {
                    (-hit.normal, hit.material.ior)
                };

                let cos_theta = (-unit_direction).dot(normal).min(1.0);
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

                let cannot_refract = refraction_ratio * sin_theta > 1.0;
                let direction;

                if cannot_refract || reflectance(cos_theta, refraction_ratio) > rand::thread_rng().gen() {
                    direction = unit_direction.reflect(normal);
                } else {
                    direction = refract(unit_direction, normal, refraction_ratio);
                }
                
                let refracted_ray = Ray::new(hit.point, direction);
                return self.trace_ray(refracted_ray, scene, depth - 1);
            }

            color
        } else {
            // Background color (sky gradient)
            let unit_direction = ray.direction.normalize();
            let t = 0.5 * (unit_direction.y + 1.0);
            Vec3::new(1.0, 1.0, 1.0) * (1.0 - t) + Vec3::new(0.5, 0.7, 1.0) * t
        }
    }

    pub fn trace_pathtrace(&self, ray: Ray, scene: &Scene, depth: u32) -> Vec3 {
        if depth == 0 {
            return Vec3::ZERO;
        }

        if let Some(hit) = scene.intersect(&ray, 0.001, f32::INFINITY) {
            let mut direct_light = Vec3::ZERO;
            
            // 1. Direct Lighting (Next Event Estimation)
            // We explicitly sample lights for non-specular materials.
            // For perfect specular (Glass, Mirror), the probability of hitting a point light is 0,
            // so we rely entirely on the recursive ray.
            
            let is_specular = match hit.material.mat_type {
                MaterialType::Dielectric => true,
                MaterialType::Metal => hit.material.roughness < 0.05, // Treat very smooth metal as specular
                MaterialType::Lambertian => false,
            };

            if !is_specular {
                for light in &scene.lights {
                    let (light_dir, distance) = match light.light_type {
                        crate::primitives::LightType::Directional => (-light.direction.normalize(), f32::INFINITY),
                        crate::primitives::LightType::Point => {
                            let dir = light.position - hit.point;
                            (dir.normalize(), dir.length())
                        }
                    };

                    // Shadow ray
                    let shadow_ray = Ray::new(hit.point, light_dir);
                    if scene.intersect(&shadow_ray, 0.001, distance).is_none() {
                        let cos_theta = hit.normal.dot(light_dir).max(0.0);
                        
                        if hit.material.mat_type == MaterialType::Lambertian {
                            // Diffuse: color * light * cos_theta
                            // We assume light intensity handles falloff/energy
                            direct_light += hit.material.color * light.color * light.intensity * cos_theta;
                        } else if hit.material.mat_type == MaterialType::Metal {
                            // Rough Metal: Specular highlight
                            // Simple Blinn-Phong-like approximation for direct light on rough metal
                            let view_dir = -ray.direction.normalize();
                            let halfway = (light_dir + view_dir).normalize();
                            let spec = hit.normal.dot(halfway).max(0.0).powf(2.0 / hit.material.roughness.max(0.01));
                            direct_light += light.color * hit.material.color * light.intensity * spec;
                        }
                    }
                }
            }

            // 2. Indirect Lighting (Recursive Ray)
            let scatter_direction;
            let attenuation;

            match hit.material.mat_type {
                MaterialType::Lambertian => {
                    // Cosine weighted sampling
                    scatter_direction = (hit.normal + random_unit_vector()).normalize();
                    attenuation = hit.material.color;
                }
                MaterialType::Metal => {
                    let reflected = ray.direction.normalize().reflect(hit.normal);
                    scatter_direction = reflected + random_unit_vector() * hit.material.roughness;
                    attenuation = hit.material.color;
                    
                    if scatter_direction.dot(hit.normal) <= 0.0 {
                        return direct_light; // Absorbed
                    }
                }
                MaterialType::Dielectric => {
                    attenuation = Vec3::ONE;
                    let unit_direction = ray.direction.normalize();
                    let dot = unit_direction.dot(hit.normal);
                    let (normal, refraction_ratio) = if dot < 0.0 {
                        (hit.normal, 1.0 / hit.material.ior)
                    } else {
                        (-hit.normal, hit.material.ior)
                    };

                    let cos_theta = (-unit_direction).dot(normal).min(1.0);
                    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
                    let cannot_refract = refraction_ratio * sin_theta > 1.0;

                    if cannot_refract || reflectance(cos_theta, refraction_ratio) > rand::thread_rng().gen() {
                        scatter_direction = unit_direction.reflect(normal);
                    } else {
                        scatter_direction = refract(unit_direction, normal, refraction_ratio);
                    }
                }
            }

            let scattered_ray = Ray::new(hit.point, scatter_direction);
            
            // For Lambertian, we effectively average the indirect light.
            // Since we added direct light, we shouldn't double count it.
            // But our lights are invisible (analytical), so they won't be hit by scattered_ray.
            // However, the sky IS visible.
            // If we hit the sky with scattered_ray, that's "ambient" light.
            // So: Result = Direct + Attenuation * Indirect
            
            return direct_light + attenuation * self.trace_pathtrace(scattered_ray, scene, depth - 1);

        } else {
            // Background color (sky gradient)
            let unit_direction = ray.direction.normalize();
            let t = 0.5 * (unit_direction.y + 1.0);
            return Vec3::new(1.0, 1.0, 1.0) * (1.0 - t) + Vec3::new(0.5, 0.7, 1.0) * t;
        }
    }

    pub fn trace_paths(&self, scene: &Scene, camera: &Camera, count: usize) -> Vec<RayPath> {
        let mut paths = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..count {
            let u = rng.gen::<f32>();
            let v = rng.gen::<f32>();
            let ray = camera.get_ray(u, v);
            
            let mut path = RayPath {
                points: vec![ray.origin],
                segment_types: Vec::new(),
                hit: false,
            };

            match self.mode {
                RenderMode::Raytracing => self.trace_path_recursive_raytracing(ray, scene, self.max_bounces, &mut path, true),
                RenderMode::Pathtracing => self.trace_path_recursive_pathtracing(ray, scene, self.max_bounces, &mut path, true),
            }
            
            paths.push(path);
        }

        paths
    }

    fn trace_path_recursive_raytracing(&self, ray: Ray, scene: &Scene, depth: u32, path: &mut RayPath, is_primary: bool) {
        if depth == 0 {
            path.points.push(ray.at(2.0));
            if !path.points.is_empty() {
                path.segment_types.push(if is_primary { RaySegmentType::Primary } else { RaySegmentType::Diffuse });
            }
            return;
        }

        if let Some(hit) = scene.intersect(&ray, 0.001, f32::INFINITY) {
            path.points.push(hit.point);
            path.hit = true;

            if hit.material.mat_type == MaterialType::Dielectric {
                 // Visualize refraction path - check this FIRST before reflectivity
                let unit_direction = ray.direction.normalize();
                let dot = unit_direction.dot(hit.normal);
                let (normal, refraction_ratio) = if dot < 0.0 {
                    (hit.normal, 1.0 / hit.material.ior)
                } else {
                    (-hit.normal, hit.material.ior)
                };

                let cos_theta = (-unit_direction).dot(normal).min(1.0);
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
                let cannot_refract = refraction_ratio * sin_theta > 1.0;
                
                let direction;
                let segment_type;
                let reflectance_value = reflectance(cos_theta, refraction_ratio);
                let random_value = rand::thread_rng().gen::<f32>();
                if cannot_refract || reflectance_value > random_value {
                    direction = unit_direction.reflect(normal);
                    segment_type = RaySegmentType::Reflection;
                } else {
                    direction = refract(unit_direction, normal, refraction_ratio);
                    segment_type = RaySegmentType::Refraction;
                }
                path.segment_types.push(if is_primary { RaySegmentType::Primary } else { segment_type });
                let refracted_ray = Ray::new(hit.point, direction);
                self.trace_path_recursive_raytracing(refracted_ray, scene, depth - 1, path, false);
            } else if hit.material.reflectivity > 0.0 {
                path.segment_types.push(if is_primary { RaySegmentType::Primary } else { RaySegmentType::Reflection });
                let reflected_ray = Ray::new(hit.point, ray.direction.reflect(hit.normal));
                self.trace_path_recursive_raytracing(reflected_ray, scene, depth - 1, path, false);
            } else {
                path.segment_types.push(if is_primary { RaySegmentType::Primary } else { RaySegmentType::Diffuse });
            }
        } else {
            path.points.push(ray.at(5.0));
            path.segment_types.push(if is_primary { RaySegmentType::Primary } else { RaySegmentType::Diffuse });
        }
    }

    fn trace_path_recursive_pathtracing(&self, ray: Ray, scene: &Scene, depth: u32, path: &mut RayPath, is_primary: bool) {
        if depth == 0 {
            path.points.push(ray.at(2.0));
            if !path.points.is_empty() {
                path.segment_types.push(if is_primary { RaySegmentType::Primary } else { RaySegmentType::Diffuse });
            }
            return;
        }

        if let Some(hit) = scene.intersect(&ray, 0.001, f32::INFINITY) {
            path.points.push(hit.point);
            path.hit = true;

            let scatter_direction;
            let segment_type;
            match hit.material.mat_type {
                MaterialType::Lambertian => {
                    scatter_direction = hit.normal + random_unit_vector();
                    segment_type = RaySegmentType::Diffuse;
                }
                MaterialType::Metal => {
                    let reflected = ray.direction.normalize().reflect(hit.normal);
                    scatter_direction = reflected + random_unit_vector() * hit.material.roughness;
                    segment_type = RaySegmentType::Reflection;
                }
                MaterialType::Dielectric => {
                    let unit_direction = ray.direction.normalize();
                    let dot = unit_direction.dot(hit.normal);
                    let (normal, refraction_ratio) = if dot < 0.0 {
                        (hit.normal, 1.0 / hit.material.ior)
                    } else {
                        (-hit.normal, hit.material.ior)
                    };

                    let cos_theta = (-unit_direction).dot(normal).min(1.0);
                    let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();
                    let cannot_refract = refraction_ratio * sin_theta > 1.0;
                    
                    let reflectance_value = reflectance(cos_theta, refraction_ratio);
                    let random_value = rand::thread_rng().gen::<f32>();
                    if cannot_refract || reflectance_value > random_value {
                        scatter_direction = unit_direction.reflect(normal);
                        segment_type = RaySegmentType::Reflection;
                    } else {
                        scatter_direction = refract(unit_direction, normal, refraction_ratio);
                        segment_type = RaySegmentType::Refraction;
                    }
                }
            }
            
            path.segment_types.push(if is_primary { RaySegmentType::Primary } else { segment_type });
            let scattered_ray = Ray::new(hit.point, scatter_direction);
            self.trace_path_recursive_pathtracing(scattered_ray, scene, depth - 1, path, false);

        } else {
            path.points.push(ray.at(5.0));
            path.segment_types.push(if is_primary { RaySegmentType::Primary } else { RaySegmentType::Diffuse });
        }
    }
}
