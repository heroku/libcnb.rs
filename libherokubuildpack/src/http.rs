use futures::stream::TryStreamExt;
use http::{Extensions, HeaderMap};
use reqwest::{IntoUrl, Request, Response};
use reqwest_middleware::{ClientBuilder, Middleware, Next};
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_tracing::{SpanBackendWithUrl, TracingMiddleware};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio_util::compat::FuturesAsyncReadCompatExt;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);

const DEFAULT_RETRIES: u32 = 5;

#[bon::builder]
pub async fn get<U, T>(
    #[builder(start_fn)] //
    url: U,
    headers: Option<HeaderMap>,
    #[builder(default = DEFAULT_CONNECT_TIMEOUT)] //
    connect_timeout: Duration,
    #[builder(default = DEFAULT_READ_TIMEOUT)] //
    read_timeout: Duration,
    #[builder(default = DEFAULT_RETRIES)] //
    max_retries: u32,
    request_logger: RequestLogger<T>,
) -> Result<Response, HttpError>
where
    U: IntoUrl + std::fmt::Display + Clone,
    RetryLoggingMiddleware<T>: Middleware,
{
    let client = ClientBuilder::new(
        reqwest::ClientBuilder::new()
            .use_rustls_tls()
            .connect_timeout(connect_timeout)
            .read_timeout(read_timeout)
            .build()
            .expect("Should be able to construct the HTTP client"),
    )
    .with(RetryTransientMiddleware::new_with_policy(
        ExponentialBackoff::builder().build_with_max_retries(max_retries),
    ))
    .with(TracingMiddleware::<SpanBackendWithUrl>::new())
    .with(RetryLoggingMiddleware::new(max_retries, request_logger))
    .build();

    let mut request_builder = client.get(url.clone());

    if let Some(headers) = headers {
        request_builder = request_builder.headers(headers);
    }

    request_builder
        .send()
        .await
        .and_then(|res| {
            res.error_for_status()
                .map_err(reqwest_middleware::Error::Reqwest)
        })
        .map_err(|e| HttpError::Request(url.to_string(), e))
}

