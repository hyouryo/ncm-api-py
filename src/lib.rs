//! Thin async Python bindings for the three supported ncm-api-rs endpoints.
mod ncm_error;

use std::future::Future;

use ncm_api_rs::{ApiClient, Query};
use ncm_error::into_py_result;
use pyo3::prelude::*;
use pyo3::types::PyAny;
use pyo3_async_runtimes::tokio::future_into_py;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};
use pythonize::pythonize;

fn search_query(
    keywords: &str,
    search_type: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Query {
    let mut query = Query::new().param("keywords", keywords);
    for (name, value) in [("type", search_type), ("limit", limit), ("offset", offset)] {
        if let Some(value) = value {
            query = query.param(name, &value.to_string());
        }
    }
    query
}

fn song_ids(ids: Vec<i64>) -> String {
    ids.into_iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[gen_stub_pyclass]
#[pyclass(module = "ncm_api_py", skip_from_py_object)]
pub struct ApiResponse {
    #[pyo3(get)]
    status: i64,
    #[pyo3(get)]
    body: Py<PyAny>,
    #[pyo3(get)]
    cookie: Vec<String>,
}

impl ApiResponse {
    fn from_rs(py: Python<'_>, response: ncm_api_rs::ApiResponse) -> PyResult<Self> {
        Ok(Self {
            status: response.status,
            body: pythonize(py, &response.body)?.unbind(),
            cookie: response.cookie,
        })
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

#[gen_stub_pyclass]
#[pyclass(module = "ncm_api_py")]
pub struct Client {
    inner: ApiClient,
}

#[gen_stub_pymethods]
#[pymethods]
impl Client {
    #[new]
    #[pyo3(signature = (cookie=None))]
    fn new(cookie: Option<String>) -> Self {
        Self {
            inner: ncm_api_rs::create_client(cookie),
        }
    }

    /// Search using upstream defaults for omitted parameters.
    #[gen_stub(override_return_type(type_repr = "typing.Awaitable[ApiResponse]", imports = ("typing")))]
    #[pyo3(signature = (keywords, r#type=None, limit=None, offset=None))]
    fn search<'py>(
        &self,
        py: Python<'py>,
        keywords: String,
        r#type: Option<i64>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let query = search_query(&keywords, r#type, limit, offset);
        let inner = self.inner.clone();
        response_future(py, async move { inner.search(&query).await })
    }

    /// Fetch song details without additional ID validation.
    #[gen_stub(override_return_type(type_repr = "typing.Awaitable[ApiResponse]", imports = ("typing")))]
    #[pyo3(signature = (ids))]
    fn song_detail<'py>(&self, py: Python<'py>, ids: Vec<i64>) -> PyResult<Bound<'py, PyAny>> {
        let query = Query::new().param("ids", &song_ids(ids));
        let inner = self.inner.clone();
        response_future(py, async move { inner.song_detail(&query).await })
    }

    /// Fetch playback URLs without additional quality validation.
    #[gen_stub(override_return_type(type_repr = "typing.Awaitable[ApiResponse]", imports = ("typing")))]
    #[pyo3(signature = (ids, level=None))]
    fn song_url_v1<'py>(
        &self,
        py: Python<'py>,
        ids: Vec<i64>,
        level: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let mut query = Query::new().param("id", &song_ids(ids));
        if let Some(level) = level {
            query = query.param("level", &level);
        }
        let inner = self.inner.clone();
        response_future(py, async move { inner.song_url_v1(&query).await })
    }
}

#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (cookie=None))]
fn create_client(cookie: Option<String>) -> Client {
    Client::new(cookie)
}

pyo3_stub_gen::define_stub_info_gatherer!(stub_info);

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
    fn leaves_defaults_to_upstream() {
        let query = search_query("", None, None, None);
        assert_eq!(query.get("keywords"), Some(""));
        for name in ["type", "limit", "offset"] {
            assert_eq!(query.get(name), None);
        }
    }

    #[test]
    fn passes_values_without_business_validation() {
        let query = search_query("music", Some(i64::MAX), Some(0), Some(-1));
        assert_eq!(query.get("type"), Some("9223372036854775807"));
        assert_eq!(query.get("limit"), Some("0"));
        assert_eq!(query.get("offset"), Some("-1"));
        assert_eq!(song_ids(vec![]), "");
        assert_eq!(
            song_ids(vec![0, -1, i64::MAX, 0]),
            "0,-1,9223372036854775807,0"
        );
    }

    #[test]
    fn preserves_json_values_and_response_metadata() {
        Python::initialize();
        Python::attach(|py| {
            for body in [
                serde_json::json!({"data": [{"url": null}]}),
                serde_json::json!([1, "text", false]),
                serde_json::Value::Null,
            ] {
                let response = ApiResponse::from_rs(
                    py,
                    ncm_api_rs::ApiResponse {
                        status: 201,
                        body: body.clone(),
                        cookie: vec!["session=value".to_owned()],
                    },
                )
                .unwrap();
                assert_eq!(response.status, 201);
                assert_eq!(response.cookie, ["session=value"]);
                let decoded: serde_json::Value =
                    pythonize::depythonize(response.body.bind(py)).unwrap();
                assert_eq!(decoded, body);
            }
        });
    }

    #[test]
    fn preserves_upstream_errors() {
        Python::initialize();
        Python::attach(|py| {
            let error = ncm_api_rs::NcmError::AuthRequired("login required".to_owned());
            let message = error.to_string();
            let error = into_py_result::<()>(Err(error)).unwrap_err();
            assert!(error.is_instance_of::<pyo3::exceptions::PyRuntimeError>(py));
            assert_eq!(error.value(py).to_string(), message);
        });
    }
}
