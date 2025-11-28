use crate::math::Ray;
use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialType {
    Lambertian,
    Metal,
    Dielectric,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub color: Vec3,
    pub specular: f32,
    pub shininess: f32,
    pub reflectivity: f32, // Kept for legacy/hybrid support
    pub roughness: f32,
    pub ior: f32,
    pub mat_type: MaterialType,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color: Vec3::new(1.0, 1.0, 1.0),
            specular: 0.5,
            shininess: 32.0,
            reflectivity: 0.0,
            roughness: 0.0,
            ior: 1.5,
            mat_type: MaterialType::Lambertian,
        }
    }
}

pub struct HitRecord {
    pub t: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Material,
}

pub trait Intersectable {
    fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl Intersectable for Sphere {
    fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let oc = ray.origin - self.center;
        let a = ray.direction.length_squared();
        let half_b = oc.dot(ray.direction);
        let c = oc.length_squared() - self.radius * self.radius;
        let discriminant = half_b * half_b - a * c;

        if discriminant < 0.0 {
            return None;
        }

        let sqrtd = discriminant.sqrt();
        let mut root = (-half_b - sqrtd) / a;

        if root < t_min || t_max < root {
            root = (-half_b + sqrtd) / a;
            if root < t_min || t_max < root {
                return None;
            }
        }

        let point = ray.at(root);
        let normal = (point - self.center) / self.radius;

        Some(HitRecord {
            t: root,
            point,
            normal,
            material: self.material,
        })
    }
}

pub struct Plane {
    pub point: Vec3,
    pub normal: Vec3,
    pub material: Material,
}

impl Intersectable for Plane {
    fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let denom = self.normal.dot(ray.direction);
        if denom.abs() > 1e-6 {
            let t = (self.point - ray.origin).dot(self.normal) / denom;
            if t >= t_min && t <= t_max {
                return Some(HitRecord {
                    t,
                    point: ray.at(t),
                    normal: self.normal,
                    material: self.material,
                });
            }
        }
        None
    }
}

pub struct Cube {
    pub min: Vec3,
    pub max: Vec3,
    pub material: Material,
}

impl Intersectable for Cube {
    fn intersect(&self, ray: &Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut t_near = t_min;
        let mut t_far = t_max;
        let mut normal = Vec3::ZERO;

        for i in 0..3 {
            let origin = ray.origin[i];
            let direction = ray.direction[i];
            let min_val = self.min[i];
            let max_val = self.max[i];

            if direction.abs() < 1e-6 {
                if origin < min_val || origin > max_val {
                    return None;
                }
            } else {
                let t1 = (min_val - origin) / direction;
                let t2 = (max_val - origin) / direction;

                let (t_min_i, t_max_i, sign) = if t1 > t2 {
                    (t2, t1, 1.0)
                } else {
                    (t1, t2, -1.0)
                };

                if t_min_i > t_near {
                    t_near = t_min_i;
                    normal = Vec3::ZERO;
                    normal[i] = sign;
                }
                if t_max_i < t_far {
                    t_far = t_max_i;
                }

                if t_near > t_far {
                    return None;
                }
            }
        }

        Some(HitRecord {
            t: t_near,
            point: ray.at(t_near),
            normal,
            material: self.material,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LightType {
    Point,
    Directional,
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub light_type: LightType,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}
