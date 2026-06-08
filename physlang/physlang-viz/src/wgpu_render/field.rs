//! wgpu 3D scalar-field views: textured Z-slice plane and isosurface stub.

use crate::camera::{bounds_center_radius, mat4_mul, OrbitCamera};
use crate::field_slice::{extract_slice, slice_to_rgba, SliceAxis};
use crate::marching_cubes::extract_isosurface;
use crate::viewer3d::ScalarField;
use crate::wgpu_render::device::{
    create_mesh_pipeline, draw_mesh, readback_png, shared_device, upload_camera_bind_group,
    upload_mesh, GpuDevice, GpuVertex, OffscreenTarget,
};

/// Render a Z-slice as a textured quad in 3D (orbit camera).
pub fn render_field_slice_3d_png(
    field: &ScalarField,
    z_index: usize,
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let gpu = shared_device()?;
    let slice = extract_slice(field, SliceAxis::Z, z_index)?;
    let rgba = slice_to_rgba(&slice);
    render_textured_quad_png(&gpu, field, z_index, &rgba, slice.width, slice.height, camera)
}

/// Legacy path: upload RGBA slice to GPU texture and read back PNG (no 3D).
pub fn render_field_slice_png(field: &ScalarField, z_index: usize) -> Result<Vec<u8>, String> {
    let slice = extract_slice(field, SliceAxis::Z, z_index)?;
    let rgba = slice_to_rgba(&slice);
    upload_rgba_png(slice.width as u32, slice.height as u32, &rgba)
}

fn render_textured_quad_png(
    gpu: &GpuDevice,
    field: &ScalarField,
    z_index: usize,
    rgba: &[u8],
    tex_w: usize,
    tex_h: usize,
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let (pipeline, bgl) = create_mesh_pipeline(gpu, "field-slice-3d");
    let target = OffscreenTarget::new(gpu, camera.width, camera.height);

    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let nz = field.shape[2].max(1);
    let t = if nz > 1 {
        z_index as f64 / (nz - 1) as f64
    } else {
        0.0
    };
    let z = z0 + t * (z1 - z0);
    let corners = [
        ([x0 as f32, y0 as f32, z as f32], pixel_color(rgba, tex_w, tex_h, 0, 0)),
        ([x1 as f32, y0 as f32, z as f32], pixel_color(rgba, tex_w, tex_h, tex_w - 1, 0)),
        ([x1 as f32, y1 as f32, z as f32], pixel_color(rgba, tex_w, tex_h, tex_w - 1, tex_h - 1)),
        ([x0 as f32, y1 as f32, z as f32], pixel_color(rgba, tex_w, tex_h, 0, tex_h - 1)),
    ];
    let (vertices, indices) = colored_quad_mesh(&corners);
    let (vb, ib, count) = upload_mesh(gpu, &vertices, &indices);

    let min = [x0 as f32, y0 as f32, z0 as f32];
    let max = [x1 as f32, y1 as f32, z1 as f32];
    let (center, distance) = bounds_center_radius(min, max);
    let view = camera.view_matrix(center, distance);
    let proj = camera.proj_matrix();
    let bind_group = upload_camera_bind_group(&gpu, &bgl, mat4_mul(proj, view));

    draw_mesh(
        gpu,
        &target,
        &pipeline,
        &bind_group,
        &vb,
        &ib,
        count,
        [0.102, 0.102, 0.180, 1.0],
    );
    readback_png(gpu, &target.color, target.width, target.height)
}

fn pixel_color(rgba: &[u8], w: usize, h: usize, ix: usize, iy: usize) -> [f32; 3] {
    if w == 0 || h == 0 || rgba.len() < 4 {
        return [0.3, 0.3, 0.5];
    }
    let ix = ix.min(w - 1);
    let iy = iy.min(h - 1);
    let o = (iy * w + ix) * 4;
    [
        rgba[o] as f32 / 255.0,
        rgba[o + 1] as f32 / 255.0,
        rgba[o + 2] as f32 / 255.0,
    ]
}

fn colored_quad_mesh(corners: &[([f32; 3], [f32; 3]); 4]) -> (Vec<GpuVertex>, Vec<u32>) {
    let normal = [0.0, 0.0, 1.0];
    let verts: Vec<GpuVertex> = corners
        .iter()
        .map(|(p, c)| GpuVertex {
            pos: *p,
            normal,
            color: *c,
        })
        .collect();
    let indices = vec![0, 1, 2, 0, 2, 3];
    (verts, indices)
}

