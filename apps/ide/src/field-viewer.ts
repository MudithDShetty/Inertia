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
  private dragging = false;
  private lastX = 0;
  private lastY = 0;
  private viewMode: FieldViewMode = "slice3d";
  private wgpuActive = false;
  private renderPending = false;
  private scheduleWgpuRender: () => void;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    this.img = document.createElement("img");
    this.img.className = "wgpu-frame";
    this.img.alt = "wgpu field view";
    this.img.style.display = "none";
    canvas.parentElement?.appendChild(this.img);
    this.scheduleWgpuRender = debounce(() => void this.requestWgpuFrame(), 32);
    this.bindEvents();
    this.resize();
    window.addEventListener("resize", () => this.resize());
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
      this.yaw += dx * 0.01;
      this.pitch = Math.max(-1.2, Math.min(1.2, this.pitch + dy * 0.01));
      this.scheduleWgpuRender();
    };
    const onPointerUp = () => {
      this.dragging = false;
    };
    this.canvas.addEventListener("mousedown", onPointerDown);
    this.img.addEventListener("mousedown", onPointerDown);
    window.addEventListener("mouseup", onPointerUp);
    window.addEventListener("mousemove", onPointerMove);
  }

  setViewMode(mode: FieldViewMode) {
    this.viewMode = mode;
    if (mode === "slice2d") {
      this.wgpuActive = false;
      this.img.style.display = "none";
      this.canvas.style.display = "block";
      this.draw();
    } else {
      void this.requestWgpuFrame();
    }
  }

  getViewMode() {
    return this.viewMode;
  }

  usesWgpu() {
    return this.wgpuActive;
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
    if (this.viewMode !== "slice2d" && this.field) {
      this.scheduleWgpuRender();
    } else {
      this.draw();
    }
  }

  load(field: FieldSliceData, maxIndex: number) {
    this.field = field;
    this.sliceIndex = field.index;
    this.maxIndex = maxIndex;
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
      this.scheduleWgpuRender();
    }
  }

  getSliceIndex() {
    return this.sliceIndex;
  }

  private async requestWgpuFrame() {
    if (!this.field?.path || this.viewMode === "slice2d" || this.renderPending) return;
    this.renderPending = true;
    const mode = this.viewMode === "isosurface" ? "isosurface" : "slice";
    const result = await renderFieldFrame(
      this.field.path,
      this.sliceIndex,
      this.cameraParams(),
      mode,
    );
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
      this.draw();
    }
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
    ctx.fillText(`${min.toExponential(2)} … ${max.toExponential(2)}`, w - 8, 8);
  }

  draw() {
    if (!this.field) {
      this.drawWithValues([], 0, 0, 0, 1, "Scalar field");
      return;
    }
    const f = this.field;
    const modeLabel =
      this.viewMode === "isosurface"
        ? "isosurface"
        : this.viewMode === "slice3d"
          ? "wgpu 3D slice"
          : "2D heatmap";
    this.drawWithValues(
      f.values,
      f.width,
      f.height,
      f.min,
      f.max,
      `${f.name} — ${f.axis} slice ${this.sliceIndex} (${modeLabel})`,
    );
  }
}
