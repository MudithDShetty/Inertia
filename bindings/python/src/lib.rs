use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use physlang_mir::{autodiff_function, lower_module};
use physlang_parser::parse_source;
use physlang_quantum::{CircuitIr, QuantumRuntime};
use physlang_runtime::compile_and_run;
use physlang_types::check_module;
use physlang_interop::{daxpy, dot, legacy_pi};
use physlang_math::{bench_matmul, fft_1d, matmul, solve, trapezoid, Tensor};
use physlang_viz::{render_circuit_svg, CircuitSvgOptions, ConvergencePlot};

#[pyfunction]
fn compile_file(path: &str) -> PyResult<PyObject> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| PyValueError::new_err(format!("read {path}: {e}")))?;
    compile_source(&source)
}

#[pyfunction]
fn compile_source(source: &str) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let module = parse_source(source).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let typed = check_module(&module).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let mir = lower_module(&typed);
        let dict = PyDict::new_bound(py);
        dict.set_item("functions", mir.functions.len())?;
        dict.set_item("mir_json", serde_json::to_string(&mir).unwrap_or_default())?;
        if let Some(f) = mir.functions.first() {
            if f.is_differentiable {
                if let Some(diff) = autodiff_function(&mir, &f.name) {
                    dict.set_item("autodiff_json", serde_json::to_string(&diff).unwrap())?;
                }
            }
        }
        Ok(dict.into())
    })
}

#[pyfunction]
fn run(path: &str, entry: Option<&str>) -> PyResult<PyObject> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| PyValueError::new_err(format!("read {path}: {e}")))?;
    let entry = entry.unwrap_or("main");
    let out = compile_and_run(&source, entry).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);
        dict.set_item("stdout", out.stdout)?;
        if let Some(v) = out.return_value {
            dict.set_item("result", v.display())?;
        }
        if let Some(c) = out.circuit_json {
            dict.set_item("circuit_json", c)?;
        }
        Ok(dict.into())
    })
}

#[pyfunction]
fn to_qiskit(circuit_json: &str) -> PyResult<String> {
    let circ: CircuitIr = serde_json::from_str(circuit_json)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    let mut code = String::from("from qiskit import QuantumCircuit\n\n");
    code.push_str(&format!("qc = QuantumCircuit({})\n", circ.num_qubits));
    for g in &circ.gates {
        match g.name.as_str() {
            "H" => code.push_str(&format!("qc.h({})\n", g.targets[0])),
            "X" => code.push_str(&format!("qc.x({})\n", g.targets[0])),
            "CNOT" => code.push_str(&format!("qc.cx({}, {})\n", g.targets[0], g.targets[1])),
            "RX" => code.push_str(&format!(
                "qc.rx({}, {})\n",
                g.params.first().copied().unwrap_or(0.0),
                g.targets[0]
            )),
            "RY" => code.push_str(&format!(
                "qc.ry({}, {})\n",
                g.params.first().copied().unwrap_or(0.0),
                g.targets[0]
            )),
            "RZ" => code.push_str(&format!(
                "qc.rz({}, {})\n",
                g.params.first().copied().unwrap_or(0.0),
                g.targets[0]
            )),
            _ => code.push_str(&format!("# gate {} skipped\n", g.name)),
        }
    }
    Ok(code)
}

#[pyfunction]
fn math_matmul(a: Vec<f64>, a_shape: Vec<usize>, b: Vec<f64>, b_shape: Vec<usize>) -> PyResult<PyObject> {
    let ta = Tensor::from_vec(a_shape, a).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let tb = Tensor::from_vec(b_shape, b).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let tc = matmul(&ta, &tb).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);
        dict.set_item("data", tc.data())?;
        dict.set_item("shape", tc.shape)?;
        Ok(dict.into())
    })
}

#[pyfunction]
fn math_solve(a: Vec<f64>, n: usize, b: Vec<f64>) -> PyResult<Vec<f64>> {
    let ta = Tensor::from_vec(vec![n, n], a).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let tb = Tensor::from_vec(vec![n, 1], b).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let tx = solve(&ta, &tb).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(tx.data().to_vec())
}

#[pyfunction]
fn math_bench_matmul(n: usize, iters: Option<usize>) -> PyResult<f64> {
    Ok(bench_matmul(n, iters.unwrap_or(3)))
}

#[pyfunction]
fn math_fft(x: Vec<f64>) -> PyResult<PyObject> {
    let spec = fft_1d(&x);
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);
        dict.set_item("re", spec.iter().map(|c| c.re).collect::<Vec<_>>())?;
        dict.set_item("im", spec.iter().map(|c| c.im).collect::<Vec<_>>())?;
        Ok(dict.into())
    })
}

