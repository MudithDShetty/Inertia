/** Z-matrix / Cartesian coordinate block editor for Gaussian inputs. */

import { invoke } from "@tauri-apps/api/core";

export async function openZMatrixEditor(
  path: string,
  onSaved: () => void,
  showTextModal: (
    title: string,
    initial: string,
    onSave: (text: string) => void,
  ) => void,
): Promise<void> {
  const block = await invoke<{ coordinate_type: string; lines: string[] }>(
    "gjf_get_coordinates",
    { path },
  );
  const label =
    block.coordinate_type === "z_matrix" ? "Z-matrix" : "Cartesian";
  showTextModal(
    `Edit ${label} — ${path.split(/[/\\]/).pop() ?? path}`,
    block.lines.join("\n"),
    (text) => {
      void (async () => {
        const lines = text
          .split("\n")
          .map((l) => l.trim())
          .filter(Boolean);
        await invoke("gjf_set_coordinates", { path, lines });
        onSaved();
      })();
    },
  );
}
