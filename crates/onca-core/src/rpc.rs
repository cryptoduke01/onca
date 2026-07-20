//! Solana JSON-RPC: request construction and response envelope handling, with
//! the actual HTTP kept behind a trait.
//!
//! The core never performs I/O. It builds the request body, hands it to a
//! [`RpcTransport`], and interprets the JSON-RPC envelope that comes back. On
//! wasm the plugin implements `RpcTransport` with the blocking `waki` client
//! (TLS is done host-side by `wasi:http`); in tests it is implemented by a
//! canned mock. This is what lets the whole RPC layer be exercised by
//! `cargo test` with zero network.

use serde_json::{json, Value};

use crate::error::{CoreError, Result};

/// The one thing a plugin must provide: POST a JSON body to the RPC URL and
/// return the raw response body as a string. Implementors own TLS, timeouts and
/// headers. The core supplies the URL it was configured with; implementors must
/// never log it (it may embed an API key).
pub trait RpcTransport {
    /// POST `body` (already-serialized JSON) to `url`, return the response text.
    fn post_json(&self, url: &str, body: &str) -> Result<String>;
}

/// A thin JSON-RPC 2.0 client bound to one endpoint and one transport.
pub struct RpcClient<'a, T: RpcTransport> {
    url: &'a str,
    transport: &'a T,
}

impl<'a, T: RpcTransport> RpcClient<'a, T> {
    /// Bind to an endpoint. `url` is the operator's RPC URL from config — it may
    /// contain an API key and must never be logged or echoed into output.
    pub fn new(url: &'a str, transport: &'a T) -> Self {
        RpcClient { url, transport }
    }

    /// Issue a single JSON-RPC call and return the `result` value. Maps a
    /// JSON-RPC `error` object to [`CoreError::RpcError`].
    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = build_request(1, method, params);
        let text = self
            .transport
            .post_json(self.url, &serde_json::to_string(&body).unwrap())?;
        parse_response(&text)
    }
}

/// Build a JSON-RPC 2.0 request object.
pub fn build_request(id: u64, method: &str, params: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    })
}

/// Parse a JSON-RPC 2.0 response string and return its `result`, or a mapped
/// error. Never surfaces the raw endpoint or full payload in the error.
pub fn parse_response(text: &str) -> Result<Value> {
    let v: Value = serde_json::from_str(text)
        .map_err(|e| CoreError::Rpc(format!("response was not JSON: {e}")))?;

    if let Some(err) = v.get("error") {
        let code = err.get("code").and_then(Value::as_i64).unwrap_or(0);
        let message = err
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        return Err(CoreError::RpcError { code, message });
    }

    v.get("result")
        .cloned()
        .ok_or_else(|| CoreError::Rpc("response had neither result nor error".into()))
}

/// Standard `{ "commitment": "..." }` config object many methods accept.
pub fn commitment(level: &str) -> Value {
    json!({ "commitment": level })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A mock transport that records the last request and returns a canned body.
    struct MockTransport {
        response: String,
        last_body: RefCell<String>,
    }
    impl MockTransport {
        fn new(response: &str) -> Self {
            MockTransport {
                response: response.to_string(),
                last_body: RefCell::new(String::new()),
            }
        }
    }
    impl RpcTransport for MockTransport {
        fn post_json(&self, _url: &str, body: &str) -> Result<String> {
            *self.last_body.borrow_mut() = body.to_string();
            Ok(self.response.clone())
        }
    }

    #[test]
    fn builds_jsonrpc_envelope() {
        let req = build_request(7, "getBalance", json!(["addr"]));
        assert_eq!(req["jsonrpc"], "2.0");
        assert_eq!(req["id"], 7);
        assert_eq!(req["method"], "getBalance");
        assert_eq!(req["params"][0], "addr");
    }

    #[test]
    fn client_extracts_result() {
        let t = MockTransport::new(r#"{"jsonrpc":"2.0","id":1,"result":{"value":42}}"#);
        let c = RpcClient::new("https://rpc.example/key-should-not-leak", &t);
        let r = c.call("getX", json!([])).unwrap();
        assert_eq!(r["value"], 42);
        // the method name reached the wire
        assert!(t.last_body.borrow().contains("getX"));
    }

    #[test]
    fn maps_jsonrpc_error() {
        let t = MockTransport::new(r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32602,"message":"bad param"}}"#);
        let c = RpcClient::new("https://rpc", &t);
        let err = c.call("getX", json!([])).unwrap_err();
        assert_eq!(
            err,
            CoreError::RpcError { code: -32602, message: "bad param".into() }
        );
    }

    #[test]
    fn rejects_non_json() {
        assert!(matches!(parse_response("<html>502</html>"), Err(CoreError::Rpc(_))));
    }
}
