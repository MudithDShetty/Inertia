import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export interface IdeCamera {
  yaw: number;
  pitch: number;
  zoom: number;
  width: number;
  height: number;
}

export interface RenderFrameResult {
  png: number[];
  backend: string;
}

export function pngBytesToDataUrl(png: number[]): string {
  const bytes = new Uint8Array(png);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return `data:image/png;base64,${btoa(binary)}`;
}

export type MolRenderStyle =
  | "ball_and_stick"
  | "wireframe"
  | "space_fill"
  | "stick";

export async function renderMoleculeFrame(
  path: string,
  camera: IdeCamera,
  style: MolRenderStyle = "ball_and_stick",
): Promise<RenderFrameResult | null> {
  try {
    return await invoke<RenderFrameResult>("render_molecule_frame", {
      path,
      camera,
      style,
    });
  } catch {
    return null;
  }
}

export async function renderFieldFrame(
  path: string,
  index: number,
  camera: IdeCamera,
  mode?: "slice" | "isosurface" | "volume",
  isoLevel?: number,
  isoSign?: number,
  isoDual?: boolean,
): Promise<RenderFrameResult | null> {
  try {
    return await invoke<RenderFrameResult>("render_field_frame", {
      path,
      index,
      camera,
      mode: mode ?? "slice",
      isoLevel,
      isoSign,
      isoDual,
    });
  } catch {
    return null;
  }
}

export function debounce<T extends (...args: never[]) => void>(fn: T, ms: number): T {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return ((...args: Parameters<T>) => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  }) as T;
}

/** Save PNG bytes via native save dialog. Returns chosen path or null if cancelled. */
export async function savePngBytes(
  png: number[],
  defaultPath: string,
): Promise<string | null> {
  const selected = await save({
    filters: [{ name: "PNG image", extensions: ["png"] }],
    defaultPath,
  });
  if (!selected) return null;
  await invoke("write_binary_file", { path: selected, data: png });
  return selected;
}

/** Export a canvas element as PNG (2D fallback / vibration modes). */
export async function saveCanvasPng(
  canvas: HTMLCanvasElement,
  defaultPath: string,
): Promise<string | null> {
  const blob = await new Promise<Blob | null>((resolve) =>
    canvas.toBlob(resolve, "image/png"),
  );
  if (!blob) return null;
  const png = Array.from(new Uint8Array(await blob.arrayBuffer()));
  return savePngBytes(png, defaultPath);
}