/// Render marching-cubes isosurface mesh in 3D.
pub fn render_field_isosurface_png(
    field: &ScalarField,
    isovalue: f64,
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let mesh = extract_isosurface(field, isovalue);
    if mesh.vertices.is_empty() {
        return Err("isosurface produced empty mesh".into());
    }
    render_colored_isosurface_meshes(&[(mesh, [0.2, 0.7, 0.95])], camera)
}

/// Render signed MO isosurface: red (+ψ) and blue (−ψ) lobes together (GaussView-style).
pub fn render_field_mo_isosurface_png(
    field: &ScalarField,
    level: f64,
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let t = level.clamp(0.01, 0.99);
    let max_abs = field
        .values
        .iter()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    if max_abs < 1e-20 {
        return Err("MO field has zero amplitude".into());
    }
    let iso = t * max_abs;
    let pos = extract_isosurface(field, iso);
    let neg = extract_isosurface(field, -iso);
    if pos.vertices.is_empty() && neg.vertices.is_empty() {
        return Err("MO isosurface produced empty mesh".into());
    }
    let mut layers = Vec::new();
    if !pos.vertices.is_empty() {
        layers.push((pos, [0.92, 0.22, 0.22]));
    }
    if !neg.vertices.is_empty() {
        layers.push((neg, [0.22, 0.35, 0.92]));
    }
    render_colored_isosurface_meshes(&layers, camera)
}

fn render_colored_isosurface_meshes(
    layers: &[(
        crate::marching_cubes::IsoMesh,
        [f32; 3],
    )],
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let gpu = shared_device()?;
    let (pipeline, bgl) = create_mesh_pipeline(&gpu, "field-iso");
    let target = OffscreenTarget::new(&gpu, camera.width, camera.height);

    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for (mesh, _) in layers {
        for v in &mesh.vertices {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
    }
    let (center, distance) = bounds_center_radius(min, max);
    let view = camera.view_matrix(center, distance);
    let proj = camera.proj_matrix();
    let bind_group = upload_camera_bind_group(&gpu, &bgl, mat4_mul(proj, view));

    let mut all_verts = Vec::new();
    let mut all_indices = Vec::new();
    let mut base = 0u32;
    for (mesh, color) in layers {
        for (p, n) in mesh.vertices.iter().zip(mesh.normals.iter()) {
            all_verts.push(GpuVertex {
                pos: *p,
                normal: *n,
                color: *color,
            });
        }
        for idx in &mesh.indices {
            all_indices.push(base + idx);
        }
        base += mesh.vertices.len() as u32;
    }
    let (vb, ib, count) = upload_mesh(&gpu, &all_verts, &all_indices);
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
    readback_png(&gpu, &target.color, target.width, target.height)
}

fn upload_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    if rgba.len() != (width as usize) * (height as usize) * 4 {
        return Err("rgba buffer size mismatch".into());
    }
    let gpu = shared_device()?;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("field-slice"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    readback_png(&gpu, &texture, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field_slice::demo_gaussian_field;

    #[test]
    fn field_slice_3d_png_non_empty() {
        let field = demo_gaussian_field(16);
        let cam = OrbitCamera {
            width: 128,
            height: 128,
            ..Default::default()
        };
        let png = render_field_slice_3d_png(&field, 8, &cam).expect("3d slice");
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn field_isosurface_png_non_empty() {
        let field = demo_gaussian_field(16);
        let cam = OrbitCamera {
            width: 128,
            height: 128,
            ..Default::default()
        };
        let png = render_field_isosurface_png(&field, 0.3, &cam).expect("iso");
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }

    #[test]
    fn field_mo_isosurface_png_non_empty() {
        let mut field = demo_gaussian_field(16);
        // Signed field: center positive, edges negative
        let mid = field.values.len() / 2;
        for (i, v) in field.values.iter_mut().enumerate() {
            *v = if i < mid { 1.0 } else { -0.8 };
        }
        let cam = OrbitCamera {
            width: 128,
            height: 128,
            ..Default::default()
        };
        let png = render_field_mo_isosurface_png(&field, 0.35, &cam).expect("mo iso");
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
