import { jetColormap } from "./colormap";
import {
  debounce,
  pngBytesToDataUrl,
  renderFieldFrame,
  type IdeCamera,
} from "./wgpu-viewer";

export interface FieldSliceData {
  name: string;
  path: string;
  axis: string;
  index: number;
  width: number;
  height: number;
  values: number[];
  min: number;
  max: number;
  depth: number;
  wgpu_png?: number[];
}

export type FieldViewMode = "slice2d" | "slice3d" | "isosurface";

export class FieldViewer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private img: HTMLImageElement;
  private field: FieldSliceData | null = null;
  private sliceIndex = 0;
  private maxIndex = 0;
  private yaw = 0.6;
  private pitch = 0.35;
  private zoom = 1.0;
  private isoLevel = 0.35;
  private isoSign = 1;
  private dragging = false;
  private lastX = 0;
  private lastY = 0;
  private viewMode: FieldViewMode = "slice3d";
  private renderPending = false;
  private debouncedWgpu: () => void;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    this.img = document.createElement("img");
    this.img.className = "wgpu-frame";
    this.img.alt = "wgpu field view";
    this.img.style.display = "none";
    canvas.parentElement?.appendChild(this.img);
    this.debouncedWgpu = debounce(() => void this.requestWgpuFrame(), 80);
    this.bindEvents();
    this.resize();
    window.addEventListener("resize", () => this.resize());
  }

  private isWgpuMode() {
    return this.viewMode === "slice3d" || this.viewMode === "isosurface";
  }

  private showCanvas() {
    this.img.style.display = "none";
    this.canvas.style.display = "block";
  }

  private showWgpu() {
    this.img.style.display = "block";
    this.canvas.style.display = "none";
  }

  private bindEvents() {
    const onPointerDown = (e: MouseEvent) => {
      if (this.viewMode === "slice2d") return;
      this.dragging = true;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
    };
    const onPointerMove = (e: MouseEvent) => {
      if (!this.dragging || this.viewMode === "slice2d") return;
      const dx = e.clientX - this.lastX;
      const dy = e.clientY - this.lastY;
      this.lastX = e.clientX;
      this.lastY = e.clientY;
      if (Math.abs(dx) <= 1 && Math.abs(dy) <= 1) return;
      this.yaw += dx * 0.01;
      this.pitch = Math.max(-1.2, Math.min(1.2, this.pitch + dy * 0.01));
      this.debouncedWgpu();
    };
    const onPointerUp = () => {
      if (this.dragging && this.isWgpuMode() && this.field?.path) {
        void this.requestWgpuFrame();
      }
      this.dragging = false;
    };
    const onWheel = (e: WheelEvent) => {
      if (!this.field || this.viewMode === "slice2d") return;
      e.preventDefault();
      const factor = e.deltaY > 0 ? 0.92 : 1.08;
      this.zoom = Math.max(0.25, Math.min(4.0, this.zoom * factor));
      this.debouncedWgpu();
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

  resetView() {
    this.yaw = 0.6;
    this.pitch = 0.35;
    this.zoom = 1.0;
    if (this.isWgpuMode() && this.field?.path) {
      void this.requestWgpuFrame();
    } else {
      this.draw();
    }
  }

  setViewMode(mode: FieldViewMode) {
    this.viewMode = mode;
    if (mode === "slice2d") {
      this.showCanvas();
      this.draw();
    } else {
      void this.requestWgpuFrame();
    }
  }

  setIsoLevel(level: number) {
    this.isoLevel = Math.max(0.05, Math.min(0.95, level));
    if (this.viewMode === "isosurface") {
      this.debouncedWgpu();
    }
  }

  setIsoSign(sign: 1 | -1) {
    this.isoSign = sign;
    if (this.viewMode === "isosurface") {
      this.debouncedWgpu();
    }
  }

  getIsoSign() {
    return this.isoSign;
  }

  getIsoLevel() {
    return this.isoLevel;
  }

  getViewMode() {
    return this.viewMode;
  }

  private cameraParams(): IdeCamera {
    const rect = this.canvas.parentElement?.getBoundingClientRect();
    const cssW = rect?.width ?? 640;
    const cssH = rect?.height ?? 480;
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const maxDim = 720;
    return {
      yaw: this.yaw,
      pitch: this.pitch,
      zoom: this.zoom,
      width: Math.max(1, Math.min(Math.floor(cssW * dpr), maxDim)),
      height: Math.max(1, Math.min(Math.floor(cssH * dpr), maxDim)),
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
    if (this.viewMode === "slice2d") {
      this.draw();
    } else if (this.field) {
      void this.requestWgpuFrame();
    }
  }

  /** Full load — resets camera and switches to wgpu on 3D modes. */
  load(field: FieldSliceData, maxIndex: number) {
    this.field = field;
    this.sliceIndex = field.index;
    this.maxIndex = maxIndex;
    this.yaw = 0.6;
    this.pitch = 0.35;
    this.zoom = 1.0;
    this.showCanvas();
    this.draw();
    if (this.isWgpuMode()) {
      void this.requestWgpuFrame();
    }
  }

  /** Slice change — preserve camera and view mode. */
  updateSlice(field: FieldSliceData) {
    this.field = field;
    this.sliceIndex = field.index;
    if (this.viewMode === "slice2d") {
      this.draw();
    } else {
      void this.requestWgpuFrame();
    }
  }

  setSlice(index: number) {
    this.sliceIndex = Math.max(0, Math.min(this.maxIndex, index));
    if (this.viewMode === "slice2d") {
      this.draw();
    } else {
      void this.requestWgpuFrame();
    }
  }

  getSliceIndex() {
    return this.sliceIndex;
  }

  private isMoField() {
    return this.field?.path.includes("|mo:") ?? false;
  }

  private async requestWgpuFrame() {
    if (!this.field?.path || this.viewMode === "slice2d" || this.renderPending) return;
    this.renderPending = true;
    const mode = this.viewMode === "isosurface" ? "isosurface" : "slice";
    const moDual = this.viewMode === "isosurface" && this.isMoField();
    const result = await renderFieldFrame(
      this.field.path,
      this.sliceIndex,
      this.cameraParams(),
      mode,
      this.viewMode === "isosurface" ? this.isoLevel : undefined,
      moDual ? undefined : this.viewMode === "isosurface" ? this.isoSign : undefined,
      moDual ? true : undefined,
    );
    this.renderPending = false;
    if (!result?.png?.length) {
      this.showCanvas();
      if (this.viewMode === "slice3d") {
        this.drawSlice3dCanvas();
      } else {
        this.draw();
      }
      return;
    }
    this.img.src = pngBytesToDataUrl(result.png);
    this.showWgpu();
  }

  drawWithValues(
    values: number[],
    width: number,
    height: number,
    min: number,
    max: number,
    label: string,
  ) {
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    const ctx = this.ctx;
    ctx.fillStyle = "#1a1a2e";
    ctx.fillRect(0, 0, w, h);

    if (values.length === 0) {
      ctx.fillStyle = "#666";
      ctx.font = "12px Segoe UI, sans-serif";
      ctx.textAlign = "center";
      ctx.fillText("No field loaded", w / 2, h / 2);
      return;
    }

    const span = Math.max(max - min, 1e-12);
    const img = ctx.createImageData(width, height);
    for (let i = 0; i < values.length; i++) {
      const t = (values[i] - min) / span;
      const [r, g, b, a] = jetColormap(t);
      const o = i * 4;
      img.data[o] = r;
      img.data[o + 1] = g;
      img.data[o + 2] = b;
      img.data[o + 3] = a;
    }

    const off = document.createElement("canvas");
    off.width = width;
    off.height = height;
    off.getContext("2d")!.putImageData(img, 0, 0);

    const scale = Math.min((w - 16) / width, (h - 40) / height);
    const dw = width * scale;
    const dh = height * scale;
    const dx = (w - dw) / 2;
    const dy = 24 + (h - 24 - dh) / 2;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(off, dx, dy, dw, dh);

    ctx.fillStyle = "#9cdcfe";
    ctx.font = "11px Segoe UI, sans-serif";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillText(label, 8, 8);
    ctx.fillStyle = "#666";
    ctx.textAlign = "right";
    const isoNote =
      this.viewMode === "isosurface"
        ? this.isMoField()
          ? ` · MO ±${(this.isoLevel * 100).toFixed(0)}%`
          : ` · iso ${this.isoSign > 0 ? "+" : "-"}${(this.isoLevel * 100).toFixed(0)}%`
        : "";
    ctx.fillText(
      `${min.toExponential(2)} … ${max.toExponential(2)}${isoNote}`,
      w - 8,
      8,
    );
  }

  draw() {
    if (!this.field) {
      this.drawWithValues([], 0, 0, 0, 1, "Scalar field");
      return;
    }
    if (this.viewMode === "slice3d") {
      this.drawSlice3dCanvas();
      return;
    }
    const f = this.field;
    this.drawWithValues(
      f.values,
      f.width,
      f.height,
      f.min,
      f.max,
      `${f.name} — ${f.axis} slice ${this.sliceIndex} (2D heatmap)`,
    );
  }

  /** Canvas fallback: pseudo-3D Z-slice plane when wgpu is unavailable. */
  private drawSlice3dCanvas() {
    if (!this.field) return;
    const f = this.field;
    const w = this.canvas.clientWidth;
    const h = this.canvas.clientHeight;
    const ctx = this.ctx;
    ctx.fillStyle = "#1a1a2e";
    ctx.fillRect(0, 0, w, h);

    const span = Math.max(f.max - f.min, 1e-12);
    const cx = w / 2;
    const cy = h / 2 + 12;
    const planeW = Math.min(w, h) * 0.55 * this.zoom;
    const planeH = planeW * (f.height / Math.max(f.width, 1));
    const cosY = Math.cos(this.yaw);
    const sinY = Math.sin(this.yaw);
    const cosP = Math.cos(this.pitch);
    const sinP = Math.sin(this.pitch);

    const corners = [
      [-planeW / 2, -planeH / 2],
      [planeW / 2, -planeH / 2],
      [planeW / 2, planeH / 2],
      [-planeW / 2, planeH / 2],
    ].map(([x, y]) => {
      const x1 = x * cosY;
      const z1 = x * sinY;
      const y1 = y * cosP - z1 * sinP;
      const z2 = y * sinP + z1 * cosP;
      return { sx: cx + x1, sy: cy - y1, depth: z2 };
    });
    corners.sort((a, b) => a.depth - b.depth);

    const off = document.createElement("canvas");
    off.width = f.width;
    off.height = f.height;
    const octx = off.getContext("2d")!;
    const img = octx.createImageData(f.width, f.height);
    for (let i = 0; i < f.values.length; i++) {
      const t = (f.values[i] - f.min) / span;
      const [r, g, b, a] = jetColormap(t);
      const o = i * 4;
      img.data[o] = r;
      img.data[o + 1] = g;
      img.data[o + 2] = b;
      img.data[o + 3] = a;
    }
    octx.putImageData(img, 0, 0);

    for (const c of corners) {
      ctx.save();
      ctx.translate(c.sx, c.sy);
      ctx.globalAlpha = 0.15;
      ctx.fillStyle = "#444";
      ctx.fillRect(-2, -2, 4, 4);
      ctx.restore();
    }

    ctx.save();
    ctx.translate(cx, cy);
    const hw = planeW / 2;
    const hh = planeH / 2;
    ctx.transform(cosY, sinP * 0.35, -sinY * 0.35, cosP, 0, 0);
    ctx.drawImage(off, -hw, -hh, planeW, planeH);
    ctx.restore();

    ctx.fillStyle = "#9cdcfe";
    ctx.font = "11px Segoe UI, sans-serif";
    ctx.textAlign = "left";
    ctx.textBaseline = "top";
    ctx.fillText(
      `${f.name} — ${f.axis} slice ${this.sliceIndex} (3D canvas fallback)`,
      8,
      8,
    );
    ctx.fillStyle = "#666";
    ctx.textAlign = "right";
    ctx.fillText(
      `${f.min.toExponential(2)} … ${f.max.toExponential(2)}`,
      w - 8,
      8,
    );
  }
}
