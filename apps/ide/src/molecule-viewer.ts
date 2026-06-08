import { invoke } from "@tauri-apps/api/core";
import {
  debounce,
  pngBytesToDataUrl,
  renderMoleculeFrame,
  type IdeCamera,
  type MolRenderStyle,
} from "./wgpu-viewer";

export interface MoleculeAtom {
  element: number;
  symbol: string;
  x: number;
  y: number;
  z: number;
  radius: number;
}

export interface ChemMeta {
  route?: string;
  title: string;
  charge?: number;
  multiplicity?: number;
  coordinate_type?: string;
  format: string;
  final_energy_hartree?: number;
  scf_cycles?: number;
  n_frequencies?: number;
  has_density?: boolean;
  has_mos?: boolean;
}

export type MeasureMode = "off" | "distance" | "angle" | "dihedral";

const BOHR_TO_ANGSTROM = 0.529177210903;

function toDisplayCoords(
  a: { x: number; y: number; z: number },
  chem?: ChemMeta,
): { x: number; y: number; z: number } {
  const gaussian = chem?.format === "fchk" || chem?.format === "log" || chem?.format === "gjf";
  if (!gaussian) return a;
  return { x: a.x * BOHR_TO_ANGSTROM, y: a.y * BOHR_TO_ANGSTROM, z: a.z * BOHR_TO_ANGSTROM };
}

function dist3(
  a: { x: number; y: number; z: number },
  b: { x: number; y: number; z: number },
): number {
  const dx = a.x - b.x;
  const dy = a.y - b.y;
  const dz = a.z - b.z;
  return Math.sqrt(dx * dx + dy * dy + dz * dz);
}

function angleDeg(
  a: { x: number; y: number; z: number },
  b: { x: number; y: number; z: number },
  c: { x: number; y: number; z: number },
): number {
  const v1 = { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z };
  const v2 = { x: c.x - b.x, y: c.y - b.y, z: c.z - b.z };
  const n1 = Math.hypot(v1.x, v1.y, v1.z);
  const n2 = Math.hypot(v2.x, v2.y, v2.z);
  if (n1 < 1e-12 || n2 < 1e-12) return 0;
  const dot = (v1.x * v2.x + v1.y * v2.y + v1.z * v2.z) / (n1 * n2);
  return (Math.acos(Math.max(-1, Math.min(1, dot))) * 180) / Math.PI;
}

function dihedralDeg(
  a: { x: number; y: number; z: number },
  b: { x: number; y: number; z: number },
  c: { x: number; y: number; z: number },
  d: { x: number; y: number; z: number },
): number {
  const b0 = { x: b.x - a.x, y: b.y - a.y, z: b.z - a.z };
  const b1 = { x: c.x - b.x, y: c.y - b.y, z: c.z - b.z };
  const b2 = { x: d.x - c.x, y: d.y - c.y, z: d.z - c.z };
  const n1len = Math.hypot(b1.x, b1.y, b1.z);
  if (n1len < 1e-12) return 0;
  const b1n = { x: b1.x / n1len, y: b1.y / n1len, z: b1.z / n1len };
  const dot0 = b0.x * b1n.x + b0.y * b1n.y + b0.z * b1n.z;
  const v = {
    x: b0.x - dot0 * b1n.x,
    y: b0.y - dot0 * b1n.y,
    z: b0.z - dot0 * b1n.z,
  };
  const dot2 = b2.x * b1n.x + b2.y * b1n.y + b2.z * b1n.z;
  const w = {
    x: b2.x - dot2 * b1n.x,
    y: b2.y - dot2 * b1n.y,
    z: b2.z - dot2 * b1n.z,
  };
  const x = v.x * w.x + v.y * w.y + v.z * w.z;
  const y =
    b1n.x * (v.y * w.z - v.z * w.y) +
    b1n.y * (v.z * w.x - v.x * w.z) +
    b1n.z * (v.x * w.y - v.y * w.x);
  return (Math.atan2(y, x) * 180) / Math.PI;
}

