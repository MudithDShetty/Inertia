import { invoke } from "@tauri-apps/api/core";

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

export async function renderMoleculeFrame(
  path: string,
  camera: IdeCamera,
): Promise<RenderFrameResult | null> {
  try {
    return await invoke<RenderFrameResult>("render_molecule_frame", { path, camera });
  } catch {
    return null;
  }
}

export async function renderFieldFrame(
  path: string,
  index: number,
  camera: IdeCamera,
  mode?: "slice" | "isosurface",
): Promise<RenderFrameResult | null> {
  try {
    return await invoke<RenderFrameResult>("render_field_frame", {
      path,
      index,
      camera,
      mode: mode ?? "slice",
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
