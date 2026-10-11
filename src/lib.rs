//! ncm-api-py —— `ncm-api-rs` 的 PyO3 绑定
//!
//! 目前只翻译三个接口：
//! - `ApiClient::search`       —— 歌曲搜索
//! - `ApiClient::song_detail`  —— 通过 id 获取歌曲信息
//! - `ApiClient::song_url_v1`  —— 通过 id 获取歌曲播放链接
mod ncm_error;

use crate::ncm_error::into_py_result;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use pythonize::pythonize;
use pyo3_async_runtimes::tokio::future_into_py;


#[pyclass(module = "ncm_api_py", skip_from_py_object)]
pub struct ApiResponse {
    #[pyo3(get)]
    pub status: i64,
    #[pyo3(get)]
    pub body: Py<PyDict>,
    #[pyo3(get)]
    pub cookie: Vec<String>,
}

impl ApiResponse {
    fn from_rs(py: Python<'_>, resp: ncm_api_rs::ApiResponse) -> PyResult<Self> {
        let obj = pythonize(py, &resp.body)?;
        let body = obj.cast_into::<PyDict>()?.unbind();
        Ok(Self {
            status: resp.status,
            body,
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
    #[new]
    #[pyo3(signature = (cookie=None))]
    fn new(cookie: Option<String>) -> Self {
        Self {
            inner: ncm_api_rs::create_client(cookie),
        }
    }

    /// 搜索
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
    ) -> PyResult<Bound<'py, PyAny>> { // python类型其实是 Awaitable[ApiResponse]
        let inner = self.inner.clone();
        future_into_py(py, async move {
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
    /// :param ids: 歌曲 id
    #[pyo3(signature = (ids))]
    fn song_detail<'py>(
        &self,
        py: Python<'py>,
        ids: Vec<i64>,
    ) -> PyResult<Bound<'py, PyAny>> { // python类型其实是 Awaitable[ApiResponse]
        let ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let ids = ids.join(",");
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let query = ncm_api_rs::Query::new().param("ids", &ids);
            let resp = into_py_result(inner.song_detail(&query).await)?;
            Python::attach(|py| ApiResponse::from_rs(py, resp))
        })
    }

    /// 歌曲播放链接
    ///
    /// :param ids: 歌曲 id
    /// :param level: 音质，对应 `Query` 的 `level`，默认 `standard`
    ///
    /// 链接在 `body["data"][i]["url"]`；无版权或需付费时该字段为 `None`，
    /// 但状态码仍是 200，调用方需自行判空。
    #[pyo3(signature = (ids, level=None))]
    fn song_url_v1<'py>(
        &self,
        py: Python<'py>,
        ids: Vec<i64>,
        level: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> { // python类型其实是 Awaitable[ApiResponse]
        let ids = ids.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        let ids = ids.join(",");
        let inner = self.inner.clone();
        future_into_py(py, async move {
            let mut query = ncm_api_rs::Query::new().param("id", &ids);
            if let Some(level) = level {
                query = query.param("level", &level);
            }
            let resp = into_py_result(inner.song_url_v1(&query).await)?;
            Python::attach(|py| ApiResponse::from_rs(py, resp))
        })
    }
}

/// 创建一个新的 API 客户端
#[pyfunction]
#[pyo3(signature = (cookie=None))]
fn create_client(cookie: Option<String>) -> Client {
    Client::new(cookie)
}

/// 将类和函数注册到 Python 模块中。
#[pymodule]
fn ncm_api_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ApiResponse>()?;
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(create_client, m)?)?;
    Ok(())
}
