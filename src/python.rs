#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (source, target = "claude", opt_level = 2))]
fn compile(source: &str, target: &str, opt_level: u8) -> PyResult<String> {
    let target = match target {
        "claude" => crate::codegen::ModelTarget::Claude,
        "gpt" => crate::codegen::ModelTarget::Gpt,
        "mistral" => crate::codegen::ModelTarget::Mistral,
        "llama" => crate::codegen::ModelTarget::Llama,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown target: {other}. Must be one of: claude, gpt, mistral, llama"
            )));
        }
    };

    crate::compile(source, target, opt_level)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[cfg(feature = "python")]
#[pyfunction]
fn check_gptisms(text: &str) -> Vec<(String, String, String)> {
    crate::analysis::gptisms::detect_gptisms(text)
        .into_iter()
        .map(|f| {
            (
                f.found,
                f.suggestion,
                format!("{:?}", f.severity),
            )
        })
        .collect()
}

#[cfg(feature = "python")]
#[pymodule]
fn promptc(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(check_gptisms, m)?)?;
    Ok(())
}
