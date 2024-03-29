// Copyright 2019-2021 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::cmp;
use std::future::Future;
use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
use std::pin::Pin;
use std::task::{Context, Poll};

use log::*;

use super::middleware::Middleware;
use super::response;
use super::response::{internal_error, malformed};

use futures_channel::mpsc;
use futures_util::{future::join_all, stream::StreamExt, FutureExt};
use hyper::header::{HeaderMap, HeaderValue};
use hyper::server::{conn::AddrIncoming, Builder as HyperBuilder};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Error as HyperError, Method};
use jsonrpsee::core::error::{Error, GenericTransportError};
use jsonrpsee::core::http_helpers::{self, read_body};
use jsonrpsee::core::server::helpers::{collect_batch_response, prepare_error, MethodSink};
use jsonrpsee::core::server::resource_limiting::Resources;
use jsonrpsee::core::server::rpc_module::{MethodKind, Methods};
use jsonrpsee::core::TEN_MB_SIZE_BYTES;
use jsonrpsee::types::error::ErrorCode;
use jsonrpsee::types::{Id, Notification, Params, Request};
use serde_json::value::RawValue;
use socket2::{Domain, Socket, Type};

/// Builder to create JSON-RPC HTTP server.
#[derive(Debug)]
pub struct Builder<M = ()> {
    resources: Resources,
    max_request_body_size: u32,
    keep_alive: bool,
    /// Custom tokio runtime to run the server on.
    tokio_runtime: Option<tokio::runtime::Handle>,
    middleware: M,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            max_request_body_size: TEN_MB_SIZE_BYTES,
            resources: Resources::default(),
            keep_alive: true,
            tokio_runtime: None,
            middleware: (),
        }
    }
}

impl Builder {
    /// Create a default server builder.
    pub fn new() -> Self {
        Self::default()
    }
}

impl<M> Builder<M> {
    /// Add a middleware to the builder [`Middleware`](../jsonrpsee_core/middleware/trait.Middleware.html).
    ///
    /// ```
    /// use std::time::Instant;
    ///
    /// use gpuproxy::http_server::middleware::Middleware;
    /// use gpuproxy::http_server::HttpServerBuilder;
    ///
    /// #[derive(Clone)]
    /// struct MyMiddleware;
    ///
    /// impl Middleware for MyMiddleware {
    ///
    ///     fn on_request(&self) -> bool {
    ///        true
    ///     }
    ///
    ///     fn on_result(&self) {
    ///        println!("do nothing")
    ///     }
    /// }
    ///
    /// let builder = HttpServerBuilder::new().set_middleware(MyMiddleware);
    /// ```
    pub fn set_middleware<T: Middleware>(self, middleware: T) -> Builder<T> {
        Builder {
            max_request_body_size: self.max_request_body_size,
            resources: self.resources,
            keep_alive: self.keep_alive,
            tokio_runtime: self.tokio_runtime,
            middleware,
        }
    }

    /// Sets the maximum size of a request body in bytes (default is 10 MiB).
    pub fn max_request_body_size(mut self, size: u32) -> Self {
        self.max_request_body_size = size;
        self
    }

    /// Enables or disables HTTP keep-alive.
    ///
    /// Default is true.
    pub fn keep_alive(mut self, keep_alive: bool) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    /// Register a new resource kind. Errors if `label` is already registered, or if the number of
    /// registered resources on this server instance would exceed 8.
    ///
    /// See the module documentation for [`resurce_limiting`](../jsonrpsee_utils/server/resource_limiting/index.html#resource-limiting)
    /// for details.
    pub fn register_resource(
        mut self,
        label: &'static str,
        capacity: u16,
        default: u16,
    ) -> Result<Self, Error> {
        self.resources.register(label, capacity, default)?;

        Ok(self)
    }

    /// Configure a custom [`tokio::runtime::Handle`] to run the server on.
    ///
    /// Default: [`tokio::spawn`]
    pub fn custom_tokio_runtime(mut self, rt: tokio::runtime::Handle) -> Self {
        self.tokio_runtime = Some(rt);
        self
    }

