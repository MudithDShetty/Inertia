use physlang_quantum::{CircuitIr, GateIr};

#[derive(Debug, Clone)]
pub struct CircuitSvgOptions {
    pub width: u32,
    pub qubit_spacing: u32,
    pub gate_width: u32,
}

impl Default for CircuitSvgOptions {
    fn default() -> Self {
        Self {
            width: 800,
            qubit_spacing: 40,
            gate_width: 48,
        }
    }
}

pub fn render_circuit_svg(circuit: &CircuitIr, opts: &CircuitSvgOptions) -> String {
    let n = circuit.num_qubits.max(1);
    let height = n * opts.qubit_spacing + 40;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">"#,
        w = opts.width,
        h = height
    );
    svg.push_str(r#"<style>.wire{stroke:#444;stroke-width:2}.gate{fill:#6c5ce7;stroke:#4834d4;rx:4}.label{fill:#fff;font:12px sans-serif;text-anchor:middle}.qlabel{fill:#666;font:11px sans-serif}</style>"#);

    for q in 0..n {
        let y = 30 + q * opts.qubit_spacing;
        svg.push_str(&format!(
            r#"<line class="wire" x1="40" y1="{y}" x2="{x2}" y2="{y}"/>"#,
            y = y,
            x2 = opts.width - 20
        ));
        svg.push_str(&format!(
            r#"<text class="qlabel" x="8" y="{}">q{}</text>"#,
            y + 4,
            q
        ));
    }

    let mut x = 60;
    for gate in &circuit.gates {
        svg.push_str(&render_gate(gate, x, &opts));
        x += opts.gate_width + 12;
    }

    svg.push_str("</svg>");
    svg
}

fn render_gate(gate: &GateIr, x: u32, opts: &CircuitSvgOptions) -> String {
    match gate.name.as_str() {
        "CNOT" if gate.targets.len() >= 2 => {
            let c = gate.targets[0] as u32;
            let t = gate.targets[1] as u32;
            let yc = 30 + c * opts.qubit_spacing;
            let yt = 30 + t * opts.qubit_spacing;
            format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"#444\"/>\
                 <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"2\"/>\
                 <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"2\"/>\
                 <circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"none\" stroke=\"#444\" stroke-width=\"2\"/>",
                x + 20, yc, x + 20, yc, x + 20, yt, x + 20, yt, x + 20, yt, x + 20, yt
            )
        }
        _ => {
            let q = gate.targets.first().copied().unwrap_or(0);
            let y = 30 + q * opts.qubit_spacing;
            let label = if gate.name.starts_with('R') && !gate.params.is_empty() {
                format!("{}({:.2})", gate.name, gate.params[0])
            } else {
                gate.name.clone()
            };
            format!(
                r#"<rect class="gate" x="{x}" y="{y}" width="{w}" height="24"/><text class="label" x="{tx}" y="{ty}">{label}</text>"#,
                x = x,
                y = y - 12,
                w = opts.gate_width,
                tx = x + opts.gate_width / 2,
                ty = y + 4,
                label = label
            )
        }
    }
}

pub fn render_circuit_json_to_svg(json: &str) -> Result<String, String> {
    let circuit: CircuitIr = serde_json::from_str(json).map_err(|e| e.to_string())?;
    Ok(render_circuit_svg(&circuit, &CircuitSvgOptions::default()))
}
