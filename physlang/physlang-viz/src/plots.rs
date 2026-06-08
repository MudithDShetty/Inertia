use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlotSeries {
    pub label: String,
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvergencePlot {
    pub title: String,
    pub x_label: String,
    pub y_label: String,
    pub series: Vec<PlotSeries>,
}

impl ConvergencePlot {
    pub fn vqe_demo() -> Self {
        let iterations: Vec<f64> = (0..20).map(|i| i as f64).collect();
        let energy: Vec<f64> = iterations
            .iter()
            .map(|i| -1.137 + 0.5 * (-0.3 * i).exp())
            .collect();
        Self {
            title: "VQE Energy Convergence".into(),
            x_label: "Iteration".into(),
            y_label: "Energy (Ha)".into(),
            series: vec![PlotSeries {
                label: "⟨H⟩".into(),
                x: iterations,
                y: energy,
            }],
        }
    }

    pub fn to_matplotlib_script(&self) -> String {
        let xs = format!("{:?}", self.series.first().map(|s| &s.x).unwrap_or(&vec![]));
        let ys = format!("{:?}", self.series.first().map(|s| &s.y).unwrap_or(&vec![]));
        format!(
            r#"
import matplotlib.pyplot as plt
xs = {xs}
ys = {ys}
plt.plot(xs, ys, label="{label}")
plt.xlabel("{x_label}")
plt.ylabel("{y_label}")
plt.title("{title}")
plt.legend()
plt.savefig("convergence.png")
"#,
            label = self.series.first().map(|s| s.label.as_str()).unwrap_or(""),
            x_label = self.x_label,
            y_label = self.y_label,
            title = self.title,
        )
    }
}
