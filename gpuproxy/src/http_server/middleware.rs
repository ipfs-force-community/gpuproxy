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

//! Middleware for `jsonrpsee` servers.

/// Defines a middleware with callbacks during the RPC request life-cycle. The primary use case for
/// this is to collect timings for a larger metrics collection solution but the only constraints on
/// the associated type is that it be [`Send`] and [`Copy`], giving users some freedom to do what
/// they need to do.
///
/// See the [`WsServerBuilder::set_middleware`](../../jsonrpsee_ws_server/struct.WsServerBuilder.html#method.set_middleware)
/// or the [`HttpServerBuilder::set_middleware`](../../jsonrpsee_http_server/struct.HttpServerBuilder.html#method.set_middleware) method
/// for examples.
pub trait Middleware: Send + Sync + Clone + 'static {
    /// Called when a new JSON-RPC comes to the server.
    fn on_request(&self) -> bool;

    /// Called on each JSON-RPC method completion, batch requests will trigger `on_result` multiple times.
    fn on_result(&self) {}
}
