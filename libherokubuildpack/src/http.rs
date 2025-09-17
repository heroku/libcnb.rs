use bullet_stream::style;
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
pub async fn get<U>(
    #[builder(start_fn)] //
    url: U,
    headers: Option<HeaderMap>,
    #[builder(default = DEFAULT_CONNECT_TIMEOUT)] //
    connect_timeout: Duration,
    #[builder(default = DEFAULT_READ_TIMEOUT)] //
    read_timeout: Duration,
    #[builder(default = DEFAULT_RETRIES)] //
    max_retries: u32,
) -> Result<Response, HttpError>
where
    U: IntoUrl + std::fmt::Display + Clone,
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
    .with(RetryLoggingMiddleware::new(max_retries))
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
impl<U, State> GetBuilder<U, State>
where
    U: IntoUrl + std::fmt::Display + Clone,
    State: get_builder::State,
{
    pub fn call_sync(self) -> Result<Response, HttpError>
    where
        State: get_builder::IsComplete,
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

struct RetryLoggingMiddleware {
    count: AtomicU32,
    max_retries: u32,
}

impl RetryLoggingMiddleware {
    fn new(max_retries: u32) -> Self {
        Self {
            count: AtomicU32::new(0),
            max_retries,
        }
    }
}

#[async_trait::async_trait]
impl Middleware for RetryLoggingMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> reqwest_middleware::Result<Response> {
        // increment and acquire the previous value
        let previous_value = self.count.fetch_add(1, SeqCst);
        let message = if previous_value == 0 {
            format!("{} {}", req.method(), style::url(req.url()))
        } else {
            format!("Retry attempt {previous_value} of {}", self.max_retries)
        };
        let timer = bullet_stream::global::print::sub_start_timer(message);
        let response = next.run(req, extensions).await;
        match &response {
            Ok(response) => {
                if let Some(reason) = response.status().canonical_reason() {
                    timer.cancel(reason);
                } else {
                    timer.cancel(format!("Status {}", response.status().as_str()));
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
                timer.cancel(reason);
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
    use bullet_stream::{global, strip_ansi};
    use indoc::indoc;
    use regex::Regex;
    use reqwest::StatusCode;
    use std::future::Future;
    use std::ops::{Div, Mul};
    use std::{env, fs, net};
    use tempfile::NamedTempFile;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

    #[test]
    fn test_download_success_no_retry() {
        let mock_download = ASYNC_RUNTIME.block_on(
            setup_mock_download()
                .respond_with_success_after_n_requests(1)
                .call(),
        );

        let dst = NamedTempFile::new().unwrap();

        let log = global::with_locked_writer(Vec::<u8>::new(), || {
            get(mock_download.url())
                .call_sync()
                .unwrap()
                .download_to_file_sync(dst.path())
                .unwrap();
        });

        assert_download_contents(&dst);
        assert_log_contains_matches(
            &log,
            &[
                request_success_matcher(mock_download.url()),
                download_file_matcher(),
            ],
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

        let log = global::with_locked_writer(Vec::<u8>::new(), || {
            get(mock_download.url())
                .max_retries(2)
                .call_sync()
                .unwrap()
                .download_to_file_sync(dst.path())
                .unwrap();
        });

        assert_download_contents(&dst);
        assert_log_contains_matches(
            &log,
            &[
                request_failed_matcher(mock_download.url(), "Internal Server Error"),
                retry_attempt_success_matcher(1, 2),
                download_file_matcher(),
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

        let log = global::with_locked_writer(Vec::<u8>::new(), || {
            get(mock_download.url())
                .max_retries(2)
                .call_sync()
                .unwrap_err();
        });

        assert_no_download_contents(&dst);
        assert_log_contains_matches(
            &log,
            &[
                request_failed_matcher(mock_download.url(), "Internal Server Error"),
                retry_attempt_failed_matcher(1, 2, "Internal Server Error"),
                retry_attempt_failed_matcher(2, 2, "Internal Server Error"),
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

        let log = global::with_locked_writer(Vec::<u8>::new(), || {
            get(download_url)
                .max_retries(2)
                .connect_timeout(connect_timeout)
                .read_timeout(read_timeout)
                .call_sync()
                .unwrap_err();
        });

        assert_no_download_contents(&dst);
        assert_log_contains_matches(
            &log,
            &[
                request_failed_matcher(download_url, "connection refused"),
                retry_attempt_failed_matcher(1, 2, "connection refused"),
                retry_attempt_failed_matcher(2, 2, "connection refused"),
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

        let log = global::with_locked_writer(Vec::<u8>::new(), || {
            get(server.uri())
                .max_retries(2)
                .read_timeout(read_timeout)
                .call_sync()
                .unwrap_err();
        });

        assert_no_download_contents(&dst);

        assert_log_contains_matches(
            &log,
            &[
                request_failed_matcher(server.uri(), "timed out"),
                retry_attempt_failed_matcher(1, 2, "timed out"),
                retry_attempt_failed_matcher(2, 2, "timed out"),
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

        global::with_locked_writer(Vec::<u8>::new(), || {
            assert!(
                get("https://self-signed.badssl.com")
                    .max_retries(0)
                    .call_sync()
                    .is_err()
            );
        });

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

        global::with_locked_writer(Vec::<u8>::new(), || {
            assert!(
                get("https://self-signed.badssl.com")
                    .max_retries(0)
                    .call_sync()
                    .is_ok()
            );
        });

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

    fn assert_log_contains_matches(log: &[u8], matchers: &[Regex]) {
        let output = strip_ansi(String::from_utf8_lossy(log));
        let actual_lines = output.lines().map(str::trim).collect::<Vec<_>>();
        assert_eq!(
            matchers.len(),
            actual_lines.len(),
            "Expected matchers does not match length of actual logged lines\nMatchers:\n{matchers:?}\nLines: {actual_lines:?}"
        );
        actual_lines
            .iter()
            .zip(matchers.iter())
            .for_each(|(actual, matcher)| {
                assert!(
                    matcher.is_match(actual),
                    "Expected matcher did not match line\nMatcher: {matcher:?}\nLine: {actual}"
                );
            });
    }

    const PROGRESS_DOTS: &str = r"\.+";
    const TIMER: &str = r"\(< \d+\.\d+s\)";

    fn request_success_matcher(url: impl AsRef<str>) -> Regex {
        let url = url.as_ref().replace('.', r"\.");
        Regex::new(&format!(r"- GET {url} {PROGRESS_DOTS} \(OK\)")).unwrap()
    }

    fn request_failed_matcher(url: impl AsRef<str>, reason: &str) -> Regex {
        let url = url.as_ref().replace('.', r"\.");
        Regex::new(&format!(r"- GET {url} {PROGRESS_DOTS} \({reason}\)")).unwrap()
    }

    fn retry_attempt_success_matcher(attempt: u32, max_retries: u32) -> Regex {
        Regex::new(&format!(
            r"- Retry attempt {attempt} of {max_retries} {PROGRESS_DOTS} \(OK\)"
        ))
        .unwrap()
    }

    fn retry_attempt_failed_matcher(attempt: u32, max_retries: u32, reason: &str) -> Regex {
        Regex::new(&format!(
            r"- Retry attempt {attempt} of {max_retries} {PROGRESS_DOTS} \({reason}\)"
        ))
        .unwrap()
    }

    fn download_file_matcher() -> Regex {
        Regex::new(&format!(r"- Downloading {PROGRESS_DOTS} {TIMER}")).unwrap()
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
