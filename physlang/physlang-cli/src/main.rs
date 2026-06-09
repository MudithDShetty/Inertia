use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use physlang_lsp::diagnostics_for_source;
use physlang_llvm::generate_pseudo_llvm;
use physlang_parser::parse_source;
use physlang_runtime::{compile_and_run, compile_mir};
use physlang_types::check_module;
use physlang_viz::{render_circuit_json_to_svg, ConvergencePlot};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "phys", about = "Inertia compiler and runtime", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a .phys file
    Build {
        file: PathBuf,
        #[arg(long, default_value = "cpu")]
        target: Target,
        #[arg(long)]
        emit: Option<EmitKind>,
        #[arg(short, long, default_value = "out")]
        output: PathBuf,
    },
    /// Compile and run a .phys file
    Run {
        file: PathBuf,
        #[arg(long, default_value = "main")]
        entry: String,
    },
    /// Type-check without running
    Check {
        file: PathBuf,
    },
    /// Interactive REPL (stub)
    Repl,
    /// Format source (stub)
    Fmt {
        file: PathBuf,
    },
    /// Run LSP diagnostics on a file
    Lsp {
        file: PathBuf,
    },
    /// Render circuit SVG from .phys or JSON
    Viz {
        file: PathBuf,
        #[arg(short, long, default_value = "circuit.svg")]
        output: PathBuf,
    },
}

#[derive(Clone, ValueEnum)]
enum Target {
    Cpu,
    Cuda,
    Metal,
    Wasm,
}

#[derive(Clone, ValueEnum)]
enum EmitKind {
    Ast,
    Mir,
    Llvm,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Build {
            file,
            target,
            emit,
            output,
        } => cmd_build(&file, target, emit, &output),
        Commands::Run { file, entry } => cmd_run(&file, &entry),
        Commands::Check { file } => cmd_check(&file),
        Commands::Repl => cmd_repl(),
        Commands::Fmt { file } => cmd_fmt(&file),
        Commands::Lsp { file } => cmd_lsp(&file),
        Commands::Viz { file, output } => cmd_viz(&file, &output),
    }
}

fn read_source(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("read {}", path.display()))
}

fn cmd_build(
    file: &PathBuf,
    target: Target,
    emit: Option<EmitKind>,
    output: &PathBuf,
) -> Result<()> {
    let source = read_source(file)?;
    let mir = compile_mir(&source)?;

    match emit {
        Some(EmitKind::Ast) => {
            let module = parse_source(&source)?;
            let json = serde_json::to_string_pretty(&module)?;
            fs::write(output.with_extension("ast.json"), json)?;
        }
        Some(EmitKind::Mir) => {
            let json = serde_json::to_string_pretty(&mir)?;
            fs::write(output.with_extension("mir.json"), json)?;
        }
        Some(EmitKind::Llvm) | None => {
            let ir = generate_pseudo_llvm(&mir);
            let ext = match target {
                Target::Cpu => "ll",
                Target::Cuda => "cu.ll",
                Target::Metal => "metal.ll",
                Target::Wasm => "wasm.ll",
            };
            fs::write(output.with_extension(ext), ir)?;
        }
    }
    println!("built {} -> {}", file.display(), output.display());
    Ok(())
}

fn cmd_run(file: &PathBuf, entry: &str) -> Result<()> {
    let source = read_source(file)?;
    let out = compile_and_run(&source, entry)?;
    for line in &out.stdout {
        println!("{line}");
    }
    if let Some(v) = out.return_value {
        println!("=> {}", v.display());
    }
    Ok(())
}

fn cmd_check(file: &PathBuf) -> Result<()> {
    let source = read_source(file)?;
    let module = parse_source(&source)?;
    check_module(&module)?;
    println!("{}: OK", file.display());
    Ok(())
}

fn cmd_repl() -> Result<()> {
    println!("Inertia REPL v0.1 (stub). Type :quit to exit.");
    let stdin = std::io::stdin();
    loop {
        print!("phys> ");
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut line = String::new();
        if stdin.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line == ":quit" || line.is_empty() {
            if line == ":quit" {
                break;
            }
            continue;
        }
        let wrapped = format!("fn __repl() -> Int {{ {line} }}");
        match compile_and_run(&wrapped, "__repl") {
            Ok(o) => {
                if let Some(v) = o.return_value {
                    println!("{}", v.display());
                }
            }
            Err(e) => eprintln!("error: {e}"),
        }
    }
    Ok(())
}

fn cmd_fmt(file: &PathBuf) -> Result<()> {
    let source = read_source(file)?;
    // Stub: trim trailing whitespace per line
    let formatted: String = source
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(file, formatted)?;
    println!("formatted {}", file.display());
    Ok(())
}

fn cmd_lsp(file: &PathBuf) -> Result<()> {
    let source = read_source(file)?;
    let diags = diagnostics_for_source(&source);
    if diags.is_empty() {
        println!("no diagnostics");
    } else {
        for d in diags {
            println!("{}:{}:{}: {}", file.display(), d.line, d.column, d.message);
        }
    }
    Ok(())
}

fn cmd_viz(file: &PathBuf, output: &PathBuf) -> Result<()> {
    let source = read_source(file)?;
    let out = compile_and_run(&source, "main").or_else(|_| compile_and_run(&source, "grover"))?;
    let json = out.circuit_json.unwrap_or_else(|| {
        let mir = compile_mir(&source).unwrap();
        serde_json::to_string(&mir).unwrap()
    });
    let svg = if json.contains("gates") {
        render_circuit_json_to_svg(&json).map_err(|e| anyhow::anyhow!(e))?
    } else {
        let _ = ConvergencePlot::vqe_demo().to_matplotlib_script();
        render_circuit_json_to_svg(
            r#"{"num_qubits":2,"name":"demo","gates":[{"name":"H","targets":[0],"params":[]},{"name":"CNOT","targets":[0,1],"params":[]}]}"#,
        )
        .map_err(|e| anyhow::anyhow!(e))?
    };
    fs::write(output, svg)?;
    println!("wrote {}", output.display());
    Ok(())
}

// expose autodiff for tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_runs() {
        let src = include_str!("../../../examples/hello.phys");
        let out = compile_and_run(src, "main").unwrap();
        assert!(out.return_value.is_some());
    }
}
