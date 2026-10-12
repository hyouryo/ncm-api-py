//! Async Python bindings for the three supported ncm-api-rs endpoints.
mod ncm_error;

use std::fmt::Write;
use std::future::Future;

use ncm_api_rs::{ApiClient, Query};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyDict, PyInt, PyList, PyType};
use pyo3_async_runtimes::tokio::future_into_py;
use pythonize::pythonize;

use crate::ncm_error::into_py_result;

const SEARCH_TYPES: &[i64] = &[1, 10, 100, 1000, 1002, 1004, 1006, 1009, 1014, 1018, 2000];
const QUALITY_LEVELS: &[&str] = &[
    "standard", "higher", "exhigh", "lossless", "hires", "jyeffect", "sky", "dolby", "jymaster",
];

/// An integer argument without Python's implicit bool/__index__ conversions.
#[derive(Clone, Copy)]
struct StrictInt(i64);

impl<'a, 'py> FromPyObject<'a, 'py> for StrictInt {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> PyResult<Self> {
        if obj.is_instance_of::<PyBool>() || !obj.is_instance_of::<PyInt>() {
            return Err(PyTypeError::new_err(
                "expected an int, not bool or another type",
            ));
        }
        obj.extract::<i64>().map(Self)
    }
}

fn search_query(
    keywords: &str,
    search_type: Option<StrictInt>,
    limit: Option<StrictInt>,
    offset: Option<StrictInt>,
) -> PyResult<Query> {
    let search_type = search_type.map_or(1, |value| value.0);
    let limit = limit.map_or(30, |value| value.0);
    let offset = offset.map_or(0, |value| value.0);
    if !SEARCH_TYPES.contains(&search_type) {
        return Err(PyValueError::new_err("unsupported search type"));
    }
    if limit <= 0 {
        return Err(PyValueError::new_err("limit must be greater than zero"));
    }
    if offset < 0 {
        return Err(PyValueError::new_err("offset must be non-negative"));
    }
    Ok(Query::new()
        .param("keywords", keywords)
        .param("type", &search_type.to_string())
        .param("limit", &limit.to_string())
        .param("offset", &offset.to_string()))
}

fn song_ids(ids: &Bound<'_, PyList>) -> PyResult<String> {
    if ids.is_empty() {
        return Err(PyValueError::new_err(
            "ids must be a non-empty list of positive integers",
        ));
    }
    let mut result = String::new();
    for (index, item) in ids.iter().enumerate() {
        let id = item.extract::<StrictInt>()?.0;
        if id <= 0 {
            return Err(PyValueError::new_err("song IDs must be greater than zero"));
        }
        if index != 0 {
            result.push(',');
        }
        write!(&mut result, "{id}").expect("writing to a String cannot fail");
    }
    Ok(result)
}

fn quality_level(level: Option<&str>) -> PyResult<&str> {
    let level = level.unwrap_or("standard");
    if !QUALITY_LEVELS.contains(&level) {
        return Err(PyValueError::new_err("unsupported quality level"));
    }
    Ok(level)
}

#[pyclass(module = "ncm_api_py", skip_from_py_object)]
pub struct ApiResponse {
    #[pyo3(get)]
    status: i64,
    #[pyo3(get)]
    body: Py<PyDict>,
    #[pyo3(get)]
    cookie: Vec<String>,
}

impl ApiResponse {
    fn from_rs(py: Python<'_>, response: ncm_api_rs::ApiResponse) -> PyResult<Self> {
        let body = pythonize(py, &response.body)?
            .cast_into::<PyDict>()?
            .unbind();
        Ok(Self {
            status: response.status,
            body,
            cookie: response.cookie,
        })
    }
}

#[pymethods]
impl ApiResponse {
    // Generic parameters describe the JSON body in the stub, not at runtime.
    #[classmethod]
    fn __class_getitem__(cls: &Bound<'_, PyType>, _item: &Bound<'_, PyAny>) -> Py<PyType> {
        cls.clone().unbind()
    }
}

fn response_future<'py, F>(py: Python<'py>, future: F) -> PyResult<Bound<'py, PyAny>>
where
    F: Future<Output = ncm_api_rs::error::Result<ncm_api_rs::ApiResponse>> + Send + 'static,
{
    future_into_py(py, async move {
        let response = into_py_result(future.await)?;
        Python::attach(|py| ApiResponse::from_rs(py, response))
    })
}

