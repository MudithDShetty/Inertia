/** Canvas 2D plots — line, scatter, histogram, contour heatmap. */

import { jetColormap } from "./colormap";

export interface PlotSeries {
  label: string;
  x: number[];
  y: number[];
}

export interface PlotData {
  title: string;
  xLabel: string;
  yLabel: string;
  series: PlotSeries[];
}

export type PlotMode = "line" | "scatter" | "histogram" | "contour";

export class PlotViewer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private data: PlotData | null = null;
  private mode: PlotMode = "line";

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d")!;
    window.addEventListener("resize", () => this.draw());
  }

  setMode(mode: PlotMode) {
    this.mode = mode;
    this.draw();
  }

  getMode() {
    return this.mode;
  }

  load(data: PlotData) {
    this.data = data;
    this.draw();
  }

  clear() {
    this.data = null;
    this.draw();
  }

  /** Append a scalar sample (live run streaming). */
  appendSample(label: string, x: number, y: number) {
    if (!this.data) {
      this.data = {
        title: "Run output",
        xLabel: "step",
        yLabel: "value",
        series: [{ label, x: [x], y: [y] }],
      };
    } else {
      let s = this.data.series.find((r) => r.label === label);
      if (!s) {
        s = { label, x: [], y: [] };
        this.data.series.push(s);
      }
      s.x.push(x);
      s.y.push(y);
      if (s.x.length > 200) {
        s.x.shift();
        s.y.shift();
      }
    }
    this.draw();
  }

  resize() {
    const wrap = this.canvas.parentElement;
    if (!wrap) return;
    const w = Math.max(200, wrap.clientWidth);
    const h = Math.max(160, wrap.clientHeight);
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
      this.draw();
    }
  }

  draw() {
    const ctx = this.ctx;
    const w = this.canvas.width;
    const h = this.canvas.height;
    ctx.fillStyle = "#1a1a2e";
    ctx.fillRect(0, 0, w, h);
    if (!this.data?.series.length) {
      ctx.fillStyle = "#666";
      ctx.font = "12px sans-serif";
      ctx.fillText("No plot data — Demo Plot, Run, or Run NB", 12, 24);
      return;
    }

    switch (this.mode) {
      case "histogram":
        this.drawHistogram(w, h);
        break;
      case "contour":
        this.drawContour(w, h);
        break;
      case "scatter":
        this.drawScatter(w, h);
        break;
      default:
        this.drawLine(w, h);
    }
  }

  private pad(w: number, h: number) {
    return { l: 48, r: 16, t: 28, b: 36, pw: w - 64, ph: h - 64 };
  }

  private axisExtents() {
    let xmin = Infinity;
    let xmax = -Infinity;
    let ymin = Infinity;
    let ymax = -Infinity;
    for (const s of this.data!.series) {
      for (let i = 0; i < s.x.length; i++) {
        xmin = Math.min(xmin, s.x[i]);
        xmax = Math.max(xmax, s.x[i]);
        ymin = Math.min(ymin, s.y[i]);
        ymax = Math.max(ymax, s.y[i]);
      }
    }
    if (!Number.isFinite(xmin)) return null;
    const xspan = xmax - xmin || 1;
    const yspan = ymax - ymin || 1;
    return {
      xmin,
      xmax,
      ymin: ymin - yspan * 0.05,
      ymax: ymax + yspan * 0.05,
      xspan,
      yspan: (ymax + yspan * 0.05) - (ymin - yspan * 0.05),
    };
  }

  private drawAxes(h: number, pad: ReturnType<PlotViewer["pad"]>) {
    const ctx = this.ctx;
    ctx.strokeStyle = "#3c3c3c";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(pad.l, pad.t);
    ctx.lineTo(pad.l, pad.t + pad.ph);
    ctx.lineTo(pad.l + pad.pw, pad.t + pad.ph);
    ctx.stroke();
    ctx.fillStyle = "#aaa";
    ctx.font = "11px sans-serif";
    ctx.textAlign = "center";
    ctx.fillText(this.data!.title, pad.l + pad.pw / 2, 16);
    ctx.save();
    ctx.translate(12, pad.t + pad.ph / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText(this.data!.yLabel, 0, 0);
    ctx.restore();
    ctx.fillText(this.data!.xLabel, pad.l + pad.pw / 2, h - 8);
  }

  private drawLine(w: number, h: number) {
    const pad = this.pad(w, h);
    const ext = this.axisExtents();
    if (!ext) return;
    this.drawAxes(h, pad);
    const colors = ["#4fc3f7", "#ffd700", "#81c784"];
    this.data!.series.forEach((s, si) => {
      this.ctx.strokeStyle = colors[si % colors.length];
      this.ctx.lineWidth = 2;
      this.ctx.beginPath();
      for (let i = 0; i < s.x.length; i++) {
        const px = pad.l + ((s.x[i] - ext.xmin) / ext.xspan) * pad.pw;
        const py = pad.t + pad.ph - ((s.y[i] - ext.ymin) / ext.yspan) * pad.ph;
        if (i === 0) this.ctx.moveTo(px, py);
        else this.ctx.lineTo(px, py);
      }
      this.ctx.stroke();
    });
  }

  private drawScatter(w: number, h: number) {
    const pad = this.pad(w, h);
    const ext = this.axisExtents();
    if (!ext) return;
    this.drawAxes(h, pad);
    const colors = ["#4fc3f7", "#ffd700", "#81c784"];
    this.data!.series.forEach((s, si) => {
      this.ctx.fillStyle = colors[si % colors.length];
      for (let i = 0; i < s.x.length; i++) {
        const px = pad.l + ((s.x[i] - ext.xmin) / ext.xspan) * pad.pw;
        const py = pad.t + pad.ph - ((s.y[i] - ext.ymin) / ext.yspan) * pad.ph;
        this.ctx.beginPath();
        this.ctx.arc(px, py, 3.5, 0, Math.PI * 2);
        this.ctx.fill();
      }
    });
  }

  private drawHistogram(w: number, h: number) {
    const pad = this.pad(w, h);
    const s = this.data!.series[0];
    if (!s?.y.length) return;
    const bins = 16;
    const lo = Math.min(...s.y);
    const hi = Math.max(...s.y);
    const span = hi - lo || 1;
    const counts = new Array(bins).fill(0);
    for (const v of s.y) {
      const t = Math.min(bins - 1, Math.floor(((v - lo) / span) * bins));
      counts[t]++;
    }
    const maxC = Math.max(...counts, 1);
    this.ctx.fillStyle = "#aaa";
    this.ctx.font = "11px sans-serif";
    this.ctx.textAlign = "center";
    this.ctx.fillText(`${this.data!.title} (histogram)`, pad.l + pad.pw / 2, 16);
    this.ctx.textAlign = "left";
    this.ctx.fillText(this.data!.yLabel, 12, pad.t + pad.ph / 2);
    this.ctx.strokeStyle = "#3c3c3c";
    this.ctx.strokeRect(pad.l, pad.t, pad.pw, pad.ph);
    const bw = pad.pw / bins;
    this.ctx.fillStyle = "#4fc3f7";
    for (let i = 0; i < bins; i++) {
      const bh = (counts[i] / maxC) * pad.ph;
      this.ctx.fillRect(pad.l + i * bw + 1, pad.t + pad.ph - bh, bw - 2, bh);
    }
  }

  /** 2D bin heatmap from first series (contour / density stub). */
  private drawContour(w: number, h: number) {
    const pad = this.pad(w, h);
    const s = this.data!.series[0];
    if (!s || s.x.length < 4) {
      this.ctx.fillStyle = "#666";
      this.ctx.fillText("Need ≥4 points for contour heatmap", pad.l, pad.t + 20);
      return;
    }
    const n = Math.floor(Math.sqrt(s.x.length));
    const nx = n >= 2 && n * n === s.x.length ? n : 24;
    const ny = nx;
    const grid = new Float64Array(nx * ny);
    let vmin = Infinity;
    let vmax = -Infinity;
    if (n >= 2 && n * n === s.x.length) {
      for (let i = 0; i < s.y.length; i++) {
        grid[i] = s.y[i];
        vmin = Math.min(vmin, s.y[i]);
        vmax = Math.max(vmax, s.y[i]);
      }
    } else {
      const ext = this.axisExtents()!;
      for (let i = 0; i < s.x.length; i++) {
        const ix = Math.min(
          nx - 1,
          Math.floor(((s.x[i] - ext.xmin) / ext.xspan) * nx),
        );
        const iy = Math.min(
          ny - 1,
          Math.floor(((s.y[i] - ext.ymin) / ext.yspan) * ny),
        );
        const idx = iy * nx + ix;
        grid[idx] += 1;
        vmin = Math.min(vmin, grid[idx]);
        vmax = Math.max(vmax, grid[idx]);
      }
    }
    const span = vmax - vmin || 1;
    this.ctx.fillStyle = "#aaa";
    this.ctx.font = "11px sans-serif";
    this.ctx.textAlign = "center";
    this.ctx.fillText(`${this.data!.title} (contour heatmap)`, pad.l + pad.pw / 2, 16);
    const cw = pad.pw / nx;
    const ch = pad.ph / ny;
    for (let j = 0; j < ny; j++) {
      for (let i = 0; i < nx; i++) {
        const v = grid[j * nx + i];
        const t = (v - vmin) / span;
        const [r, g, b] = jetColormap(t);
        this.ctx.fillStyle = `rgb(${r},${g},${b})`;
        this.ctx.fillRect(pad.l + i * cw, pad.t + (ny - 1 - j) * ch, cw + 0.5, ch + 0.5);
      }
    }
    this.ctx.strokeStyle = "#3c3c3c";
    this.ctx.strokeRect(pad.l, pad.t, pad.pw, pad.ph);
  }
}
