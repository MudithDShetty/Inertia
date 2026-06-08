/** Jet colormap — matches physlang-viz field_slice::jet_colormap */
export function jetColormap(t: number): [number, number, number, number] {
  const x = Math.max(0, Math.min(1, t));
  const r = Math.max(0, Math.min(1, 1.5 - Math.abs(4 * x - 3)));
  const g = Math.max(0, Math.min(1, 1.5 - Math.abs(4 * x - 2)));
  const b = Math.max(0, Math.min(1, 1.5 - Math.abs(4 * x - 1)));
  return [
    Math.round(r * 255),
    Math.round(g * 255),
    Math.round(b * 255),
    255,
  ];
}
