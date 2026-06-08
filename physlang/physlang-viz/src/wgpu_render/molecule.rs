//! wgpu ball-and-stick molecule renderer (offscreen PNG).

use crate::camera::{mat4_mul, molecule_orbit_fit, OrbitCamera};
use crate::gjf::{styled_geometry, MolRenderStyle};
use crate::viewer3d::{element_symbol, MoleculeGeometry};
use crate::wgpu_render::device::{
    shared_device, create_mesh_pipeline, draw_mesh, upload_camera_bind_group, upload_mesh,
    GpuVertex, OffscreenTarget,
};

pub fn cpk_color(element: u8) -> [f32; 3] {
    let sym = element_symbol(element);
    match sym {
        "H" => [1.0, 1.0, 1.0],
        "C" => [0.565, 0.565, 0.565],
        "N" => [0.188, 0.314, 0.973],
        "O" => [1.0, 0.051, 0.051],
        "F" => [0.565, 0.878, 0.314],
        "S" => [1.0, 1.0, 0.188],
        "Cl" => [0.122, 0.941, 0.122],
        "Fe" => [0.878, 0.400, 0.200],
        _ => [1.0, 0.412, 0.706],
    }
}

pub fn render_molecule_png(
    mol: &MoleculeGeometry,
    camera: &OrbitCamera,
    style: MolRenderStyle,
) -> Result<Vec<u8>, String> {
    let mol = styled_geometry(mol, style);
    if mol.atoms.is_empty() {
        return Err("molecule has no atoms".into());
    }
    let gpu = shared_device()?;
    let (pipeline, bgl) = create_mesh_pipeline(&gpu, "molecule");
    let target = OffscreenTarget::new(&gpu, camera.width, camera.height);

    let (min, max) = molecule_bounds(&mol);
    let _ = (min, max);
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
    let view = camera.view_matrix(center, distance);
    let proj = camera.proj_matrix();
    let view_proj = mat4_mul(proj, view);
    let bind_group = upload_camera_bind_group(&gpu, &bgl, view_proj);

    let (vertices, indices) = build_molecule_mesh(&mol, style);
    let (vb, ib, count) = upload_mesh(&gpu, &vertices, &indices);
    draw_mesh(
        &gpu,
        &target,
        &pipeline,
        &bind_group,
        &vb,
        &ib,
        count,
        [0.102, 0.102, 0.180, 1.0],
    );

    crate::wgpu_render::device::readback_png(&gpu, &target.color, target.width, target.height)
}

fn molecule_bounds(mol: &MoleculeGeometry) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for a in &mol.atoms {
        let r = a.radius as f32;
        min[0] = min[0].min(a.x as f32 - r);
        min[1] = min[1].min(a.y as f32 - r);
        min[2] = min[2].min(a.z as f32 - r);
        max[0] = max[0].max(a.x as f32 + r);
        max[1] = max[1].max(a.y as f32 + r);
        max[2] = max[2].max(a.z as f32 + r);
    }
    (min, max)
}

fn build_molecule_mesh(mol: &MoleculeGeometry, style: MolRenderStyle) -> (Vec<GpuVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let draw_bonds = !matches!(style, MolRenderStyle::SpaceFill);

    for a in &mol.atoms {
        let color = cpk_color(a.element);
        let scale = a.radius as f32;
        append_sphere(
            &mut vertices,
            &mut indices,
            [a.x as f32, a.y as f32, a.z as f32],
            scale,
            color,
            6,
            10,
        );
    }

    if draw_bonds {
        for bond in &mol.bonds {
            let i = bond[0];
            let j = bond[1];
            if i >= mol.atoms.len() || j >= mol.atoms.len() {
                continue;
            }
            let a = &mol.atoms[i];
            let b = &mol.atoms[j];
            let bond_r = match style {
                MolRenderStyle::Wireframe => 0.04,
                MolRenderStyle::Stick => 0.12,
                _ => 0.08,
            };
            append_cylinder(
                &mut vertices,
                &mut indices,
                [a.x as f32, a.y as f32, a.z as f32],
                [b.x as f32, b.y as f32, b.z as f32],
                bond_r,
                [0.42, 0.42, 0.55],
                8,
            );
        }
    }

    (vertices, indices)
}

fn append_sphere(
    verts: &mut Vec<GpuVertex>,
    indices: &mut Vec<u32>,
    center: [f32; 3],
    radius: f32,
    color: [f32; 3],
    stacks: u32,
    slices: u32,
) -> u32 {
    let base = verts.len() as u32;
    for stack in 0..=stacks {
        let v = stack as f32 / stacks as f32;
        let phi = v * std::f32::consts::PI;
        for slice in 0..=slices {
            let u = slice as f32 / slices as f32;
            let theta = u * std::f32::consts::TAU;
            let nx = phi.sin() * theta.cos();
            let ny = phi.cos();
            let nz = phi.sin() * theta.sin();
            verts.push(GpuVertex {
                pos: [
                    center[0] + nx * radius,
                    center[1] + ny * radius,
                    center[2] + nz * radius,
                ],
                normal: [nx, ny, nz],
                color,
            });
        }
    }
    let ring = slices + 1;
    for stack in 0..stacks {
        for slice in 0..slices {
            let i0 = base + stack * ring + slice;
            let i1 = i0 + ring;
            indices.extend([i0, i1, i0 + 1, i0 + 1, i1, i1 + 1]);
        }
    }
    base
}

fn append_cylinder(
    verts: &mut Vec<GpuVertex>,
    indices: &mut Vec<u32>,
    a: [f32; 3],
    b: [f32; 3],
    radius: f32,
    color: [f32; 3],
    sides: u32,
) {
    let dir = sub(b, a);
    let len = length(dir).max(1e-6);
    let dir_n = scale(dir, 1.0 / len);
    let (right, up) = orthonormal_basis(dir_n);
    let base = verts.len() as u32;

    for cap in 0..2 {
        let t = cap as f32;
        let center = add(a, scale(dir, t));
        for s in 0..sides {
            let ang = (s as f32 / sides as f32) * std::f32::consts::TAU;
            let ca = ang.cos();
            let sa = ang.sin();
            let offset = add(scale(right, ca * radius), scale(up, sa * radius));
            let pos = add(center, offset);
            let normal = normalize(offset);
            verts.push(GpuVertex {
                pos,
                normal,
                color,
            });
        }
    }

    for s in 0..sides {
        let s1 = (s + 1) % sides;
        let i0 = base + s;
        let i1 = base + s1;
        let i2 = base + sides + s;
        let i3 = base + sides + s1;
        indices.extend([i0, i2, i1, i1, i2, i3]);
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

fn length(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = length(v).max(1e-8);
    scale(v, 1.0 / l)
}

fn orthonormal_basis(dir: [f32; 3]) -> ([f32; 3], [f32; 3]) {
    let up = if dir[1].abs() < 0.99 {
        [0.0, 1.0, 0.0]
    } else {
        [1.0, 0.0, 0.0]
    };
    let right = normalize(cross(dir, up));
    let up2 = normalize(cross(right, dir));
    (right, up2)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_xyz;

    #[test]
    fn molecule_png_non_empty() {
        let xyz = "3\nwater\nO 0.0 0.0 0.0\nH 0.96 0.0 0.0\nH -0.24 0.93 0.0\n";
        let mol = parse_xyz(xyz).unwrap();
        let cam = OrbitCamera {
            width: 256,
            height: 256,
            ..Default::default()
        };
        let png = render_molecule_png(&mol, &cam, MolRenderStyle::BallAndStick).expect("render");
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
