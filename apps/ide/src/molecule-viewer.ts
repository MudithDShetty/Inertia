import {
  debounce,
  pngBytesToDataUrl,
  renderMoleculeFrame,
  type IdeCamera,
} from "./wgpu-viewer";

export interface MoleculeAtom {
  element: number;
  symbol: string;
  x: number;
  y: number;
  z: number;
  radius: number;
}

export interface MoleculeData {
  name: string;
  path: string;
  atoms: MoleculeAtom[];
  bonds: [number, number][];
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

export class MoleculeViewer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private img: HTMLImageElement;
  private mol: MoleculeData | null = null;
  private yaw = 0.6;
  private pitch = 0.35;
  private zoom = 1.0;
  private dragging = false;
  private lastX = 0;
  private lastY = 0;
  private wgpuActive = false;
  private renderPending = false;
  private scheduleWgpuRender: () => void;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    this.img = document.createElement("img");
    this.img.className = "wgpu-frame";
    this.img.alt = "wgpu molecule view";
    this.img.style.display = "none";
    canvas.parentElement?.appendChild(this.img);
    this.scheduleWgpuRender = debounce(() => void this.requestWgpuFrame(), 32);
    this.bindEvents();
    this.resize();
    window.addEventListener("resize", () => this.resize());
  }

  private bindEvents() {
    const onPointerDown = (e: MouseEvent) => {
      this.dragging = true;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
    };
    const onPointerMove = (e: MouseEvent) => {
      if (!this.dragging) return;
      const dx = e.clientX - this.lastX;
      const dy = e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      this.yaw += dx * 0.01;
      this.pitch = Math.max(-1.2, Math.min(1.2, this.pitch + dy * 0.01));
      if (this.wgpuActive) {
        this.scheduleWgpuRender();
      } else {
        this.drawCanvas();
      }
    };
    const onPointerUp = () => {
      this.dragging = false;
    };

    this.canvas.addEventListener("mousedown", onPointerDown);
    this.img.addEventListener("mousedown", onPointerDown);
    window.addEventListener("mouseup", onPointerUp);
    window.addEventListener("mousemove", onPointerMove);
  }

  private cameraParams(): IdeCamera {
    const rect = this.canvas.parentElement?.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    return {
      yaw: this.yaw,
      pitch: this.pitch,
      zoom: this.zoom,
      width: Math.max(1, Math.floor((rect?.width ?? 640) * dpr)),
      height: Math.max(1, Math.floor((rect?.height ?? 480) * dpr)),
    };
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
    if (this.wgpuActive) {
      this.scheduleWgpuRender();
    } else {
      this.drawCanvas();
    }
  }

  load(mol: MoleculeData) {
    this.mol = mol;
    this.yaw = 0.6;
    this.pitch = 0.35;
    void this.requestWgpuFrame();
  }

  clear() {
    this.mol = null;
    this.wgpuActive = false;
    this.img.style.display = "none";
    this.canvas.style.display = "block";
    this.drawCanvas();
  }

  usesWgpu() {
    return this.wgpuActive;
  }

  private async requestWgpuFrame() {
    if (!this.mol?.path || this.renderPending) return;
    this.renderPending = true;
    const result = await renderMoleculeFrame(this.mol.path, this.cameraParams());
    this.renderPending = false;
    if (result?.png?.length) {
      this.wgpuActive = true;
      this.img.src = pngBytesToDataUrl(result.png);
      this.img.style.display = "block";
      this.canvas.style.display = "none";
    } else {
      this.wgpuActive = false;
      this.img.style.display = "none";
      this.canvas.style.display = "block";
      this.drawCanvas();
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
      ctx.fillText("Open a .xyz file to view", w / 2, h / 2);
      return;
    }

    const cx = w / 2;
    const cy = h / 2;
    let maxR = 0;
    for (const a of this.mol.atoms) {
      const r = Math.hypot(a.x, a.y, a.z);
      maxR = Math.max(maxR, r);
    }
    const scale = maxR > 0 ? (Math.min(w, h) * 0.35) / maxR : 40;

    const projected = this.mol.atoms.map((a) =>
      this.project(a.x, a.y, a.z, cx, cy, scale),
    );

    const bondItems = this.mol.bonds.map(([i, j]) => ({
      i,
      j,
      depth: (projected[i].depth + projected[j].depth) / 2,
    }));
    bondItems.sort((a, b) => a.depth - b.depth);

    ctx.lineWidth = 2;
    for (const b of bondItems) {
      const p1 = projected[b.i];
      const p2 = projected[b.j];
      ctx.strokeStyle = "#6a6a8a";
      ctx.beginPath();
      ctx.moveTo(p1.sx, p1.sy);
      ctx.lineTo(p2.sx, p2.sy);
      ctx.stroke();
    }

    const atomOrder = projected
      .map((p, i) => ({ ...p, i }))
      .sort((a, b) => a.depth - b.depth);

    for (const p of atomOrder) {
      const atom = this.mol.atoms[p.i];
      const colors = CPK[atom.symbol] ?? CPK.X;
      const r = Math.max(6, atom.radius * scale * 0.55);
      ctx.beginPath();
      ctx.arc(p.sx, p.sy, r, 0, Math.PI * 2);
      ctx.fillStyle = colors.fill;
      ctx.fill();
      ctx.strokeStyle = colors.stroke;
      ctx.lineWidth = 1.5;
      ctx.stroke();
      ctx.fillStyle = atom.symbol === "H" ? "#333" : "#fff";
      ctx.font = `bold ${Math.max(9, r * 0.7)}px Segoe UI, sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(atom.symbol, p.sx, p.sy);
    }

    ctx.fillStyle = "#9cdcfe";
    ctx.font = "11px Segoe UI, sans-serif";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    const backend = this.wgpuActive ? "wgpu 3D" : "canvas fallback";
    ctx.fillText(
      `${this.mol.name} — ${this.mol.atoms.length} atoms (${backend})`,
      8,
      8,
    );
    ctx.fillStyle = "#666";
    ctx.textAlign = "right";
    ctx.fillText("drag to orbit", w - 8, 8);
  }
}
