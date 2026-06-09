//! GPU 3D-texture volume ray-march (wgpu fragment shader).

use crate::camera::{bounds_center_radius, OrbitCamera};
use crate::viewer3d::ScalarField;
use crate::wgpu_render::device::{readback_png, shared_device, GpuDevice, OffscreenTarget};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct VolumeUniform {
    eye: [f32; 4],
    forward: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
    bounds_min: [f32; 4],
    bounds_max: [f32; 4],
    /// aspect, tan_half_fov, opacity, steps
    params: [f32; 4],
    /// nx, ny, nz, _pad
    dims: [f32; 4],
}

const VOLUME_SHADER: &str = r#"
struct VolumeParams {
    eye: vec4<f32>,
    forward: vec4<f32>,
    right: vec4<f32>,
    up: vec4<f32>,
    bounds_min: vec4<f32>,
    bounds_max: vec4<f32>,
    params: vec4<f32>,
    dims: vec4<f32>,
}
@group(0) @binding(0) var<uniform> vol: VolumeParams;
@group(0) @binding(1) var volume_tex: texture_3d<f32>;
@group(0) @binding(2) var volume_samp: sampler;

struct VsOut { @builtin(position) clip: vec4<f32>, @location(0) uv: vec2<f32> }

@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var pos = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0),
        vec2(3.0, -1.0),
        vec2(-1.0, 3.0),
    );
    var out: VsOut;
    out.clip = vec4(pos[vi], 0.0, 1.0);
    out.uv = pos[vi] * 0.5 + 0.5;
    return out;
}

fn jet(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    if (x < 0.125) { return vec3(0.0, 0.0, 0.5 + x * 4.0); }
    if (x < 0.375) { return vec3(0.0, (x - 0.125) * 4.0, 1.0); }
    if (x < 0.625) { return vec3((x - 0.375) * 4.0, 1.0, 1.0 - (x - 0.375) * 4.0); }
    if (x < 0.875) { return vec3(1.0, 1.0 - (x - 0.625) * 4.0, 0.0); }
    return vec3(1.0 - (x - 0.875) * 4.0, 0.0, 0.0);
}

fn ray_aabb(origin: vec3<f32>, dir: vec3<f32>, bmin: vec3<f32>, bmax: vec3<f32>) -> vec2<f32> {
    var t_enter = 0.0;
    var t_exit = 1e30;
    for (var i = 0; i < 3; i++) {
        let o = origin[i];
        let d = dir[i];
        let mn = bmin[i];
        let mx = bmax[i];
        if (abs(d) < 1e-8) {
            if (o < mn || o > mx) { return vec2(-1.0, -1.0); }
            continue;
        }
        let inv = 1.0 / d;
        var t0 = (mn - o) * inv;
        var t1 = (mx - o) * inv;
        if (t0 > t1) { let tmp = t0; t0 = t1; t1 = tmp; }
        t_enter = max(t_enter, t0);
        t_exit = min(t_exit, t1);
        if (t_enter > t_exit) { return vec2(-1.0, -1.0); }
    }
    if (t_exit < 0.0) { return vec2(-1.0, -1.0); }
    return vec2(max(t_enter, 0.0), t_exit);
}

fn sample_norm(p: vec3<f32>) -> f32 {
    let bmin = vol.bounds_min.xyz;
    let bmax = vol.bounds_max.xyz;
    let dims = vol.dims.xyz;
    let nx = max(dims.x, 2.0);
    let ny = max(dims.y, 2.0);
    let nz = max(dims.z, 2.0);
    let uvw = (p - bmin) / (bmax - bmin);
    if (any(uvw < vec3(0.0)) || any(uvw > vec3(1.0))) { return 0.0; }
    let tc = uvw * vec3((nx - 1.0) / nx, (ny - 1.0) / ny, (nz - 1.0) / nz) + vec3(0.5 / nx, 0.5 / ny, 0.5 / nz);
    return textureSampleLevel(volume_tex, volume_samp, tc, 0.0).r;
}