#[pyclass(module = "ncm_api_py")]
pub struct Client {
    inner: ApiClient,
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

    /// Search. Invalid arguments fail before a network request is scheduled.
    #[pyo3(signature = (keywords, r#type=None, limit=None, offset=None))]
    fn search<'py>(
        &self,
        py: Python<'py>,
        keywords: String,
        r#type: Option<StrictInt>,
        limit: Option<StrictInt>,
        offset: Option<StrictInt>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = search_query(&keywords, r#type, limit, offset)?;
        let inner = self.inner.clone();
        response_future(py, async move { inner.search(&query).await })
    }

    /// Fetch details for a non-empty list of positive integer song IDs.
    #[pyo3(signature = (ids))]
    fn song_detail<'py>(
        &self,
        py: Python<'py>,
        ids: &Bound<'_, PyList>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = Query::new().param("ids", &song_ids(ids)?);
        let inner = self.inner.clone();
        response_future(py, async move { inner.song_detail(&query).await })
    }

    /// Fetch playback URLs. A successful response can still contain a null URL.
    #[pyo3(signature = (ids, level=None))]
    fn song_url_v1<'py>(
        &self,
        py: Python<'py>,
        ids: &Bound<'_, PyList>,
        level: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = Query::new()
            .param("id", &song_ids(ids)?)
            .param("level", quality_level(level.as_deref())?);
        let inner = self.inner.clone();
        response_future(py, async move { inner.song_url_v1(&query).await })
    }
}

#[pyfunction]
#[pyo3(signature = (cookie=None))]
fn create_client(cookie: Option<String>) -> Client {
    Client::new(cookie)
}

#[pymodule]
fn ncm_api_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ApiResponse>()?;
    m.add_class::<Client>()?;
    m.add_function(wrap_pyfunction!(create_client, m)?)?;
    m.add("__all__", ["ApiResponse", "Client", "create_client"])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_defaults_and_voice_type() {
        let query = search_query("music", None, None, None).unwrap();
        assert_eq!(query.get("keywords"), Some("music"));
        assert_eq!(query.get("type"), Some("1"));
        assert_eq!(query.get("limit"), Some("30"));
        assert_eq!(query.get("offset"), Some("0"));
        let query = search_query(
            "voice",
            Some(StrictInt(2000)),
            Some(StrictInt(5)),
            Some(StrictInt(2)),
        )
        .unwrap();
        assert_eq!(query.get("type"), Some("2000"));
        assert_eq!(query.get("limit"), Some("5"));
        assert_eq!(query.get("offset"), Some("2"));
    }

    #[test]
    fn quality_defaults_and_supported_levels() {
        assert_eq!(quality_level(None).unwrap(), "standard");
        for &level in QUALITY_LEVELS {
            assert_eq!(quality_level(Some(level)).unwrap(), level);
        }
    }

    #[test]
    fn serializes_ids_without_losing_order_or_precision() {
        Python::initialize();
        Python::attach(|py| {
            let ids = PyList::new(py, [186016_i64, i64::MAX, 186016]).unwrap();
            assert_eq!(song_ids(&ids).unwrap(), "186016,9223372036854775807,186016");
        });
    }

    #[test]
    fn converts_json_body_and_nullable_url() {
        Python::initialize();
        Python::attach(|py| {
            let response = ncm_api_rs::ApiResponse {
                status: 200,
                body: serde_json::json!({"code": 200, "data": [{"id": 186016, "url": null}]}),
                cookie: vec!["session=example".to_owned()],
            };
            let response = ApiResponse::from_rs(py, response).unwrap();
            assert_eq!(response.status, 200);
            assert_eq!(response.cookie, ["session=example"]);
            let data = response.body.bind(py).get_item("data").unwrap().unwrap();
            assert!(data.get_item(0).unwrap().get_item("url").unwrap().is_none());
        });
    }

    #[test]
    fn rejects_non_object_response_body() {
        Python::initialize();
        Python::attach(|py| {
            let response = ncm_api_rs::ApiResponse {
                status: 200,
                body: serde_json::json!([]),
                cookie: vec![],
            };
            let error = ApiResponse::from_rs(py, response).err().unwrap();
            assert!(error.is_instance_of::<PyTypeError>(py));
        });
    }
}