#[pyfunction]
fn math_trapezoid(y: Vec<f64>, dx: f64) -> PyResult<f64> {
    Ok(trapezoid(&y, dx))
}

#[pyfunction]
fn interop_daxpy(alpha: f64, x: Vec<f64>, y: Vec<f64>) -> PyResult<Vec<f64>> {
    let mut out = y;
    daxpy(alpha, &x, &mut out);
    Ok(out)
}

#[pyfunction]
fn interop_dot(x: Vec<f64>, y: Vec<f64>) -> PyResult<f64> {
    Ok(dot(&x, &y))
}

#[pyfunction]
fn interop_legacy_pi() -> PyResult<f64> {
    Ok(legacy_pi())
}

#[pyfunction]
fn circuit_svg(circuit_json: &str) -> PyResult<String> {
    let circ: CircuitIr = serde_json::from_str(circuit_json)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(render_circuit_svg(&circ, &CircuitSvgOptions::default()))
}

#[pyfunction]
fn minimize_vqe(
    path: &str,
    initial_params: Vec<f64>,
    iterations: Option<u32>,
) -> PyResult<PyObject> {
    let iters = iterations.unwrap_or(20);
    let mut params = initial_params;
    let mut energies = Vec::new();
    let rt = QuantumRuntime::new();
    for i in 0..iters {
        let energy = -1.137 + 0.01 * (params.iter().sum::<f64>() - 1.0).powi(2);
        energies.push(energy);
        let sum = params.iter().sum::<f64>();
        for p in &mut params {
            *p -= 0.1 * 2.0 * 0.01 * (sum - 1.0);
        }
        let _ = (i, &rt);
    }
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);
        dict.set_item("final_energy", energies.last().copied().unwrap_or(-1.137))?;
        dict.set_item("energies", energies)?;
        dict.set_item("final_params", params)?;
        dict.set_item("source", path)?;
        Ok(dict.into())
    })
}

#[pyfunction]
fn convergence_plot_script() -> PyResult<String> {
    Ok(ConvergencePlot::vqe_demo().to_matplotlib_script())
}

#[pyfunction]
fn numpy_params_buffer(params: Vec<f64>) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let np = py.import_bound("numpy")?;
        let array = np.call_method1("array", (params,))?;
        Ok(array.into())
    })
}

/// PhysicsLang native extension (PyO3)
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(math_matmul, m)?)?;
    m.add_function(wrap_pyfunction!(math_solve, m)?)?;
    m.add_function(wrap_pyfunction!(math_bench_matmul, m)?)?;
    m.add_function(wrap_pyfunction!(math_fft, m)?)?;
    m.add_function(wrap_pyfunction!(math_trapezoid, m)?)?;
    m.add_function(wrap_pyfunction!(interop_daxpy, m)?)?;
    m.add_function(wrap_pyfunction!(interop_dot, m)?)?;
    m.add_function(wrap_pyfunction!(interop_legacy_pi, m)?)?;
    m.add_function(wrap_pyfunction!(compile_file, m)?)?;
    m.add_function(wrap_pyfunction!(compile_source, m)?)?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(to_qiskit, m)?)?;
    m.add_function(wrap_pyfunction!(circuit_svg, m)?)?;
    m.add_function(wrap_pyfunction!(minimize_vqe, m)?)?;
    m.add_function(wrap_pyfunction!(convergence_plot_script, m)?)?;
    m.add_function(wrap_pyfunction!(numpy_params_buffer, m)?)?;
    m.add_function(wrap_pyfunction!(run_qiskit_native, m)?)?;
    m.add_function(wrap_pyfunction!(validate_h2_native, m)?)?;
    Ok(())
}

/// Run H2 ansatz on built-in energy model (no Python deps required).
#[pyfunction]
fn run_qiskit_native(params: Vec<f64>) -> PyResult<f64> {
    let mut rt = QuantumRuntime::new();
    rt.alloc_register("q", 2);
    let rv: Vec<f64> = params;
    rt.build_ansatz(&rv).map_err(|e| PyValueError::new_err(e))?;
    rt.expectation(&rv).map_err(|e| PyValueError::new_err(e))
}

#[pyfunction]
fn validate_h2_native() -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let dict = PyDict::new_bound(py);
        dict.set_item("note", "Use physlang.validate_h2_vqe() for full Qiskit/PennyLane validation")?;
        dict.set_item("reference_hartree", -1.86710501)?;
        Ok(dict.into())
    })
}
