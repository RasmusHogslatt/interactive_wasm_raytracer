use crate::math::{Ray, Transform};
use glam::{Mat4, Vec3, Quat};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub transform: Transform,
    pub fov: f32,
    pub aspect_ratio: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            transform: Transform {
                position: Vec3::new(0.0, 2.5, 6.0),
                rotation: Quat::IDENTITY, // Will be set by look_at
                scale: Vec3::ONE,
            },
            fov: 45.0,
            aspect_ratio: 16.0 / 9.0,
        }
    }
}

impl Camera {
    pub fn new(position: Vec3, target: Vec3, fov: f32, aspect_ratio: f32) -> Self {
        let mut cam = Self {
            transform: Transform {
                position,
                ..Default::default()
            },
            fov,
            aspect_ratio,
        };
        cam.look_at(target);
        cam
    }

    pub fn look_at(&mut self, target: Vec3) {
        let forward = (target - self.transform.position).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
        self.transform.rotation = Quat::from_mat3(&glam::Mat3::from_cols(right, up, -forward));
    }

    pub fn get_ray(&self, u: f32, v: f32) -> Ray {
        let theta = self.fov.to_radians();
        let h = (theta / 2.0).tan();
        let viewport_height = 2.0 * h;
        let viewport_width = self.aspect_ratio * viewport_height;

        let forward = self.transform.forward();
        let right = self.transform.right();
        let up = self.transform.up();

        let horizontal = right * viewport_width;
        let vertical = up * viewport_height;
        let lower_left_corner = self.transform.position - horizontal / 2.0 - vertical / 2.0 + forward;

        let direction = lower_left_corner + horizontal * u + vertical * v - self.transform.position;
        
        Ray::new(self.transform.position, direction)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.transform.rotation, self.transform.position).inverse()
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov.to_radians(), self.aspect_ratio, 0.1, 100.0)
    }
}
