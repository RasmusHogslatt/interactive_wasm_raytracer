use crate::primitives::{Cube, Intersectable, Light, LightType, Plane, Sphere, HitRecord, Material, MaterialType};
use crate::math::Ray;
use glam::Vec3;

pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub cubes: Vec<Cube>,
    pub planes: Vec<Plane>,
    pub lights: Vec<Light>,
}

impl Default for Scene {
    fn default() -> Self {
        let mut spheres = Vec::new();
        let mut cubes = Vec::new();
        let mut planes = Vec::new();
        
        // Floor Plane
        planes.push(Plane {
            point: Vec3::new(0.0, -0.5, 0.0),
            normal: Vec3::new(0.0, 1.0, 0.0),
            material: Material {
                color: Vec3::new(0.5, 0.5, 0.5),
                specular: 0.0,
                shininess: 0.0,
                reflectivity: 0.0,
                roughness: 1.0,
                ior: 1.5,
                mat_type: MaterialType::Lambertian,
            },
        });

        // 5 Cubes with Spheres in a circle
        let count = 5;
        let radius = 3.0;
        
        for i in 0..count {
            let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
            let x = angle.cos() * radius;
            let z = angle.sin() * radius;
            
            // Cube (Light turqoise diffuse)
            cubes.push(Cube {
                min: Vec3::new(x - 0.5, -0.5, z - 0.5),
                max: Vec3::new(x + 0.5, 0.5, z + 0.5),
                material: Material {
                    color: Vec3::new(0.1, 0.8, 0.8),
                    specular: 0.0,
                    shininess: 0.0,
                    reflectivity: 0.0,
                    roughness: 1.0,
                    ior: 1.5,
                    mat_type: MaterialType::Lambertian,
                },
            });

            // Sphere on top
            let material = match i {
                0 => Material { // Red Lambertian
                    color: Vec3::new(0.8, 0.1, 0.1),
                    specular: 0.5,
                    shininess: 32.0,
                    reflectivity: 0.0,
                    roughness: 0.1,
                    ior: 1.5,
                    mat_type: MaterialType::Lambertian,
                },
                1 => Material { // Gray Metal
                    color: Vec3::new(0.6, 0.6, 0.6),
                    specular: 0.7,
                    shininess: 64.0,
                    reflectivity: 0.8,
                    roughness: 0.1,
                    ior: 1.5,
                    mat_type: MaterialType::Metal,
                },
                2 => Material { // Glass
                    color: Vec3::new(1.0, 1.0, 1.0),
                    specular: 1.0,
                    shininess: 100.0,
                    reflectivity: 0.1,
                    roughness: 0.0,
                    ior: 1.52,
                    mat_type: MaterialType::Dielectric,
                },
                3 => Material { // Blue Metal (Rough)
                    color: Vec3::new(0.1, 0.1, 0.8),
                    specular: 0.5,
                    shininess: 32.0,
                    reflectivity: 0.4,
                    roughness: 0.4,
                    ior: 1.5,
                    mat_type: MaterialType::Metal,
                },
                _ => Material { // Yellow Lambertian
                    color: Vec3::new(0.8, 0.8, 0.1),
                    specular: 0.5,
                    shininess: 32.0,
                    reflectivity: 0.0,
                    roughness: 0.1,
                    ior: 1.5,
                    mat_type: MaterialType::Lambertian,
                },
            };

            spheres.push(Sphere {
                center: Vec3::new(x, 1.0, z),
                radius: 0.5,
                material,
            });
        }

        // 3-Point Lighting Setup
        
        // Key Light: Main light from front-right, slightly above
        // Warm white, brightest light
        let key_light = Light {
            light_type: LightType::Point,
            position: Vec3::new(5.0, 6.0, 4.0),
            direction: Vec3::ZERO,
            color: Vec3::new(1.0, 0.98, 0.95), // Slightly warm
            intensity: 0.8,
        };

        // Fill Light: Softer light from front-left to reduce harsh shadows
        // Cool white, about 50% of key light intensity
        let fill_light = Light {
            light_type: LightType::Point,
            position: Vec3::new(-4.0, 3.0, 3.0),
            direction: Vec3::ZERO,
            color: Vec3::new(0.95, 0.98, 1.0), // Slightly cool
            intensity: 0.4,
        };

        // Rim/Back Light: From behind to create separation and highlights
        // Positioned high and behind the scene
        let rim_light = Light {
            light_type: LightType::Point,
            position: Vec3::new(0.0, 7.0, -5.0),
            direction: Vec3::ZERO,
            color: Vec3::new(1.0, 1.0, 1.0),
            intensity: 0.5,
        };

        Scene {
            spheres,
            cubes,
            planes,
            lights: vec![key_light, fill_light, rim_light],
        }
    }
}

impl Scene {
    pub fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest_hit: Option<HitRecord> = None;
        let mut closest_t = t_max;

        for sphere in &self.spheres {
            if let Some(hit) = sphere.intersect(ray, t_min, closest_t) {
                closest_t = hit.t;
                closest_hit = Some(hit);
            }
        }

        for cube in &self.cubes {
            if let Some(hit) = cube.intersect(ray, t_min, closest_t) {
                closest_t = hit.t;
                closest_hit = Some(hit);
            }
        }

        for plane in &self.planes {
            if let Some(hit) = plane.intersect(ray, t_min, closest_t) {
                closest_t = hit.t;
                closest_hit = Some(hit);
            }
        }

        closest_hit
    }
}