/// Extend the [`bon::builder`] for [`get`]
impl<U, T, State> GetBuilder<U, T, State>
where
    U: IntoUrl + std::fmt::Display + Clone,
    State: get_builder::State,
    RetryLoggingMiddleware<T>: Middleware,
{
    pub fn call_sync(self) -> Result<Response, HttpError>
    where
        State: get_builder::IsComplete,
        RetryLoggingMiddleware<T>: Middleware,
    {
        ASYNC_RUNTIME.block_on(async { self.call().await })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HttpError {
    #[error("Request to `{0}` failed\nError: {1}")]
    Request(String, reqwest_middleware::Error),
    #[error("Reading response from request to `{0}` failed\nError: {1}")]
    ReadResponse(String, reqwest_middleware::Error),
    #[error("Could not open file at `{0}` for download of `{1}`\nError: {2}")]
    OpenFile(PathBuf, String, io::Error),
    #[error("Could not write to file at `{0}` for download of `{1}`\nError: {2}")]
    WriteFile(PathBuf, String, io::Error),
}

pub trait ResponseExt {
    #[allow(async_fn_in_trait)]
    async fn download_to_file(self, download_to: impl AsRef<Path>) -> Result<(), HttpError>;
    fn download_to_file_sync(self, download_to: impl AsRef<Path>) -> Result<(), HttpError>;
}

impl ResponseExt for Response {
    async fn download_to_file(self, download_to: impl AsRef<Path>) -> Result<(), HttpError> {
        let url = &self.url().clone();
        let to_file = download_to.as_ref();

        let timer = bullet_stream::global::print::sub_start_timer("Downloading");

        let mut reader = FuturesAsyncReadCompatExt::compat(
            self.bytes_stream()
                .map_err(io::Error::other)
                .into_async_read(),
        );

        let mut writer = tokio::fs::File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(to_file)
            .await
            .map_err(|e| HttpError::OpenFile(to_file.to_path_buf(), url.to_string(), e))?;

        tokio::io::copy(&mut reader, &mut writer)
            .await
            .map_err(|e| HttpError::WriteFile(to_file.to_path_buf(), url.to_string(), e))?;

        timer.done();

        Ok(())
    }

    fn download_to_file_sync(self, download_to: impl AsRef<Path>) -> Result<(), HttpError> {
        ASYNC_RUNTIME.block_on(async { self.download_to_file(download_to).await })
    }
}

pub struct RequestLogger<T> {
    pub on_request_start: Box<dyn Fn(String) -> T + Send + Sync + 'static>,
    pub on_request_end: Box<dyn Fn(T, String) + Send + Sync + 'static>,
}

#[cfg(feature = "bullet_stream")]
#[must_use]
pub fn bullet_stream_request_logger() -> RequestLogger<bullet_stream::GlobalTimer> {
    RequestLogger {
        on_request_start: Box::new(bullet_stream::global::print::sub_start_timer),
        on_request_end: Box::new(bullet_stream::GlobalTimer::cancel),
    }
}

pub struct RetryLoggingMiddleware<T> {
    count: AtomicU32,
    max_retries: u32,
    request_logger: RequestLogger<T>,
}

impl<T> RetryLoggingMiddleware<T> {
    fn new(max_retries: u32, request_logger: RequestLogger<T>) -> Self {
        Self {
            count: AtomicU32::new(0),
            max_retries,
            request_logger,
        }
    }
}

#[async_trait::async_trait]
impl<T> Middleware for RetryLoggingMiddleware<T>
where
    T: Send + Sync + 'static,
{
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        // increment and acquire the previous value
        let previous_value = self.count.fetch_add(1, SeqCst);
        let message = if previous_value == 0 {
            format!("{} {}", req.method(), bullet_stream::style::url(req.url()))
        } else {
            format!("Retry attempt {previous_value} of {}", self.max_retries)
        };
        let on_request_start_value = (self.request_logger.on_request_start)(message);

        let response = next.run(req, extensions).await;

        match &response {
            Ok(response) => {
                if let Some(reason) = response.status().canonical_reason() {
                    (self.request_logger.on_request_end)(
                        on_request_start_value,
                        reason.to_string(),
                    );
                } else {
                    (self.request_logger.on_request_end)(
                        on_request_start_value,
                        format!("Status {}", response.status().as_str()),
                    );
                }
            }
            Err(e) => {
                let reason = if e.is_connect() {
                    "connection refused".into()
                } else if e.is_timeout() {
                    "timed out".into()
                } else {
                    e.to_string()
                };
                (self.request_logger.on_request_end)(on_request_start_value, reason);
            }
        }
        response
    }
}

static ASYNC_RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("Should be able to construct the Async Runtime")
});

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;
    use reqwest::StatusCode;
    use std::future::Future;
    use std::ops::{Div, Mul};
    use std::sync::{Arc, Mutex};
    use std::{env, fs, net};
    use tempfile::NamedTempFile;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

    struct TestLogger {
        messages: Arc<Mutex<Vec<String>>>,
    }

    impl TestLogger {
        fn new() -> Self {
            Self {
                messages: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn request_logger(&self) -> RequestLogger<Arc<Mutex<Vec<String>>>> {
            let messages = self.messages.clone();
            RequestLogger {
                on_request_start: Box::new(move |message| {
                    let mut writer = messages.lock().unwrap();
                    writer.push(message);
                    messages.clone()
                }),
                on_request_end: Box::new(|messages, message| {
                    let mut writer = messages.lock().unwrap();
                    writer.push(format!(" ... ({message})\n"));
                }),
            }
        }

        fn into_lines(self) -> Vec<String> {
            self.messages
                .lock()
                .unwrap()
                .clone()
                .join("")
                .lines()
                .map(String::from)
                .map(strip_ansi)
                .collect()
        }
    }

    // XXX: style information is still bleeding into our implementation, we shouldn't need this
    fn strip_ansi(contents: impl AsRef<str>) -> String {
        let contents = contents.as_ref();
        let mut result = String::with_capacity(contents.len());
        let mut in_sequence = false;
        for char in contents.chars() {
            // If current character is an escape, set the escape flag which will begin ignoring characters
            // until the end of the sequence is found.
            if char == '\x1B' {
                in_sequence = true;
            } else if in_sequence {
                // If we're in a sequence discard the character, an 'm' indicates the end of the sequence
                if char == 'm' {
                    in_sequence = false;
                }
            } else {
                result.push(char);
            }
        }
        result.shrink_to_fit();

        result
    }

    #[test]
    fn test_download_success_no_retry() {
        let mock_download = ASYNC_RUNTIME.block_on(
            setup_mock_download()
                .respond_with_success_after_n_requests(1)
                .call(),
        );

        let dst = NamedTempFile::new().unwrap();

        let log = TestLogger::new();

        get(mock_download.url())
            .request_logger(log.request_logger())
            .call_sync()
            .unwrap()
            .download_to_file_sync(dst.path())
            .unwrap();

        assert_download_contents(&dst);
        assert_eq!(
            log.into_lines(),
            &[format!("GET {} ... (OK)", mock_download.url())]
        );
    }

    #[test]
    fn test_download_success_after_retry() {
        let mock_download = ASYNC_RUNTIME.block_on(
            setup_mock_download()
                .respond_with_success_after_n_requests(2)
                .call(),
        );

        let dst = NamedTempFile::new().unwrap();

        let log = TestLogger::new();

        get(mock_download.url())
            .max_retries(2)
            .request_logger(log.request_logger())
            .call_sync()
            .unwrap()
            .download_to_file_sync(dst.path())
            .unwrap();

        assert_download_contents(&dst);
        assert_eq!(
            log.into_lines(),
            &[
                format!("GET {} ... (Internal Server Error)", mock_download.url()),
                "Retry attempt 1 of 2 ... (OK)".to_string()
            ],
        );
    }

    #[test]
    fn test_download_failed_after_retry() {
        let mock_download = ASYNC_RUNTIME.block_on(
            setup_mock_download()
                .respond_with_success_after_n_requests(4)
                // because we'll only ever get to 3 (first request + 2 retries)
                .expected_responses(3)
                .call(),
        );

        let dst = NamedTempFile::new().unwrap();

        let log = TestLogger::new();

        get(mock_download.url())
            .max_retries(2)
            .request_logger(log.request_logger())
            .call_sync()
            .unwrap_err();

        assert_no_download_contents(&dst);
        assert_eq!(
            log.into_lines(),
            &[
                format!("GET {} ... (Internal Server Error)", mock_download.url()),
                "Retry attempt 1 of 2 ... (Internal Server Error)".to_string(),
                "Retry attempt 2 of 2 ... (Internal Server Error)".to_string(),
            ],
        );
    }

    #[test]
    fn test_download_retry_connect_timeout() {
        // let respond_with_success_after_n_requests = 1;
        let connect_delay = Duration::from_secs(1);
        let connect_timeout = connect_delay.div(2);
        let read_timeout = connect_delay.mul(2);

        // Borrowed this url from the reqwest test for connect timeouts
        let download_url = "http://192.0.2.1:81/slow";

        let dst = NamedTempFile::new().unwrap();

        let log = TestLogger::new();

        get(download_url)
            .request_logger(log.request_logger())
            .max_retries(2)
            .connect_timeout(connect_timeout)
            .read_timeout(read_timeout)
            .call_sync()
            .unwrap_err();

        assert_no_download_contents(&dst);
        assert_eq!(
            log.into_lines(),
            &[
                format!("GET {download_url} ... (connection refused)"),
                "Retry attempt 1 of 2 ... (connection refused)".to_string(),
                "Retry attempt 2 of 2 ... (connection refused)".to_string(),
            ],
        );
    }

    #[test]
    fn test_download_retry_response_timeout() {
        let read_timeout = Duration::from_millis(100);

        let server = test_server(move |_req| {
            async {
                // delay returning the response
                tokio::time::sleep(Duration::from_millis(300)).await;
                http::Response::default()
            }
        });

        let dst = NamedTempFile::new().unwrap();

        let log = TestLogger::new();

        get(server.uri())
            .request_logger(log.request_logger())
            .max_retries(2)
            .read_timeout(read_timeout)
            .call_sync()
            .unwrap_err();

        assert_no_download_contents(&dst);
        assert_eq!(
            log.into_lines(),
            &[
                format!("GET {} ... (timed out)", server.uri()),
                "Retry attempt 1 of 2 ... (timed out)".to_string(),
                "Retry attempt 2 of 2 ... (timed out)".to_string()
            ],
        );
    }

    #[test]
    #[allow(unsafe_code)]
    fn test_self_signed_cert() {
        // this is technically not thread-safe but should be okay for now
        // since this is the only test that sets this env var
        unsafe {
            env::remove_var("SSL_CERT_FILE");
        }

        assert!(
            get("https://self-signed.badssl.com")
                .request_logger(TestLogger::new().request_logger())
                .max_retries(0)
                .call_sync()
                .is_err()
        );

        let badssl_self_signed_cert_dir = tempfile::tempdir().unwrap();
        let badssl_self_signed_cert = badssl_self_signed_cert_dir
            .path()
            .join("badssl_self_signed_cert.pem");

        // https://github.com/rustls/rustls-native-certs/blob/main/tests/badssl-com-chain.pem
        fs::write(
            &badssl_self_signed_cert,
            indoc! { "
            -----BEGIN CERTIFICATE-----
            MIIDeTCCAmGgAwIBAgIJAMnA8BB8xT6wMA0GCSqGSIb3DQEBCwUAMGIxCzAJBgNV
            BAYTAlVTMRMwEQYDVQQIDApDYWxpZm9ybmlhMRYwFAYDVQQHDA1TYW4gRnJhbmNp
            c2NvMQ8wDQYDVQQKDAZCYWRTU0wxFTATBgNVBAMMDCouYmFkc3NsLmNvbTAeFw0y
            MTEwMTEyMDAzNTRaFw0yMzEwMTEyMDAzNTRaMGIxCzAJBgNVBAYTAlVTMRMwEQYD
            VQQIDApDYWxpZm9ybmlhMRYwFAYDVQQHDA1TYW4gRnJhbmNpc2NvMQ8wDQYDVQQK
            DAZCYWRTU0wxFTATBgNVBAMMDCouYmFkc3NsLmNvbTCCASIwDQYJKoZIhvcNAQEB
            BQADggEPADCCAQoCggEBAMIE7PiM7gTCs9hQ1XBYzJMY61yoaEmwIrX5lZ6xKyx2
            PmzAS2BMTOqytMAPgLaw+XLJhgL5XEFdEyt/ccRLvOmULlA3pmccYYz2QULFRtMW
            hyefdOsKnRFSJiFzbIRMeVXk0WvoBj1IFVKtsyjbqv9u/2CVSndrOfEk0TG23U3A
            xPxTuW1CrbV8/q71FdIzSOciccfCFHpsKOo3St/qbLVytH5aohbcabFXRNsKEqve
            ww9HdFxBIuGa+RuT5q0iBikusbpJHAwnnqP7i/dAcgCskgjZjFeEU4EFy+b+a1SY
            QCeFxxC7c3DvaRhBB0VVfPlkPz0sw6l865MaTIbRyoUCAwEAAaMyMDAwCQYDVR0T
            BAIwADAjBgNVHREEHDAaggwqLmJhZHNzbC5jb22CCmJhZHNzbC5jb20wDQYJKoZI
            hvcNAQELBQADggEBAC4DensZ5tCTeCNJbHABYPwwqLUFOMITKOOgF3t8EqOan0CH
            ST1NNi4jPslWrVhQ4Y3UbAhRBdqXl5N/NFfMzDosPpOjFgtifh8Z2s3w8vdlEZzf
            A4mYTC8APgdpWyNgMsp8cdXQF7QOfdnqOfdnY+pfc8a8joObR7HEaeVxhJs+XL4E
            CLByw5FR+svkYgCbQGWIgrM1cRpmXemt6Gf/XgFNP2PdubxqDEcnWlTMk8FCBVb1
            nVDSiPjYShwnWsOOshshCRCAiIBPCKPX0QwKDComQlRrgMIvddaSzFFTKPoNZjC+
            CUspSNnL7V9IIHvqKlRSmu+zIpm2VJCp1xLulk8=
            -----END CERTIFICATE-----
        "},
        )
        .unwrap();

        unsafe {
            env::set_var("SSL_CERT_FILE", badssl_self_signed_cert);
        }

        assert!(
            get("https://self-signed.badssl.com")
                .max_retries(0)
                .request_logger(TestLogger::new().request_logger())
                .call_sync()
                .is_ok()
        );

        unsafe {
            env::remove_var("SSL_CERT_FILE");
        }
    }

    #[bon::builder]
    async fn setup_mock_download(
        respond_with_success_after_n_requests: u32,
        expected_responses: Option<u32>,
    ) -> MockDownload {
        let server = MockServer::start().await;
        let expected_responses =
            expected_responses.unwrap_or(respond_with_success_after_n_requests);
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(RetryResponder::new(respond_with_success_after_n_requests))
            .expect(u64::from(expected_responses))
            .mount(&server)
            .await;
        MockDownload { server }
    }

    struct MockDownload {
        server: MockServer,
    }

    impl MockDownload {
        pub(crate) fn url(&self) -> String {
            format!("{}/", self.server.uri())
        }
    }

    struct RetryResponder {
        requests_attempted: AtomicU32,
        respond_with_success_after_n_requests: u32,
    }

    impl RetryResponder {
        fn new(respond_with_success_after_n_requests: u32) -> Self {
            Self {
                requests_attempted: AtomicU32::new(0),
                respond_with_success_after_n_requests,
            }
        }
    }

    impl Respond for RetryResponder {
        fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
            let requests_attempted = self.requests_attempted.fetch_add(1, SeqCst) + 1;

            if requests_attempted >= self.respond_with_success_after_n_requests {
                ResponseTemplate::new(StatusCode::OK).set_body_string(TEST_REQUEST_CONTENTS)
            } else {
                ResponseTemplate::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }

    const TEST_REQUEST_CONTENTS: &str = "test request";

    fn assert_download_contents(download_file: &NamedTempFile) {
        assert_eq!(
            fs::read_to_string(download_file.path()).unwrap(),
            TEST_REQUEST_CONTENTS
        );
    }

    fn assert_no_download_contents(download_file: &NamedTempFile) {
        assert_eq!(download_file.as_file().metadata().unwrap().len(), 0);
    }

    struct TestServer {
        addr: net::SocketAddr,
        panic_rx: std::sync::mpsc::Receiver<()>,
        shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    }

    impl TestServer {
        fn uri(&self) -> String {
            format!("http://{}/", self.addr)
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }

            if !::std::thread::panicking() {
                self.panic_rx
                    .recv_timeout(Duration::from_secs(3))
                    .expect("test server should not panic");
            }
        }
    }

    // matches reqwest test server to help simulate read response timeouts
    fn test_server<F, Fut>(func: F) -> TestServer
    where
        F: Fn(http::Request<hyper::body::Incoming>) -> Fut + Clone + Send + 'static,
        Fut: Future<Output = http::Response<reqwest::Body>> + Send + 'static,
    {
        let test_name = std::thread::current()
            .name()
            .unwrap_or("<unknown>")
            .to_string();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("new rt");
            let listener = rt.block_on(async move {
                tokio::net::TcpListener::bind(&std::net::SocketAddr::from(([127, 0, 0, 1], 0)))
                    .await
                    .unwrap()
            });
            let addr = listener.local_addr().unwrap();

            let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();
            let (panic_tx, panic_rx) = std::sync::mpsc::channel();
            let tname = format!("test({test_name})-support-server");
            std::thread::Builder::new()
                .name(tname)
                .spawn(move || {
                    rt.block_on(async move {
                        let builder =
                            hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new());

                        loop {
                            tokio::select! {
                            _ = &mut shutdown_rx => {
                                break;
                            }
                            accepted = listener.accept() => {
                                let (io, _) = accepted.expect("accepted");
                                let func = func.clone();
                                let svc = hyper::service::service_fn(move |req| {
                                    let fut = func(req);
                                    async move { Ok::<_, std::convert::Infallible>(fut.await) }
                                });
                                let builder = builder.clone();
                                tokio::spawn(async move {
                                    let _ = builder.serve_connection_with_upgrades(hyper_util::rt::TokioIo::new(io), svc).await;
                                });
                            }
                        }
                        }
                        let _ = panic_tx.send(());
                    });
                })
                .expect("thread spawn");

            TestServer {
                addr,
                panic_rx,
                shutdown_tx: Some(shutdown_tx),
            }
        })
            .join()
            .unwrap()
    }
}
