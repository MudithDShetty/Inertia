//! PhysicsLang visualization — circuit diagrams and plot IR.

mod camera;
mod circuit_svg;
mod field_slice;
mod marching_cubes;
mod pdb;
mod plots;
mod viewer3d;

#[cfg(feature = "wgpu")]
mod wgpu_render;

pub use camera::OrbitCamera;
pub use circuit_svg::{render_circuit_json_to_svg, render_circuit_svg, CircuitSvgOptions};
pub use field_slice::{
    demo_gaussian_field, extract_slice, jet_colormap, slice_to_rgba, FieldSlice, SliceAxis,
};
pub use marching_cubes::{extract_isosurface_stub, IsoMesh};
pub use pdb::{parse_pdb, parse_pdb_with_bonds};
pub use plots::{ConvergencePlot, PlotSeries};
pub use viewer3d::{
    element_symbol, infer_bonds, parse_structure, parse_xyz, parse_xyz_with_bonds, AtomBall,
    MoleculeGeometry, ScalarField, ViewerScene, WgpuViewerStub,
};

#[cfg(feature = "wgpu")]
pub use wgpu_render::{
    render_field_isosurface_png, render_field_slice_3d_png, render_field_slice_png,
    render_molecule_png,
};

#[cfg(test)]
mod tests;
