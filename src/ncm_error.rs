use pyo3::PyErr;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::PyResult;

/// 把 `ncm_api_rs::NcmError` 转成 Python 异常（保留原始错误文案）。
pub fn to_py_err(err: ncm_api_rs::NcmError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// 把 Rust 侧的 `Result` 平移到 Python 侧的 `PyResult`。
pub fn into_py_result<T>(result: ncm_api_rs::error::Result<T>) -> PyResult<T> {
    result.map_err(to_py_err)
}