@fragment fn fs_main(input: VsOut) -> @location(0) vec4<f32> {
    let aspect = vol.params.x;
    let tan_half = vol.params.y;
    let opacity = vol.params.z;
    let steps = max(vol.params.w, 8.0);
    let ndc_x = input.uv.x * 2.0 - 1.0;
    let ndc_y = 1.0 - input.uv.y * 2.0;
    let eye = vol.eye.xyz;
    let forward = normalize(vol.forward.xyz);
    let right = normalize(vol.right.xyz);
    let up = normalize(vol.up.xyz);
    let dir = normalize(forward + right * ndc_x * tan_half * aspect + up * ndc_y * tan_half);
    let bmin = vol.bounds_min.xyz;
    let bmax = vol.bounds_max.xyz;
    let te = ray_aabb(eye, dir, bmin, bmax);
    if (te.x < 0.0) {
        return vec4(0.102, 0.102, 0.180, 1.0);
    }
    let dt = (te.y - te.x) / steps;
    var color = vec3(0.0);
    var alpha = 0.0;
    for (var s = 0.0; s < steps; s += 1.0) {
        let t = te.x + (s + 0.5) * dt;
        let p = eye + dir * t;
        let norm = sample_norm(p);
        if (norm < 0.02) { continue; }
        let rgb = jet(norm);
        let a = clamp(norm * norm * opacity * dt * 8.0, 0.0, 1.0);
        let one_minus = 1.0 - alpha;
        color += one_minus * a * rgb;
        alpha += one_minus * a;
        if (alpha > 0.98) { break; }
    }
    if (alpha < 0.01) {
        return vec4(0.102, 0.102, 0.180, 1.0);
    }
    return vec4(color, 1.0);
}
"#;

/// Max voxels for GPU 3D texture upload (fallback to CPU above this).
const GPU_VOXEL_LIMIT: usize = 128 * 128 * 128;

pub fn render_field_volume_gpu_png(
    field: &ScalarField,
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let voxels = field.shape[0] * field.shape[1] * field.shape[2];
    if voxels > GPU_VOXEL_LIMIT || field.shape.iter().any(|&d| d < 2) {
        return Err("field too large or degenerate for GPU volume".into());
    }
    let gpu = shared_device()?;
    render_volume_inner(&gpu, field, camera)
}

fn render_volume_inner(
    gpu: &GpuDevice,
    field: &ScalarField,
    camera: &OrbitCamera,
) -> Result<Vec<u8>, String> {
    let [nx, ny, nz] = field.shape;
    let (vmin, vmax) = field
        .values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
            (lo.min(v), hi.max(v))
        });
    let span = (vmax - vmin).max(1e-12);
    let tex_data: Vec<f32> = field
        .values
        .iter()
        .map(|v| ((v - vmin) / span) as f32)
        .collect();

    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("volume-3d"),
        size: wgpu::Extent3d {
            width: nx as u32,
            height: ny as u32,
            depth_or_array_layers: nz as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&tex_data),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * nx as u32),
            rows_per_image: Some(ny as u32),
        },
        wgpu::Extent3d {
            width: nx as u32,
            height: ny as u32,
            depth_or_array_layers: nz as u32,
        },
    );
    let tex_view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D3),
        ..Default::default()
    });

    let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("volume-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let [[x0, y0, z0], [x1, y1, z1]] = field.bounds;
    let min = [x0 as f32, y0 as f32, z0 as f32];
    let max = [x1 as f32, y1 as f32, z1 as f32];
    let (center, distance) = bounds_center_radius(min, max);
    let eye = camera.eye(center, distance);
    let view = camera.view_matrix(center, distance);
    let _ = view;
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

    let uniform = VolumeUniform {
        eye: [eye[0], eye[1], eye[2], 0.0],
        forward: [forward[0], forward[1], forward[2], 0.0],
        right: [right[0], right[1], right[2], 0.0],
        up: [up[0], up[1], up[2], 0.0],
        bounds_min: [min[0], min[1], min[2], 0.0],
        bounds_max: [max[0], max[1], max[2], 0.0],
        params: [aspect, tan_half, 0.18, 96.0],
        dims: [nx as f32, ny as f32, nz as f32, 0.0],
    };
    let ub = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("volume-uniform"),
        size: std::mem::size_of::<VolumeUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    gpu.queue
        .write_buffer(&ub, 0, bytemuck::bytes_of(&uniform));

    let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("volume-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D3,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("volume-bg"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: ub.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&tex_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    let shader = gpu
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("volume-shader"),
            source: wgpu::ShaderSource::Wgsl(VOLUME_SHADER.into()),
        });
    let pl = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("volume-pl"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
    let pipeline = gpu
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("volume-pipeline"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

    let target = OffscreenTarget::new(gpu, camera.width, camera.height);
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("volume-encoder"),
        });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("volume-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.102,
                        g: 0.102,
                        b: 0.180,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
    gpu.queue.submit(Some(encoder.finish()));
    readback_png(gpu, &target.color, target.width, target.height)
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-8);
    [v[0] / len, v[1] / len, v[2] / len]
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
    use crate::field_slice::demo_gaussian_field;

    #[test]
    fn gpu_volume_png_non_empty() {
        let field = demo_gaussian_field(24);
        let cam = OrbitCamera {
            width: 128,
            height: 128,
            ..Default::default()
        };
        let png = render_field_volume_gpu_png(&field, &cam).expect("gpu volume");
        assert_eq!(&png[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