    /// Finalizes the configuration of the server.
    ///
    /// ```rust
    /// #[tokio::main]
    /// async fn main() {
    ///   let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    ///   let occupied_addr = listener.local_addr().unwrap();
    ///   let addrs: &[std::net::SocketAddr] = &[
    ///       occupied_addr,
    ///       "127.0.0.1:0".parse().unwrap(),
    ///   ];
    ///   assert!(jsonrpsee::http_server::HttpServerBuilder::default().build(occupied_addr).is_err());
    ///   assert!(jsonrpsee::http_server::HttpServerBuilder::default().build(addrs).is_ok());
    /// }
    /// ```
    pub fn build(self, addrs: impl ToSocketAddrs) -> Result<Server<M>, Error> {
        let mut err: Option<Error> = None;

        for addr in addrs.to_socket_addrs()? {
            let (listener, local_addr) = match self.inner_builder(addr) {
                Ok(res) => res,
                Err(e) => {
                    err = Some(e);
                    continue;
                }
            };

            return Ok(Server {
                listener,
                local_addr,
                max_request_body_size: self.max_request_body_size,
                resources: self.resources,
                tokio_runtime: self.tokio_runtime,
                middleware: self.middleware,
            });
        }

        let err = err.unwrap_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "No address found").into()
        });
        Err(err)
    }

    fn inner_builder(
        &self,
        addr: SocketAddr,
    ) -> Result<
        (
            hyper::server::Builder<hyper::server::conn::AddrIncoming>,
            Option<SocketAddr>,
        ),
        Error,
    > {
        let domain = Domain::for_address(addr);
        let socket = Socket::new(domain, Type::STREAM, None)?;
        socket.set_nodelay(true)?;
        socket.set_reuse_address(true)?;
        socket.set_nonblocking(true)?;
        socket.set_keepalive(self.keep_alive)?;
        let address = addr.into();
        socket.bind(&address)?;

        socket.listen(128)?;
        let listener: TcpListener = socket.into();
        let local_addr = listener.local_addr().ok();
        let listener = hyper::Server::from_tcp(listener)?;
        Ok((listener, local_addr))
    }
}

/// Handle used to run or stop the server.
#[derive(Debug)]
pub struct ServerHandle {
    stop_sender: mpsc::Sender<()>,
    pub(crate) handle: Option<tokio::task::JoinHandle<()>>,
}

impl ServerHandle {
    /// Requests server to stop. Returns an error if server was already stopped.
    pub fn stop(mut self) -> Result<tokio::task::JoinHandle<()>, Error> {
        let stop = self.stop_sender.try_send(()).map(|_| self.handle.take());
        match stop {
            Ok(Some(handle)) => Ok(handle),
            _ => Err(Error::AlreadyStopped),
        }
    }
}

impl Future for ServerHandle {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let handle = match &mut self.handle {
            Some(handle) => handle,
            None => return Poll::Ready(()),
        };

        handle.poll_unpin(cx).map(|_| ())
    }
}

/// An HTTP JSON RPC server.
#[derive(Debug)]
pub struct Server<M = ()> {
    /// Hyper server.
    listener: HyperBuilder<AddrIncoming>,
    /// Local address
    local_addr: Option<SocketAddr>,
    /// Max request body size.
    max_request_body_size: u32,
    /// Tracker for currently used resources on the server
    resources: Resources,
    /// Custom tokio runtime to run the server on.
    tokio_runtime: Option<tokio::runtime::Handle>,
    middleware: M,
}

impl<M: Middleware> Server<M> {
    /// Returns socket address to which the server is bound.
    pub fn local_addr(&self) -> Result<SocketAddr, Error> {
        self.local_addr
            .ok_or_else(|| Error::Custom("Local address not found".into()))
    }

