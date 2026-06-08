//! 3D viewer stub — wgpu + winit backend planned for Phase 3.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScalarField {
    pub shape: [usize; 3],
    pub values: Vec<f64>,
    /// Axis-aligned bounds [[xmin, ymin, zmin], [xmax, ymax, zmax]].
    pub bounds: [[f64; 3]; 2],
}

impl ScalarField {
    pub fn new(shape: [usize; 3], values: Vec<f64>, bounds: [[f64; 3]; 2]) -> Result<Self, String> {
        let n = shape[0] * shape[1] * shape[2];
        if values.len() != n {
            return Err(format!("values len {n} != product of shape {:?}", shape));
        }
        Ok(Self {
            shape,
            values,
            bounds,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomBall {
    pub element: u8,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoleculeGeometry {
    pub name: String,
    pub atoms: Vec<AtomBall>,
    pub bonds: Vec<[usize; 2]>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ViewerScene {
    pub fields: Vec<ScalarField>,
    pub molecules: Vec<MoleculeGeometry>,
}

/// Placeholder for future wgpu renderer.
#[derive(Debug, Default)]
pub struct WgpuViewerStub {
    scene: ViewerScene,
}

impl WgpuViewerStub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_field(&mut self, field: ScalarField) {
        self.scene.fields.push(field);
    }

    pub fn load_molecule(&mut self, mol: MoleculeGeometry) {
        self.scene.molecules.push(mol);
    }

    pub fn scene(&self) -> &ViewerScene {
        &self.scene
    }

    /// Returns PNG bytes via offscreen wgpu renderer (requires `wgpu` feature).
    #[cfg(feature = "wgpu")]
    pub fn render_frame(&self, camera: crate::OrbitCamera) -> Result<Vec<u8>, String> {
        if let Some(mol) = self.scene.molecules.first() {
            return crate::render_molecule_png(mol, &camera, crate::MolRenderStyle::BallAndStick);
        }
        if let Some(field) = self.scene.fields.first() {
            let z = field.shape[2] / 2;
            return crate::render_field_slice_3d_png(field, z, &camera);
        }
        Err("empty scene".into())
    }

    #[cfg(not(feature = "wgpu"))]
    pub fn render_frame(&self, _camera: crate::OrbitCamera) -> Result<Vec<u8>, String> {
        Err("wgpu feature disabled".into())
    }
}

/// Parse minimal XYZ format (element x y z per line).
pub fn parse_xyz(source: &str) -> Result<MoleculeGeometry, String> {
    let mut lines = source.lines().filter(|l| !l.trim().is_empty());
    let count: usize = lines
        .next()
        .ok_or("xyz: missing atom count")?
        .trim()
        .parse()
        .map_err(|_| "xyz: invalid atom count")?;
    let name = lines
        .next()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "molecule".into());
    let mut atoms = Vec::with_capacity(count);
    for (i, line) in lines.take(count).enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(format!("xyz: line {} expected element x y z", i + 3));
        }
        let z = element_symbol_to_z(parts[0])?;
        let x: f64 = parts[1].parse().map_err(|_| format!("xyz: bad x on line {}", i + 3))?;
        let y: f64 = parts[2].parse().map_err(|_| format!("xyz: bad y on line {}", i + 3))?;
        let zc: f64 = parts[3].parse().map_err(|_| format!("xyz: bad z on line {}", i + 3))?;
        atoms.push(AtomBall {
            element: z,
            x,
            y,
            z: zc,
            radius: 0.5,
        });
    }
    let mut mol = MoleculeGeometry {
        name,
        atoms,
        bonds: vec![],
    };
    infer_bonds(&mut mol);
    Ok(mol)
}

/// Parse XYZ and infer bonds from covalent radii.
pub fn parse_xyz_with_bonds(source: &str) -> Result<MoleculeGeometry, String> {
    parse_xyz(source)
}

/// Distance-based bond inference (1.15× sum of covalent radii).
pub fn infer_bonds(mol: &mut MoleculeGeometry) {
    let mut bonds = Vec::new();
    for i in 0..mol.atoms.len() {
        for j in (i + 1)..mol.atoms.len() {
            let a = &mol.atoms[i];
            let b = &mol.atoms[j];
            let dx = a.x - b.x;
            let dy = a.y - b.y;
            let dz = a.z - b.z;
            let dist = (dx * dx + dy * dy + dz * dz).sqrt();
            let cutoff = (crate::elements::covalent_radius(a.element)
                + crate::elements::covalent_radius(b.element))
                * 1.15;
            if dist > 0.4 && dist <= cutoff {
                bonds.push([i, j]);
            }
        }
    }
    mol.bonds = bonds;
}

pub fn element_symbol(z: u8) -> &'static str {
    crate::elements::z_to_symbol(z)
}

/// Parse molecular structure by format sniffing or caller hint.
pub fn parse_structure(source: &str, path_hint: Option<&str>) -> Result<MoleculeGeometry, String> {
    let hint = path_hint.unwrap_or("").to_ascii_lowercase();
    if hint.ends_with(".fchk") {
        return crate::fchk::parse_fchk_geometry(source);
    }
    if hint.ends_with(".log") || source.contains("Standard orientation:") {
        if let Ok(log) = crate::gaussian_log::parse_gaussian_log(source) {
            if let Some(geo) = log.geometry {
                return Ok(geo);
            }
        }
    }
    if hint.ends_with(".gjf") || hint.ends_with(".com") || source.trim_start().starts_with('#') {
        if let Ok(job) = crate::gjf::parse_gjf(source) {
            return Ok(job.geometry);
        }
    }
    if hint.ends_with(".pdb") || source.contains("ATOM  ") || source.contains("HETATM") {
        return crate::pdb::parse_pdb_with_bonds(source);
    }
    parse_xyz(source)
}

fn element_symbol_to_z(sym: &str) -> Result<u8, String> {
    crate::elements::symbol_to_z(sym).map_err(|e| format!("xyz: {e}"))
}
