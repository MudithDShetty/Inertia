//! Screen-space atom picking aligned with wgpu molecule renderer.

use crate::camera::{molecule_orbit_fit, normalize3, OrbitCamera};
use crate::gjf::{styled_geometry, MolRenderStyle};
use crate::viewer3d::MoleculeGeometry;

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// Ray from camera eye through a screen pixel (render pixel coordinates).
fn screen_ray(
    camera: &OrbitCamera,
    center: [f32; 3],
    distance: f32,
    screen_x: f32,
    screen_y: f32,
) -> ([f32; 3], [f32; 3]) {
    let eye = camera.eye(center, distance);
    let forward = normalize3(sub3(center, eye));
    let world_up = [0.0_f32, 1.0, 0.0];
    let right = normalize3(cross3(forward, world_up));
    let up = cross3(right, forward);
    let half_fov = (camera.fov_y_deg.to_radians() * 0.5).max(0.01);
    let tan_fov = half_fov.tan();
    let aspect = camera.aspect();
    let nx = (screen_x / camera.width.max(1) as f32) * 2.0 - 1.0;
    let ny = 1.0 - (screen_y / camera.height.max(1) as f32) * 2.0;
    let dir = normalize3([
        forward[0] + right[0] * nx * tan_fov * aspect + up[0] * ny * tan_fov,
        forward[1] + right[1] * nx * tan_fov * aspect + up[1] * ny * tan_fov,
        forward[2] + right[2] * nx * tan_fov * aspect + up[2] * ny * tan_fov,
    ]);
    (eye, dir)
}

/// Closest positive ray-sphere hit distance, if any.
fn ray_sphere(origin: [f32; 3], dir: [f32; 3], center: [f32; 3], radius: f32) -> Option<f32> {
    let oc = sub3(origin, center);
    let b = dot3(oc, dir);
    let c = dot3(oc, oc) - radius * radius;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let t = -b - disc.sqrt();
    if t > 1e-4 {
        Some(t)
    } else {
        let t2 = -b + disc.sqrt();
        if t2 > 1e-4 { Some(t2) } else { None }
    }
}

/// Pick atom index from screen coordinates in render pixel space (matches `OrbitCamera` width/height).
pub fn pick_molecule_atom(
    mol: &MoleculeGeometry,
    camera: &OrbitCamera,
    style: MolRenderStyle,
    screen_x: f32,
    screen_y: f32,
) -> Option<usize> {
    let mol = styled_geometry(mol, style);
    if mol.atoms.is_empty() {
        return None;
    }
    let atom_data: Vec<([f32; 3], f32)> = mol
        .atoms
        .iter()
        .map(|a| ([a.x as f32, a.y as f32, a.z as f32], a.radius as f32))
        .collect();
    let padding = match style {
        MolRenderStyle::SpaceFill => 2.0,
        MolRenderStyle::Wireframe => 1.5,
        _ => 1.65,
    };
    let (center, distance) = molecule_orbit_fit(&atom_data, camera.fov_y_deg, padding);
    let (origin, dir) = screen_ray(camera, center, distance, screen_x, screen_y);

    let pick_slack = match style {
        MolRenderStyle::Wireframe | MolRenderStyle::Stick => 0.25,
        _ => 0.15,
    };

    let mut best: Option<(usize, f32)> = None;
    for (i, a) in mol.atoms.iter().enumerate() {
        let pos = [a.x as f32, a.y as f32, a.z as f32];
        let radius = a.radius as f32 + pick_slack;
        if let Some(t) = ray_sphere(origin, dir, pos, radius) {
            if best.map(|(_, bt)| t < bt).unwrap_or(true) {
                best = Some((i, t));
            }
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viewer3d::{AtomBall, MoleculeGeometry};

    #[test]
    fn picks_atom_at_screen_center() {
        let mol = MoleculeGeometry {
            name: "O".into(),
            atoms: vec![AtomBall {
                element: 8,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                radius: 0.5,
            }],
            bonds: vec![],
        };
        let cam = OrbitCamera {
            width: 640,
            height: 480,
            ..Default::default()
        };
        let idx = pick_molecule_atom(&mol, &cam, MolRenderStyle::BallAndStick, 320.0, 240.0);
        assert_eq!(idx, Some(0));
    }
}
