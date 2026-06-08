//! Orbit camera for offscreen wgpu 3D views.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    /// Multiplier on auto-fit distance (1.0 = fit bounds).
    pub zoom: f32,
    pub width: u32,
    pub height: u32,
    pub fov_y_deg: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.35,
            zoom: 1.0,
            width: 640,
            height: 480,
            fov_y_deg: 45.0,
        }
    }
}

impl OrbitCamera {
    pub fn aspect(&self) -> f32 {
        if self.height == 0 {
            1.0
        } else {
            self.width as f32 / self.height as f32
        }
    }

    /// Eye position orbiting `target` at `distance`.
    pub fn eye(&self, target: [f32; 3], distance: f32) -> [f32; 3] {
        let d = distance / self.zoom.max(0.05);
        let cp = self.pitch.clamp(-1.4, 1.4).cos();
        let sp = self.pitch.clamp(-1.4, 1.4).sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        [
            target[0] + d * cp * sy,
            target[1] + d * sp,
            target[2] + d * cp * cy,
        ]
    }

    pub fn view_matrix(&self, target: [f32; 3], distance: f32) -> [[f32; 4]; 4] {
        look_at(self.eye(target, distance), target, [0.0, 1.0, 0.0])
    }

    pub fn proj_matrix(&self) -> [[f32; 4]; 4] {
        perspective(self.fov_y_deg.to_radians(), self.aspect(), 0.01, 1000.0)
    }
}

/// Auto-fit distance from axis-aligned bounds (min/max triples).
pub fn bounds_center_radius(min: [f32; 3], max: [f32; 3]) -> ([f32; 3], f32) {
    bounds_center_radius_fov(min, max, 45.0)
}

/// Fit orbit distance from AABB; `fov_y_deg` is vertical field of view.
pub fn bounds_center_radius_fov(min: [f32; 3], max: [f32; 3], fov_y_deg: f32) -> ([f32; 3], f32) {
    let center = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let dx = (max[0] - min[0]).max(1e-6);
    let dy = (max[1] - min[1]).max(1e-6);
    let dz = (max[2] - min[2]).max(1e-6);
    let extent = (dx * dx + dy * dy + dz * dz).sqrt().max(0.4);
    let half_fov = (fov_y_deg.to_radians() * 0.5).max(0.01);
    let distance = (extent * 0.5) / half_fov.tan() * 1.65;
    (center, distance.max(1.5))
}

/// Fit from atomic positions + per-atom radii (matches 2D canvas framing better).
pub fn molecule_orbit_fit(atoms: &[([f32; 3], f32)], fov_y_deg: f32, padding: f32) -> ([f32; 3], f32) {
    if atoms.is_empty() {
        return ([0.0, 0.0, 0.0], 4.0);
    }
    let n = atoms.len() as f32;
    let center = atoms.iter().fold([0.0f32; 3], |acc, (p, _)| {
        [acc[0] + p[0], acc[1] + p[1], acc[2] + p[2]]
    });
    let center = [center[0] / n, center[1] / n, center[2] / n];
    let mut max_r = 0.0f32;
    for (p, rad) in atoms {
        let dx = p[0] - center[0];
        let dy = p[1] - center[1];
        let dz = p[2] - center[2];
        let d = (dx * dx + dy * dy + dz * dz).sqrt() + *rad;
        max_r = max_r.max(d);
    }
    max_r = max_r.max(0.4);
    let half_fov = (fov_y_deg.to_radians() * 0.5).max(0.01);
    let distance = max_r / half_fov.tan() * padding.max(1.2);
    (center, distance.max(1.5))
}

pub fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            out[i][j] = a[i][0] * b[0][j]
                + a[i][1] * b[1][j]
                + a[i][2] * b[2][j]
                + a[i][3] * b[3][j];
        }
    }
    out
}

fn look_at(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let f = normalize(sub(center, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-dot(s, eye), -dot(u, eye), dot(f, eye), 1.0],
    ]
}

fn perspective(fovy: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let t = (fovy * 0.5).tan();
    let sy = 1.0 / t;
    let sx = sy / aspect.max(1e-6);
    let a = far / (near - far);
    let b = (near * far) / (near - far);
    [
        [sx, 0.0, 0.0, 0.0],
        [0.0, sy, 0.0, 0.0],
        [0.0, 0.0, a, -1.0],
        [0.0, 0.0, b, 0.0],
    ]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / len, v[1] / len, v[2] / len]
}

pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    normalize(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_eye_non_zero() {
        let cam = OrbitCamera::default();
        let eye = cam.eye([0.0, 0.0, 0.0], 5.0);
        let dist = (eye[0] * eye[0] + eye[1] * eye[1] + eye[2] * eye[2]).sqrt();
        assert!(dist > 1.0);
    }
}
