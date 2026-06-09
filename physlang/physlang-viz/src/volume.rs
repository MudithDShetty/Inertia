//! CPU ray-marching volume renderer (Phase 3 stub — front-to-back compositing).

use crate::camera::{bounds_center_radius, normalize3, OrbitCamera};
use crate::field_slice::jet_colormap;
use crate::viewer3d::ScalarField;

#[derive(Debug, Clone, Copy)]
pub struct VolumeRenderConfig {
    /// Number of samples along each view ray.
    pub steps: u32,
    /// Opacity scale for density transfer function.
    pub opacity: f32,
}

impl Default for VolumeRenderConfig {
    fn default() -> Self {
        Self {
            steps: 96,
            opacity: 0.18,
        }
    }
}

/// Ray-march a scalar field and return RGBA8 (width × height × 4).
pub fn ray_march_rgba(
    field: &ScalarField,
    camera: &OrbitCamera,
    config: VolumeRenderConfig,
) -> Result<Vec<u8>, String> {
    let w = camera.width.max(1);
    let h = camera.height.max(1);
    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let min = [x0 as f32, y0 as f32, z0 as f32];
    let max = [x1 as f32, y1 as f32, z1 as f32];
    let (center, distance) = bounds_center_radius(min, max);
    let eye = camera.eye(center, distance);

    let (vmin, vmax) = field
        .values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let span = (vmax - vmin).max(1e-12);

    let steps = config.steps.max(8) as usize;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let bg = [26, 26, 46, 255];

    for py in 0..h {
        for px in 0..w {
            let (origin, dir) = pixel_ray(camera, center, distance, eye, px, py, w, h);
            let (t_enter, t_exit) = match ray_aabb(origin, dir, min, max) {
                Some(t) => t,
                None => {
                    put_pixel(&mut rgba, w, px, py, bg);
                    continue;
                }
            };
            let mut color = [0.0f32; 3];
            let mut alpha = 0.0f32;
            let dt = (t_exit - t_enter).max(0.0) / steps as f32;
            for s in 0..steps {
                let t = t_enter + (s as f32 + 0.5) * dt;
                let p = [
                    origin[0] + dir[0] * t,
                    origin[1] + dir[1] * t,
                    origin[2] + dir[2] * t,
                ];
                let v = sample_field(field, p[0], p[1], p[2]);
                let norm = ((v - vmin) / span).clamp(0.0, 1.0);
                if norm < 0.02 {
                    continue;
                }
                let [r, g, b, _] = jet_colormap(norm);
                let a = (norm * norm * config.opacity as f64) as f32 * dt * 8.0;
                let a = a.clamp(0.0, 1.0);
                let one_minus = 1.0 - alpha;
                color[0] += one_minus * a * (r as f32 / 255.0);
                color[1] += one_minus * a * (g as f32 / 255.0);
                color[2] += one_minus * a * (b as f32 / 255.0);
                alpha += one_minus * a;
                if alpha > 0.98 {
                    break;
                }
            }
            let out = if alpha < 0.01 {
                bg
            } else {
                [
                    (color[0].clamp(0.0, 1.0) * 255.0) as u8,
                    (color[1].clamp(0.0, 1.0) * 255.0) as u8,
                    (color[2].clamp(0.0, 1.0) * 255.0) as u8,
                    255,
                ]
            };
            put_pixel(&mut rgba, w, px, py, out);
        }
    }
    Ok(rgba)
}

fn put_pixel(buf: &mut [u8], w: u32, px: u32, py: u32, c: [u8; 4]) {
    let i = ((py * w + px) * 4) as usize;
    if i + 3 < buf.len() {
        buf[i..i + 4].copy_from_slice(&c);
    }
}