export interface MoleculeData {
  name: string;
  path: string;
  atoms: MoleculeAtom[];
  bonds: [number, number][];
  chem?: ChemMeta;
}

const CPK: Record<string, { fill: string; stroke: string }> = {
  H: { fill: "#ffffff", stroke: "#888888" },
  C: { fill: "#909090", stroke: "#606060" },
  N: { fill: "#3050f8", stroke: "#1030c8" },
  O: { fill: "#ff0d0d", stroke: "#c00000" },
  F: { fill: "#90e050", stroke: "#60b030" },
  S: { fill: "#ffff30", stroke: "#c0c000" },
  Cl: { fill: "#1ff01f", stroke: "#10a010" },
  Fe: { fill: "#e06633", stroke: "#a04020" },
  X: { fill: "#ff69b4", stroke: "#c04080" },
};

const VDW_SCALE: Record<string, number> = {
  H: 1.2,
  C: 1.7,
  N: 1.55,
  O: 1.52,
  F: 1.47,
  S: 1.8,
  Cl: 1.75,
  default: 1.6,
};

/** Match physlang-viz styled_geometry radii for consistent framing. */
function styledRadius(atom: MoleculeAtom, style: MolRenderStyle): number {
  const vdw = VDW_SCALE[atom.symbol] ?? VDW_SCALE.default;
  switch (style) {
    case "space_fill":
      return vdw * 0.5;
    case "wireframe":
      return 0.15;
    case "stick":
      return 0.12;
    default:
      return Math.max(vdw * 0.18, 0.18);
  }
}

