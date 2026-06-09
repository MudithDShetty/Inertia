//! PhysicsLang visualization — circuit diagrams and plot IR.

mod camera;
mod circuit_svg;
mod cube;
mod elements;
mod field_slice;
mod fchk;
mod fchk_basis;
mod fchk_grid;
mod gaussian_log;
mod gjf;
mod marching_cubes;
mod pdb;
mod pick;
mod plots;
mod vibration;
mod viewer3d;
mod volume;
mod vtk;

#[cfg(feature = "wgpu")]
mod wgpu_render;

pub use camera::OrbitCamera;
pub use circuit_svg::{render_circuit_json_to_svg, render_circuit_svg, CircuitSvgOptions};
pub use cube::{cube_to_scalar_field, parse_cube, scalar_field_to_cube, CubeVolume};
pub use elements::{covalent_radius, symbol_to_z, vdw_radius, z_to_symbol};
pub use field_slice::{
    demo_gaussian_field, extract_slice, jet_colormap, slice_to_rgba, FieldSlice, SliceAxis,
};
pub use fchk::{parse_fchk, parse_fchk_geometry, FchkFile};
pub use fchk_grid::{
    fchk_density_field, fchk_density_field_from_geometry, fchk_esp_field, fchk_mo_field,
    promolecule_density_field,
};
pub use fchk_basis::{basis_from_fchk, gto_mo_at, BasisSet};
pub use gaussian_log::{parse_gaussian_log, GaussianLogResult};
pub use gjf::{
    parse_gjf, parse_gjf_geometry, styled_geometry, extract_gjf_coordinate_block,
    replace_gjf_coordinate_block, CoordinateType, GaussianInput, MolRenderStyle,
};
pub use vibration::{
    animate_geometry, parse_log_vibrations, NormalMode, VibrationData,
};
pub use marching_cubes::{extract_isosurface, extract_isosurface_stepped, extract_isosurface_stub, IsoMesh};
pub use pdb::{parse_pdb, parse_pdb_with_bonds};
pub use pick::pick_molecule_atom;
pub use plots::{ConvergencePlot, PlotSeries};
pub use viewer3d::{
    element_symbol, infer_bonds, parse_structure, parse_xyz, parse_xyz_with_bonds, AtomBall,
    MoleculeGeometry, ScalarField, ViewerScene, WgpuViewerStub,
};
pub use volume::{ray_march_rgba, VolumeRenderConfig};
pub use vtk::scalar_field_to_vtk;

#[cfg(feature = "wgpu")]
pub use wgpu_render::{
    render_field_isosurface_png, render_field_mo_isosurface_png, render_field_slice_3d_png,
    render_field_slice_png, render_field_volume_png, render_molecule_png,
};

#[cfg(test)]
mod tests;