fn pixel_ray(
    camera: &OrbitCamera,
    center: [f32; 3],
    _distance: f32,
    eye: [f32; 3],
    px: u32,
    py: u32,
    w: u32,
    h: u32,
) -> ([f32; 3], [f32; 3]) {
    let forward = normalize3([
        center[0] - eye[0],
        center[1] - eye[1],
        center[2] - eye[2],
    ]);
    let world_up = [0.0f32, 1.0, 0.0];
    let right = normalize3(cross(forward, world_up));
    let up = cross(right, forward);
    let aspect = camera.aspect();
    let tan_half = (camera.fov_y_deg.to_radians() * 0.5).tan();
    let ndc_x = ((px as f32 + 0.5) / w as f32) * 2.0 - 1.0;
    let ndc_y = 1.0 - ((py as f32 + 0.5) / h as f32) * 2.0;
    let dir = normalize3([
        forward[0] + right[0] * ndc_x * tan_half * aspect + up[0] * ndc_y * tan_half,
        forward[1] + right[1] * ndc_x * tan_half * aspect + up[1] * ndc_y * tan_half,
        forward[2] + right[2] * ndc_x * tan_half * aspect + up[2] * ndc_y * tan_half,
    ]);
    (eye, dir)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn ray_aabb(origin: [f32; 3], dir: [f32; 3], min: [f32; 3], max: [f32; 3]) -> Option<(f32, f32)> {
    let mut t_enter = 0.0f32;
    let mut t_exit = f32::INFINITY;
    for i in 0..3 {
        if dir[i].abs() < 1e-8 {
            if origin[i] < min[i] || origin[i] > max[i] {
                return None;
            }
            continue;
        }
        let inv = 1.0 / dir[i];
        let mut t0 = (min[i] - origin[i]) * inv;
        let mut t1 = (max[i] - origin[i]) * inv;
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }
        t_enter = t_enter.max(t0);
        t_exit = t_exit.min(t1);
        if t_enter > t_exit {
            return None;
        }
    }
    if t_exit < 0.0 {
        return None;
    }
    Some((t_enter.max(0.0), t_exit))
}

fn sample_field(field: &ScalarField, wx: f32, wy: f32, wz: f32) -> f64 {
    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let [nx, ny, nz] = field.shape;
    if nx < 2 || ny < 2 || nz < 2 {
        return 0.0;
    }
    let dx = (x1 - x0).max(1e-12);
    let dy = (y1 - y0).max(1e-12);
    let dz = (z1 - z0).max(1e-12);
    if wx < x0 as f32 || wx > x1 as f32 || wy < y0 as f32 || wy > y1 as f32 || wz < z0 as f32 || wz > z1 as f32
    {
        return 0.0;
    }
    let fx = (wx as f64 - x0) / dx * (nx - 1) as f64;
    let fy = (wy as f64 - y0) / dy * (ny - 1) as f64;
    let fz = (wz as f64 - z0) / dz * (nz - 1) as f64;
    trilinear(field, fx, fy, fz)
}

fn trilinear(field: &ScalarField, fx: f64, fy: f64, fz: f64) -> f64 {
    let [nx, ny, nz] = field.shape;
    let x0 = fx.floor().clamp(0.0, (nx - 2) as f64) as usize;
    let y0 = fy.floor().clamp(0.0, (ny - 2) as f64) as usize;
    let z0 = fz.floor().clamp(0.0, (nz - 2) as f64) as usize;
    let tx = fx - x0 as f64;
    let ty = fy - y0 as f64;
    let tz = fz - z0 as f64;
    let c000 = grid_at(field, x0, y0, z0);
    let c100 = grid_at(field, x0 + 1, y0, z0);
    let c010 = grid_at(field, x0, y0 + 1, z0);
    let c110 = grid_at(field, x0 + 1, y0 + 1, z0);
    let c001 = grid_at(field, x0, y0, z0 + 1);
    let c101 = grid_at(field, x0 + 1, y0, z0 + 1);
    let c011 = grid_at(field, x0, y0 + 1, z0 + 1);
    let c111 = grid_at(field, x0 + 1, y0 + 1, z0 + 1);
    let c00 = c000 * (1.0 - tx) + c100 * tx;
    let c01 = c001 * (1.0 - tx) + c101 * tx;
    let c10 = c010 * (1.0 - tx) + c110 * tx;
    let c11 = c011 * (1.0 - tx) + c111 * tx;
    let c0 = c00 * (1.0 - ty) + c10 * ty;
    let c1 = c01 * (1.0 - ty) + c11 * ty;
    c0 * (1.0 - tz) + c1 * tz
}

fn grid_at(field: &ScalarField, i: usize, j: usize, k: usize) -> f64 {
    let [nx, ny, _] = field.shape;
    field.values[i + j * nx + k * nx * ny]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_slice::demo_gaussian_field;

    #[test]
    fn ray_march_demo_field_non_empty() {
        let field = demo_gaussian_field(24);
        let mut cam = OrbitCamera::default();
        cam.width = 64;
        cam.height = 64;
        let rgba = ray_march_rgba(&field, &cam, VolumeRenderConfig::default()).expect("ray march");
        assert_eq!(rgba.len(), 64 * 64 * 4);
        let lit = rgba.chunks(4).filter(|p| p[0] > 40 || p[1] > 40 || p[2] > 40).count();
        assert!(lit > 100, "expected visible voxels, got {lit} lit pixels");
    }
}