export class MoleculeViewer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private img: HTMLImageElement;
  private mol: MoleculeData | null = null;
  private renderStyle: MolRenderStyle = "ball_and_stick";
  private yaw = 0.6;
  private pitch = 0.35;
  private zoom = 1.0;
  private dragging = false;
  private lastX = 0;
  private lastY = 0;
  private wgpuActive = false;
  private renderPending = false;
  private debouncedWgpu: () => void;
  private measureMode: MeasureMode = "off";
  private pickedAtoms: number[] = [];
  private measureText = "";
  private pointerDownX = 0;
  private pointerDownY = 0;
  private onMeasureChange: (() => void) | null = null;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    this.img = document.createElement("img");
    this.img.className = "wgpu-frame";
    this.img.alt = "wgpu molecule view";
    this.img.style.display = "none";
    canvas.parentElement?.appendChild(this.img);
    this.debouncedWgpu = debounce(() => void this.requestWgpuFrame(), 80);
    this.bindEvents();
    this.resize();
    window.addEventListener("resize", () => this.resize());
  }

  setRenderStyle(style: MolRenderStyle) {
    this.renderStyle = style;
    if (this.measureMode === "off") {
      void this.requestWgpuFrame();
    } else {
      this.drawMeasureOverlay();
    }
  }

  getRenderStyle() {
    return this.renderStyle;
  }

  resetView() {
    this.yaw = 0.6;
    this.pitch = 0.35;
    this.zoom = 1.0;
    if (this.measureMode === "off") {
      void this.requestWgpuFrame();
    } else {
      this.drawMeasureOverlay();
    }
  }

  setMeasureMode(mode: MeasureMode) {
    this.measureMode = mode;
    this.pickedAtoms = [];
    this.measureText = mode === "off" ? "" : `Pick atoms (${mode})`;
    if (mode === "off") {
      this.canvas.classList.remove("measure-overlay");
      void this.requestWgpuFrame();
    } else {
      this.canvas.classList.add("measure-overlay");
      this.showCanvasOverlay();
      this.drawMeasureOverlay();
    }
  }

  getMeasureMode() {
    return this.measureMode;
  }

  getMeasureText() {
    return this.measureText;
  }

  clearMeasure() {
    this.pickedAtoms = [];
    this.measureText = this.measureMode === "off" ? "" : `Pick atoms (${this.measureMode})`;
    if (this.wgpuActive) {
      this.drawMeasureOverlay();
    } else {
      this.drawCanvas();
    }
    this.onMeasureChange?.();
  }

  setMeasureCallback(cb: () => void) {
    this.onMeasureChange = cb;
  }

  private hideWgpu() {
    this.wgpuActive = false;
    this.img.style.display = "none";
    this.canvas.style.display = "block";
    this.canvas.classList.remove("measure-overlay");
  }

  private showWgpu() {
    this.wgpuActive = true;
    this.img.style.display = "block";
    if (this.measureMode === "off") {
      this.canvas.style.display = "none";
      this.canvas.classList.remove("measure-overlay");
    } else {
      this.showCanvasOverlay();
    }
  }

  private showCanvasOverlay() {
    this.canvas.style.display = "block";
    this.canvas.classList.add("measure-overlay");
  }

  private viewRect(): DOMRect {
    return (
      this.canvas.parentElement?.getBoundingClientRect() ??
      this.canvas.getBoundingClientRect()
    );
  }

  private screenToCamera(mx: number, my: number): [number, number] {
    const rect = this.viewRect();
    const cam = this.cameraParams();
    const sx = (mx / Math.max(rect.width, 1)) * cam.width;
    const sy = (my / Math.max(rect.height, 1)) * cam.height;
    return [sx, sy];
  }

  private pickLimit(): number {
    switch (this.measureMode) {
      case "distance":
        return 2;
      case "angle":
        return 3;
      case "dihedral":
        return 4;
      default:
        return 0;
    }
  }

  private updateMeasurement() {
    if (!this.mol || this.pickedAtoms.length === 0) return;
    const atoms = this.mol.atoms;
    const chem = this.mol.chem;
    const idx = this.pickedAtoms;
    const labels = idx.map((i) => atoms[i]?.symbol ?? "?");
    const pos = (i: number) => toDisplayCoords(atoms[i], chem);
    if (this.measureMode === "distance" && idx.length >= 2) {
      const d = dist3(pos(idx[0]), pos(idx[1]));
      this.measureText = `${labels[0]}–${labels[1]}: ${d.toFixed(3)} Å`;
    } else if (this.measureMode === "angle" && idx.length >= 3) {
      const deg = angleDeg(pos(idx[0]), pos(idx[1]), pos(idx[2]));
      this.measureText = `${labels[0]}–${labels[1]}–${labels[2]}: ${deg.toFixed(1)}°`;
    } else if (this.measureMode === "dihedral" && idx.length >= 4) {
      const deg = dihedralDeg(pos(idx[0]), pos(idx[1]), pos(idx[2]), pos(idx[3]));
      this.measureText = `${labels[0]}–${labels[1]}–${labels[2]}–${labels[3]}: ${deg.toFixed(1)}°`;
    } else {
      this.measureText = `Picked: ${labels.join(", ")}`;
    }
    this.onMeasureChange?.();
  }

  private async tryPickAtom(clientX: number, clientY: number) {
    if (!this.mol || this.measureMode === "off") return;
    const rect = this.viewRect();
    const mx = clientX - rect.left;
    const my = clientY - rect.top;
    let best = -1;

    if (this.wgpuActive && this.mol.path) {
      const [sx, sy] = this.screenToCamera(mx, my);
      try {
        const idx = await invoke<number | null>("pick_molecule_atom_cmd", {
          path: this.mol.path,
          camera: this.cameraParams(),
          style: this.renderStyle,
          screenX: sx,
          screenY: sy,
        });
        if (idx != null) best = idx;
      } catch {
        /* fall back to canvas pick */
      }
    }

    if (best < 0) {
      const w = rect.width;
      const h = rect.height;
      const cx = w / 2;
      const cy = h / 2;
      const [molCx, molCy, molCz] = this.moleculeCentroid();
      const extent = this.moleculeExtent();
      const scale = this.viewScale(w, h, extent);
      let bestD = Infinity;
      for (let i = 0; i < this.mol.atoms.length; i++) {
        const a = this.mol.atoms[i];
        const p = this.project(a.x - molCx, a.y - molCy, a.z - molCz, cx, cy, scale);
        const r = this.drawAtomRadius(a, scale);
        const d = Math.hypot(p.sx - mx, p.sy - my);
        if (d < r + 6 && d < bestD) {
          bestD = d;
          best = i;
        }
      }
    }

    if (best < 0) return;
    const limit = this.pickLimit();
    if (this.pickedAtoms.length >= limit) {
      this.pickedAtoms = [];
    }
    this.pickedAtoms.push(best);
    this.updateMeasurement();
    if (this.wgpuActive) {
      this.drawMeasureOverlay();
    } else {
      this.drawCanvas();
    }
  }

  private bindEvents() {
    const onPointerDown = (e: MouseEvent) => {
      this.dragging = true;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      this.pointerDownX = e.clientX;
      this.pointerDownY = e.clientY;
    };
    const onPointerMove = (e: MouseEvent) => {
      if (!this.dragging) return;
      const dx = e.clientX - this.lastX;
      const dy = e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      this.yaw += dx * 0.01;
      this.pitch = Math.max(-1.2, Math.min(1.2, this.pitch + dy * 0.01));
      if (this.measureMode === "off") {
        this.debouncedWgpu();
      } else if (this.wgpuActive) {
        this.drawMeasureOverlay();
      } else {
        this.drawCanvas();
      }
    };
    const onPointerUp = (e: MouseEvent) => {
      const moved = Math.hypot(e.clientX - this.pointerDownX, e.clientY - this.pointerDownY);
      if (moved < 5) {
        void this.tryPickAtom(e.clientX, e.clientY);
      } else if (this.measureMode === "off") {
        void this.requestWgpuFrame();
      }
      this.dragging = false;
    };
    const onWheel = (e: WheelEvent) => {
      if (!this.mol) return;
      e.preventDefault();
      const factor = e.deltaY > 0 ? 0.92 : 1.08;
      this.zoom = Math.max(0.25, Math.min(4.0, this.zoom * factor));
      if (this.measureMode === "off") {
        this.debouncedWgpu();
      } else if (this.wgpuActive) {
        this.drawMeasureOverlay();
      } else {
        this.drawCanvas();
      }
    };
    const onDblClick = () => this.resetView();

    this.canvas.addEventListener("mousedown", onPointerDown);
    this.img.addEventListener("mousedown", onPointerDown);
    this.canvas.addEventListener("wheel", onWheel, { passive: false });
    this.img.addEventListener("wheel", onWheel, { passive: false });
    this.canvas.addEventListener("dblclick", onDblClick);
    this.img.addEventListener("dblclick", onDblClick);
    window.addEventListener("mouseup", onPointerUp);
    window.addEventListener("mousemove", onPointerMove);
  }

  private cameraParams(): IdeCamera {
    const rect = this.canvas.parentElement?.getBoundingClientRect();
    const cssW = rect?.width ?? 640;
    const cssH = rect?.height ?? 480;
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const maxDim = 720;
    const w = Math.max(1, Math.min(Math.floor(cssW * dpr), maxDim));
    const h = Math.max(1, Math.min(Math.floor(cssH * dpr), maxDim));
    return {
      yaw: this.yaw,
      pitch: this.pitch,
      zoom: this.zoom,
      width: w,
      height: h,
    };
  }

  private moleculeCentroid(): [number, number, number] {
    if (!this.mol?.atoms.length) return [0, 0, 0];
    let sx = 0;
    let sy = 0;
    let sz = 0;
    for (const a of this.mol.atoms) {
      sx += a.x;
      sy += a.y;
      sz += a.z;
    }
    const n = this.mol.atoms.length;
    return [sx / n, sy / n, sz / n];
  }

  private moleculeExtent(): number {
    if (!this.mol?.atoms.length) return 1;
    const [cx, cy, cz] = this.moleculeCentroid();
    let maxR = 0;
    for (const a of this.mol.atoms) {
      const d =
        Math.hypot(a.x - cx, a.y - cy, a.z - cz) +
        styledRadius(a, this.renderStyle);
      maxR = Math.max(maxR, d);
    }
    const pad = this.renderStyle === "space_fill" ? 1.85 : 1.55;
    return Math.max(maxR * pad, 0.5);
  }

  private viewScale(w: number, h: number, extent: number): number {
    return ((Math.min(w, h) * 0.46) / extent) * this.zoom;
  }

  resize() {
    const rect = this.canvas.parentElement?.getBoundingClientRect();
    if (!rect) return;
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = rect.width * dpr;
    this.canvas.height = rect.height * dpr;
    this.canvas.style.width = `${rect.width}px`;
    this.canvas.style.height = `${rect.height}px`;
    this.img.style.width = `${rect.width}px`;
    this.img.style.height = `${rect.height}px`;
    this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    this.drawCanvas();
  }

  load(mol: MoleculeData, useWgpu = true) {
    this.mol = mol;
    this.yaw = 0.6;
    this.pitch = 0.35;
    this.zoom = 1.0;
    this.wgpuActive = false;
    this.hideWgpu();
    this.drawCanvas();
    if (useWgpu && this.measureMode === "off") {
      void this.requestWgpuFrame();
    }
  }

  loadDirect(mol: MoleculeData) {
    this.load(mol, false);
  }

  clear() {
    this.mol = null;
    this.wgpuActive = false;
    this.hideWgpu();
    this.drawCanvas();
  }

  private async requestWgpuFrame() {
    if (!this.mol?.path || this.renderPending || this.measureMode !== "off") return;
    this.renderPending = true;
    const result = await renderMoleculeFrame(
      this.mol.path,
      this.cameraParams(),
      this.renderStyle,
    );
    this.renderPending = false;
    if (this.measureMode !== "off" || !result?.png?.length) return;
    this.img.src = pngBytesToDataUrl(result.png);
    this.showWgpu();
  }

  private drawAtomRadius(atom: MoleculeAtom, scale: number): number {
    const r = styledRadius(atom, this.renderStyle);
    switch (this.renderStyle) {
      case "wireframe":
      case "stick":
        return Math.max(3, scale * 0.05);
      default:
        return Math.max(4, r * scale);
    }
  }

  private drawBonds(projected: { sx: number; sy: number; depth: number }[], bonds: [number, number][]) {
    if (this.renderStyle === "space_fill") return;
    const ctx = this.ctx;
    const bondItems = bonds.map(([i, j]) => ({
      i,
      j,
      depth: (projected[i].depth + projected[j].depth) / 2,
    }));
    bondItems.sort((a, b) => a.depth - b.depth);
    ctx.lineWidth = this.renderStyle === "wireframe" ? 1 : 2;
    for (const b of bondItems) {
      const p1 = projected[b.i];
      const p2 = projected[b.j];
      ctx.strokeStyle = "#6a6a8a";
      ctx.beginPath();
      ctx.moveTo(p1.sx, p1.sy);
      ctx.lineTo(p2.sx, p2.sy);
      ctx.stroke();
    }
  }

  private project(x: number, y: number, z: number, cx: number, cy: number, scale: number) {
    const x1 = x * Math.cos(this.yaw) - z * Math.sin(this.yaw);
    const z1 = x * Math.sin(this.yaw) + z * Math.cos(this.yaw);
    const y1 = y * Math.cos(this.pitch) - z1 * Math.sin(this.pitch);
    return { sx: cx + x1 * scale, sy: cy - y1 * scale, depth: z1 };
  }

  private drawCanvas() {
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    const ctx = this.ctx;
    ctx.fillStyle = "#1a1a2e";
    ctx.fillRect(0, 0, w, h);

    if (!this.mol || this.mol.atoms.length === 0) {
      ctx.fillStyle = "#666";
      ctx.font = "12px Segoe UI, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText("Open .xyz, .pdb, or .gjf to view", w / 2, h / 2);
      return;
    }

    const cx = w / 2;
    const cy = h / 2;
    const [molCx, molCy, molCz] = this.moleculeCentroid();
    const extent = this.moleculeExtent();
    const scale = this.viewScale(w, h, extent);

    const projected = this.mol.atoms.map((a) =>
      this.project(a.x - molCx, a.y - molCy, a.z - molCz, cx, cy, scale),
    );

    this.drawBonds(projected, this.mol.bonds);

    const atomOrder = projected
      .map((p, i) => ({ ...p, i }))
      .sort((a, b) => a.depth - b.depth);

    const showLabels = this.renderStyle !== "space_fill" && this.renderStyle !== "wireframe";

    for (const p of atomOrder) {
      const atom = this.mol.atoms[p.i];
      const colors = CPK[atom.symbol] ?? CPK.X;
      const r = this.drawAtomRadius(atom, scale);
      ctx.beginPath();
      ctx.arc(p.sx, p.sy, r, 0, Math.PI * 2);
      ctx.fillStyle = colors.fill;
      ctx.fill();
      if (this.renderStyle !== "space_fill") {
        ctx.strokeStyle = colors.stroke;
        ctx.lineWidth = 1.5;
        ctx.stroke();
      }
      if (showLabels && r > 5) {
        ctx.fillStyle = atom.symbol === "H" ? "#333" : "#fff";
        ctx.font = `bold ${Math.max(9, r * 0.7)}px Segoe UI, sans-serif`;
        ctx.textAlign = "center";
        ctx.textBaseline = "middle";
        ctx.fillText(atom.symbol, p.sx, p.sy);
      }
    }

    ctx.fillStyle = "#9cdcfe";
    ctx.font = "11px Segoe UI, sans-serif";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillText(
      `${this.mol.name} — ${this.mol.atoms.length} atoms (${this.renderStyle})`,
      8,
      8,
    );
    if (this.measureText) {
      ctx.fillStyle = "#ffd700";
      ctx.fillText(this.measureText, 8, 24);
    }
    if (this.pickedAtoms.length > 0) {
      ctx.strokeStyle = "#ffd700";
      ctx.lineWidth = 2;
      for (const i of this.pickedAtoms) {
        const p = projected[i];
        ctx.beginPath();
        ctx.arc(p.sx, p.sy, 10, 0, Math.PI * 2);
        ctx.stroke();
      }
    }
    ctx.fillStyle = "#666";
    ctx.textAlign = "right";
    ctx.fillText("drag · scroll zoom · dbl-click reset", w - 8, 8);
  }

  /** Transparent overlay for measurement rings when wgpu frame is visible. */
  private drawMeasureOverlay() {
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    const ctx = this.ctx;
    ctx.clearRect(0, 0, w, h);
    if (!this.mol || this.measureMode === "off") return;

    const cx = w / 2;
    const cy = h / 2;
    const [molCx, molCy, molCz] = this.moleculeCentroid();
    const extent = this.moleculeExtent();
    const scale = this.viewScale(w, h, extent);
    const projected = this.mol.atoms.map((a) =>
      this.project(a.x - molCx, a.y - molCy, a.z - molCz, cx, cy, scale),
    );

    if (this.measureText) {
      ctx.fillStyle = "#ffd700";
      ctx.font = "11px Segoe UI, sans-serif";
      ctx.textAlign = "left";
      ctx.textBaseline = "top";
      ctx.fillText(this.measureText, 8, 8);
    }
    if (this.pickedAtoms.length > 0) {
      ctx.strokeStyle = "#ffd700";
      ctx.lineWidth = 2;
      for (const i of this.pickedAtoms) {
        const p = projected[i];
        ctx.beginPath();
        ctx.arc(p.sx, p.sy, 10, 0, Math.PI * 2);
        ctx.stroke();
      }
    }
  }
}
