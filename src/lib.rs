//! ncm-api-py —— `ncm-api-rs` 的 PyO3 绑定
//!
//! 目前只翻译两个接口，且都是原样转发：
//! - `ApiClient::search`       —— 歌曲搜索
//! - `ApiClient::song_detail`  —— 通过 id 获取歌曲信息
//!
//! 不增删任何请求参数，也不改动词段名、状态码与错误语义。

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyList};

/// 把 `serde_json::Value` 原样映射成 Python 对象：
/// null → None，bool → bool，number → int/float，string → str，
/// array → list，object → dict。
///
/// 只做类型映射，不改动任何字段名或值。
fn json_to_py(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    use serde_json::Value;

    Ok(match value {
        Value::Null => py.None(),
        Value::Bool(b) => PyBool::new(py, *b).to_owned().unbind().into_any(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py)?.unbind().into_any()
            } else if let Some(u) = n.as_u64() {
                u.into_pyobject(py)?.unbind().into_any()
            } else {
                n.as_f64()
                    .unwrap_or(f64::NAN)
                    .into_pyobject(py)?
                    .unbind()
                    .into_any()
            }
        }
        Value::String(s) => s.into_pyobject(py)?.unbind().into_any(),
        Value::Array(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(json_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (key, item) in map {
                dict.set_item(key, json_to_py(py, item)?)?;
            }
            dict.into_any().unbind()
        }
    })
}

/// 把 `ncm_api_rs::NcmError` 转成 Python 异常（保留原始错误文案）。
fn to_py_err(err: ncm_api_rs::NcmError) -> PyErr {
    PyRuntimeError::new_err(err.to_string())
}

/// 把 Rust 侧的 `Result` 平移到 Python 侧的 `PyResult`。
fn into_py_result<T>(result: ncm_api_rs::error::Result<T>) -> PyResult<T> {
    result.map_err(to_py_err)
}

/// 对应 `ncm_api_rs::ApiResponse`。
#[pyclass(module = "ncm_api_py", skip_from_py_object)]
pub struct ApiResponse {
    /// 对应 `ApiResponse::status`
    #[pyo3(get)]
    pub status: i64,
    /// 对应 `ApiResponse::body`
    #[pyo3(get)]
    pub body: Py<PyAny>,
    /// 对应 `ApiResponse::cookie`
    #[pyo3(get)]
    pub cookie: Vec<String>,
}

impl ApiResponse {
    fn from_rs(py: Python<'_>, resp: ncm_api_rs::ApiResponse) -> PyResult<Self> {
        Ok(Self {
            status: resp.status,
            body: json_to_py(py, &resp.body)?,
            cookie: resp.cookie,
        })
    }
}

/// 对应 `ncm_api_rs::ApiClient`。
#[pyclass(module = "ncm_api_py")]
pub struct Client {
    inner: ncm_api_rs::ApiClient,
}

#[pymethods]
impl Client {
    /// 创建一个新的 API 客户端，对应 `ApiClient::new`。
    #[new]
    #[pyo3(signature = (cookie=None))]
    fn new(cookie: Option<String>) -> Self {
        Self {
            inner: ncm_api_rs::create_client(cookie),
        }
    }

    /// 搜索
    ///
    /// 对应 `ApiClient::search`（即 `/search`）。
    ///
    /// :param keywords: 搜索关键字，对应 `Query` 的 `keywords`
    /// :param type: 搜索类型，对应 `Query` 的 `type`，默认 `1`（单曲）
    /// :param limit: 返回数量，对应 `Query` 的 `limit`，默认 `30`
    /// :param offset: 偏移量，对应 `Query` 的 `offset`，默认 `0`
    #[pyo3(signature = (keywords, r#type=None, limit=None, offset=None))]
    fn search<'py>(
        &self,
        py: Python<'py>,
        keywords: String,
        r#type: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let mut query = ncm_api_rs::Query::new().param("keywords", &keywords);
            if let Some(search_type) = r#type {
                query = query.param("type", &search_type.to_string());
            }
            if let Some(limit) = limit {
                query = query.param("limit", &limit.to_string());
            }
            if let Some(offset) = offset {
                query = query.param("offset", &offset.to_string());
            }

            let resp = into_py_result(inner.search(&query).await)?;
            Python::attach(|py| ApiResponse::from_rs(py, resp))
        })
    }

    /// 歌曲详情
    ///
    /// 对应 `ApiClient::song_detail`（即 `/song/detail`）。
    ///
    /// :param ids: 歌曲 id，支持 `186016` / `"186016"` / `[186016, 186017]` / `"186016,186017"`
    #[pyo3(signature = (ids))]
    fn song_detail<'py>(
        &self,
        py: Python<'py>,
        ids: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let ids = collect_ids(ids)?;
        let inner = self.inner.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let query = ncm_api_rs::Query::new().param("ids", &ids);
            let resp = into_py_result(inner.song_detail(&query).await)?;
            Python::attach(|py| ApiResponse::from_rs(py, resp))
        })
    }
}

/// 把 Python 传入的 id 参数拍平成 `Query` 需要的字符串。
///
/// 接受 `int` / `str` / 任意可迭代对象（`list`、`tuple`、生成器等）。
/// 切分、去空白、丢掉空串都交给 `ncm-api-rs` 的 `song_detail` 处理，
/// 这里只负责把 Python 对象拍平成逗号分隔的字符串。
fn collect_ids(ids: &Bound<'_, PyAny>) -> PyResult<String> {
    if ids.is_none() {
        return Ok(String::new());
    }
    // 先于 str 判断：Python 的 bool 是 int 的子类，这里按原样透传给 Rust 侧。
    if let Ok(single) = ids.extract::<i64>() {
        return Ok(single.to_string());
    }
    if let Ok(single) = ids.extract::<String>() {
        return Ok(single);
    }
    // dict 也可迭代（迭代得到 key），但传进来几乎一定是笔误，直接拒绝。
    if ids.cast::<PyDict>().is_ok() {
        return Err(pyo3::exceptions::PyTypeError::new_err(
            "ids 必须是 int、str 或可迭代对象，而不是 dict",
        ));
    }

    let iter = match ids.try_iter() {
        Ok(iter) => iter,
        Err(_) => {
            return Err(pyo3::exceptions::PyTypeError::new_err(format!(
                "ids 必须是 int、str 或可迭代对象，而不是 {}",
                ids.get_type().name()?
            )));
        }
    };

    let mut parts = Vec::new();
    for item in iter {
        let item = item?;
        if let Ok(id) = item.extract::<i64>() {
            parts.push(id.to_string());
        } else {
            parts.push(item.str()?.to_string());
        }
    }
    Ok(parts.join(","))
}

/// 创建一个新的 API 客户端，对应 `ncm_api_rs::create_client`。
#[pyfunction]
#[pyo3(signature = (cookie=None))]
fn create_client(cookie: Option<String>) -> Client {
    Client::new(cookie)
}

/// A Python module implemented in Rust.
#[pymodule]
fn ncm_api_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ApiResponse>()?;
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(create_client, m)?)?;
    Ok(())
}
