use pyo3::PyErr;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::PyResult;

/// 把 `ncm_api_rs::NcmError` 转成 Python 异常（保留原始错误文案）。
fn to_py_err(err: ncm_api_rs::NcmError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// 把 Rust 侧的 `Result` 平移到 Python 侧的 `PyResult`。
pub(crate) fn into_py_result<T>(result: ncm_api_rs::error::Result<T>) -> PyResult<T> {
    result.map_err(to_py_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_upstream_error_text() {
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            let error = ncm_api_rs::NcmError::AuthRequired("login required".to_owned());
            let expected = error.to_string();
            let error = into_py_result::<()>(Err(error)).unwrap_err();
            assert!(error.is_instance_of::<PyRuntimeError>(py));
            assert_eq!(error.value(py).to_string(), expected);
            assert_eq!(into_py_result::<i64>(Ok(200)).unwrap(), 200);
        });
    }
}