    /// Start the server.
    pub fn start(mut self, methods: impl Into<Methods>) -> Result<ServerHandle, Error> {
        let max_request_body_size = self.max_request_body_size;
        let (tx, mut rx) = mpsc::channel(1);
        let listener = self.listener;
        let resources = self.resources;
        let middleware = self.middleware;
        let methods = methods.into().initialize_resources(&resources)?;

        let make_service = make_service_fn(move |_| {
            let methods = methods.clone();
            let resources = resources.clone();
            let middleware = middleware.clone();

            async move {
                Ok::<_, HyperError>(service_fn(move |request| {
                    let methods = methods.clone();
                    let resources = resources.clone();
                    let middleware = middleware.clone();

                    // Run some validation on the http request, then read the body and try to deserialize it into one of
                    // two cases: a single RPC request or a batch of RPC requests.
                    async move {
                        // Only `POST` and `OPTIONS` methods are allowed.
                        match *request.method() {
                            Method::POST if content_type_is_json(&request) => {
                                let origin =
                                    return_origin_if_different_from_host(request.headers())
                                        .cloned();
                                let res = process_validated_request(
                                    request,
                                    middleware,
                                    methods,
                                    resources,
                                    max_request_body_size,
                                )
                                .await?;
                                Ok::<_, HyperError>(res)
                            }
                            // Error scenarios:
                            Method::POST => Ok(response::unsupported_content_type()),
                            _ => Ok(response::method_not_allowed()),
                        }
                    }
                }))
            }
        });

        let rt = match self.tokio_runtime.take() {
            Some(rt) => rt,
            None => tokio::runtime::Handle::current(),
        };

        let handle = rt.spawn(async move {
            let server = listener.serve(make_service);
            let _ = server
                .with_graceful_shutdown(async move { rx.next().await.map_or((), |_| ()) })
                .await;
        });

        Ok(ServerHandle {
            handle: Some(handle),
            stop_sender: tx,
        })
    }
}

// Checks the origin and host headers. If they both exist, return the origin if it does not match the host.
// If one of them doesn't exist (origin most probably), or they are identical, return None.
fn return_origin_if_different_from_host(headers: &HeaderMap) -> Option<&HeaderValue> {
    if let (Some(origin), Some(host)) = (headers.get("origin"), headers.get("host")) {
        if origin != host {
            Some(origin)
        } else {
            None
        }
    } else {
        None
    }
}

/// Checks that content type of received request is valid for JSON-RPC.
fn content_type_is_json(request: &hyper::Request<hyper::Body>) -> bool {
    is_json(request.headers().get("content-type"))
}

/// Returns true if the `content_type` header indicates a valid JSON message.
fn is_json(content_type: Option<&hyper::header::HeaderValue>) -> bool {
    matches!(content_type.and_then(|val| val.to_str().ok()), Some(content) if content.eq_ignore_ascii_case("application/json")
               || content.eq_ignore_ascii_case("application/json; charset=utf-8")
                || content.eq_ignore_ascii_case("application/json;charset=utf-8"))
}

