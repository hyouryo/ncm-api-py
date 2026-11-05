//! Generate Python stubs from the Rust API annotations.
//! Run with `cargo run --locked --bin stub_gen`.
use std::fs;
use std::path::Path;

use pyo3_stub_gen::Result;
use pyo3_stub_gen::generate::{ClassDef, Parameters};

fn main() -> Result<()> {
    let mut stub = ncm_api_py::stub_info()?;
    // pyo3-stub-gen 0.23.2 preserves Rust raw identifiers in parameter names.
    // Normalize metadata, not generated Python text or docstrings.
    for module in stub.modules.values_mut() {
        for class in module.class.values_mut() {
            normalize_class(class);
        }
        for functions in module.function.values_mut() {
            for function in functions {
                normalize_parameters(&mut function.parameters);
            }
        }
    }
    stub.generate()?;

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ncm_api_py.pyi");
    let text = fs::read_to_string(&path)?;
    fs::write(&path, format!("{}\n", text.trim_end()))?;
    Ok(())
}

fn normalize_class(class: &mut ClassDef) {
    for methods in class.methods.values_mut() {
        for method in methods {
            normalize_parameters(&mut method.parameters);
        }
    }
    for nested in &mut class.classes {
        normalize_class(nested);
    }
}

fn normalize_parameters(parameters: &mut Parameters) {
    for parameter in parameters
        .positional_only
        .iter_mut()
        .chain(&mut parameters.positional_or_keyword)
        .chain(&mut parameters.varargs)
        .chain(&mut parameters.keyword_only)
        .chain(&mut parameters.varkw)
    {
        if let Some(name) = parameter.name.strip_prefix("r#") {
            parameter.name = name;
        }
    }
}
