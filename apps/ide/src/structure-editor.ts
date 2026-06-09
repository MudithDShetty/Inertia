/** Visual atom coordinate table (Cartesian Å) — structure builder stub. */

import { invoke } from "@tauri-apps/api/core";
import type { MoleculeAtom, MoleculeData } from "./molecule-viewer";

const BOHR_TO_ANGSTROM = 0.529177210903;

function displayCoord(a: MoleculeAtom, gaussian: boolean): number[] {
  if (!gaussian) return [a.x, a.y, a.z];
  return [a.x * BOHR_TO_ANGSTROM, a.y * BOHR_TO_ANGSTROM, a.z * BOHR_TO_ANGSTROM];
}

function toInternalCoord(x: number, y: number, z: number, gaussian: boolean): [number, number, number] {
  if (!gaussian) return [x, y, z];
  return [x / BOHR_TO_ANGSTROM, y / BOHR_TO_ANGSTROM, z / BOHR_TO_ANGSTROM];
}

export function openStructureTable(
  mol: MoleculeData,
  onSaved: (path: string) => void,
): void {
  const gaussian =
    mol.chem?.format === "fchk" ||
    mol.chem?.format === "log" ||
    mol.chem?.format === "gjf" ||
    mol.chem?.format === "com";

  const overlay = document.createElement("div");
  overlay.className = "modal-overlay";
  const box = document.createElement("div");
  box.className = "modal-box modal-wide";

  const h = document.createElement("h3");
  h.textContent = `Structure — ${mol.name} (${mol.atoms.length} atoms, Å display)`;

  const table = document.createElement("table");
  table.className = "struct-table";
  table.innerHTML =
    "<thead><tr><th>#</th><th>Symbol</th><th>X</th><th>Y</th><th>Z</th></tr></thead>";
  const tbody = document.createElement("tbody");

  const rows: HTMLInputElement[][] = [];
  mol.atoms.forEach((a, i) => {
    const [x, y, z] = displayCoord(a, gaussian);
    const tr = document.createElement("tr");
    tr.innerHTML = `<td>${i + 1}</td>`;
    const sym = document.createElement("input");
    sym.value = a.symbol;
    sym.size = 3;
    const ix = document.createElement("input");
    ix.type = "number";
    ix.step = "0.0001";
    ix.value = x.toFixed(4);
    const iy = document.createElement("input");
    iy.type = "number";
    iy.step = "0.0001";
    iy.value = y.toFixed(4);
    const iz = document.createElement("input");
    iz.type = "number";
    iz.step = "0.0001";
    iz.value = z.toFixed(4);
    [sym, ix, iy, iz].forEach((inp) => {
      const td = document.createElement("td");
      td.appendChild(inp);
      tr.appendChild(td);
    });
    tbody.appendChild(tr);
    rows.push([sym, ix, iy, iz]);
  });
  table.appendChild(tbody);

  const row = document.createElement("div");
  row.className = "modal-actions";
  const cancel = document.createElement("button");
  cancel.className = "field-mode-btn";
  cancel.textContent = "Cancel";
  const save = document.createElement("button");
  save.className = "field-mode-btn";
  save.textContent = "Save XYZ";
  cancel.addEventListener("click", () => overlay.remove());
  save.addEventListener("click", () => {
    void (async () => {
      const atoms = rows.map(([sym, ix, iy, iz]) => {
        const [x, y, z] = toInternalCoord(
          Number(ix.value),
          Number(iy.value),
          Number(iz.value),
          gaussian,
        );
        return { symbol: sym.value.trim(), x, y, z };
      });
      const out = await invoke<string>("save_molecule_xyz", {
        path: mol.path,
        title: mol.name,
        atoms,
      });
      overlay.remove();
      onSaved(out);
    })();
  });

  row.append(cancel, save);
  box.append(h, table, row);
  overlay.appendChild(box);
  overlay.addEventListener("click", (e) => {
    if (e.target === overlay) overlay.remove();
  });
  document.body.appendChild(overlay);
}