/// Process a verified request, it implies a POST request with content type JSON.
async fn process_validated_request(
    request: hyper::Request<hyper::Body>,
    middleware: impl Middleware,
    methods: Methods,
    resources: Resources,
    max_request_body_size: u32,
) -> Result<hyper::Response<hyper::Body>, HyperError> {
    let (parts, body) = request.into_parts();
    let (body, mut is_single) = match read_body(&parts.headers, body, max_request_body_size).await {
        Ok(r) => r,
        Err(GenericTransportError::TooLarge) => return Ok(response::too_large()),
        Err(GenericTransportError::Malformed) => return Ok(response::malformed()),
        Err(GenericTransportError::Inner(e)) => {
            error!("Internal error reading request body: {}", e);
            return Ok(response::internal_error());
        }
    };

    let request_start = middleware.on_request();

    // NOTE(niklasad1): it's a channel because it's needed for batch requests.
    let (tx, mut rx) = mpsc::unbounded::<String>();
    let sink = MethodSink::new_with_limit(tx, max_request_body_size);

    type Notif<'a> = Notification<'a, Option<&'a RawValue>>;

    // Single request or notification
    if is_single {
        if let Ok(req) = serde_json::from_slice::<Request>(&body) {
            let method = req.method.as_ref();

            let id = req.id.clone();
            let params = Params::new(req.params.map(|params| params.get()));

            let result = match methods.method_with_name(method) {
                None => {
                    sink.send_error(req.id, ErrorCode::MethodNotFound.into());
                    false
                }
                Some((name, method_callback)) => match method_callback.inner() {
                    MethodKind::Sync(callback) => {
                        match method_callback.claim(&req.method, &resources) {
                            Ok(guard) => {
                                let result = (callback)(id, params, &sink);
                                drop(guard);
                                result
                            }
                            Err(err) => {
                                error!("[Methods::execute_with_resources] failed to lock resources: {:?}", err);
                                sink.send_error(req.id, ErrorCode::ServerIsBusy.into());
                                false
                            }
                        }
                    }
                    MethodKind::Async(callback) => {
                        match method_callback.claim(name, &resources) {
                            Ok(guard) => {
                                (callback)(
                                    id.into_owned(),
                                    params.into_owned(),
                                    sink.clone(),
                                    0,
                                    Some(guard),
                                )
                                .await
                            }
                            Err(err) => {
                                error!("[Methods::execute_with_resources] failed to lock resources: {:?}", err);
                                sink.send_error(req.id, ErrorCode::ServerIsBusy.into());
                                false
                            }
                        }
                    }
                    MethodKind::Subscription(_) => {
                        error!("Subscriptions not supported on HTTP");
                        sink.send_error(req.id, ErrorCode::InternalError.into());
                        false
                    }
                },
            };
            middleware.on_result();
        } else if let Ok(_req) = serde_json::from_slice::<Notif>(&body) {
            return Ok::<_, HyperError>(response::ok_response("".into()));
        } else {
            let (id, code) = prepare_error(&body);
            sink.send_error(id, code.into());
        }
    // Batch of requests or notifications
    } else if let Ok(batch) = serde_json::from_slice::<Vec<Request>>(&body) {
        if !batch.is_empty() {
            let middleware = &middleware;

            join_all(batch.into_iter().filter_map(move |req| {
                let id = req.id.clone();
                let params = Params::new(req.params.map(|params| params.get()));

                match methods.method_with_name(&req.method) {
                    None => {
                        sink.send_error(req.id, ErrorCode::MethodNotFound.into());
                        None
                    }
                    Some((name, method_callback)) => match method_callback.inner() {
                        MethodKind::Sync(callback) => match method_callback.claim(name, &resources) {
                            Ok(guard) => {
                                let result = (callback)(id, params, &sink);
                                middleware.on_result();
                                drop(guard);
                                None
                            }
                            Err(err) => {
                                error!("[Methods::execute_with_resources] failed to lock resources: {:?}", err);
                                sink.send_error(req.id, ErrorCode::ServerIsBusy.into());
                                middleware.on_result();
                                None
                            }
                        },
                        MethodKind::Async(callback) => match method_callback.claim(name, &resources) {
                            Ok(guard) => {
                                let sink = sink.clone();
                                let id = id.into_owned();
                                let params = params.into_owned();
                                let callback = callback.clone();

                                Some(async move {
                                    let result = (callback)(id, params, sink, 0, Some(guard)).await;
                                    middleware.on_result();
                                })
                            }
                            Err(err) => {
                                error!("[Methods::execute_with_resources] failed to lock resources: {:?}", err);
                                sink.send_error(req.id, ErrorCode::ServerIsBusy.into());
                                middleware.on_result();
                                None
                            }
                        },
                        MethodKind::Subscription(_) => {
                            error!("Subscriptions not supported on HTTP");
                            sink.send_error(req.id, ErrorCode::InternalError.into());
                            middleware.on_result();
                            None
                        }
                    },
                }
            }))
            .await;
        } else {
            // "If the batch rpc call itself fails to be recognized as an valid JSON or as an
            // Array with at least one value, the response from the Server MUST be a single
            // Response object." – The Spec.
            is_single = true;
            sink.send_error(Id::Null, ErrorCode::InvalidRequest.into());
        }
    } else if let Ok(_batch) = serde_json::from_slice::<Vec<Notif>>(&body) {
        return Ok(response::ok_response("".into()));
    } else {
        // "If the batch rpc call itself fails to be recognized as an valid JSON or as an
        // Array with at least one value, the response from the Server MUST be a single
        // Response object." – The Spec.
        is_single = true;
        let (id, code) = prepare_error(&body);
        sink.send_error(id, code.into());
    }

    // Closes the receiving half of a channel without dropping it. This prevents any further
    // messages from being sent on the channel.
    rx.close();
    let response = if is_single {
        rx.next()
            .await
            .expect("Sender is still alive managed by us above; qed")
    } else {
        collect_batch_response(rx).await
    };
    debug!(
        "[service_fn] sending back: {:?}",
        &response[..cmp::min(response.len(), 1024)]
    );
    Ok(response::ok_response(response))
}
