//! Schema-constrained Claude Messages requests over HTTP.
//!
//! Construct a [`Claude`], fill a [`Request`], and await [`Claude::structured`].
//! Schemas are sent unchanged; the API enforces the supplied schema.

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};
use std::time::Duration;

use reqwest::StatusCode;
use reqwest::header::RETRY_AFTER;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAXIMUM_RETRIES: u32 = 2;
const MAXIMUM_RETRY_DELAY: Duration = Duration::from_secs(60);

/// Sends schema-constrained requests using a reusable HTTP connection pool.
#[derive(Clone)]
pub struct Claude {
    client: reqwest::Client,
    key: String,
    model: String,
    base_url: String,
    timeout: Duration,
    backoff_base: Duration,
    /// Whether requests ask for server-side fallbacks on a refusal.
    fallbacks: bool,
}

impl Claude {
    /// Creates a client for `https://api.anthropic.com` with a 120-second timeout.
    pub fn new(key: impl Into<String>, model: impl Into<String>) -> Result<Self, ClaudeError> {
        let key = key.into();
        let client = reqwest::Client::builder()
            // The application owns the retry limit, including protocol retries.
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ClaudeError::Network {
                message: redact(&error.to_string(), &key),
            })?;
        let model = model.into();
        Ok(Self {
            client,
            key,
            fallbacks: offers_fallbacks(&model),
            model,
            base_url: "https://api.anthropic.com".into(),
            timeout: Duration::from_secs(120),
            backoff_base: Duration::from_secs(1),
        })
    }

    /// Turns server-side fallbacks on or off, overriding the guess from the
    /// model's name.
    pub fn with_fallbacks(mut self, fallbacks: bool) -> Self {
        self.fallbacks = fallbacks;
        self
    }

    /// Overrides the API origin, primarily for local HTTP tests.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Sets the timeout for each attempt, including reading its response body.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the initial exponential backoff; zero disables backoff and jitter.
    ///
    /// A valid `retry-after` header takes precedence, capped at 60 seconds.
    pub fn with_backoff_base(mut self, backoff_base: Duration) -> Self {
        self.backoff_base = backoff_base;
        self
    }

    /// Requests JSON, preserving refusals and truncated replies as distinct outcomes.
    ///
    /// Retries transient HTTP statuses, timeouts, and connection failures twice.
    /// When the API rejects the request outright (HTTP 400) and it carried
    /// optional settings, effort, fallbacks, or cache markers, which not every
    /// model takes, it is sent once more without them.
    pub async fn structured(&self, request: &Request) -> Result<Reply, ClaudeError> {
        if request.messages.last().map(|message| message.role) != Some(Role::User) {
            return Err(ClaudeError::InvalidRequest {
                message: "the conversation must end with a user message".into(),
            });
        }
        let full = self
            .exchange(&self.request_body(request, false), self.fallbacks)
            .await;
        match full {
            Err((_, Some(StatusCode::BAD_REQUEST))) if self.has_options(request) => self
                .exchange(&self.request_body(request, true), false)
                .await
                .map_err(|(error, _)| error),
            result => result.map_err(|(error, _)| error),
        }
    }

    fn has_options(&self, request: &Request) -> bool {
        let cached = |blocks: &[Block]| blocks.iter().any(|block| block.cache.is_some());
        request.effort.is_some()
            || self.fallbacks
            || cached(&request.system)
            || request
                .messages
                .iter()
                .any(|message| cached(&message.content))
    }

    // Sends one body with retries, returning the error with the HTTP status
    // that caused it, if there was one.
    async fn exchange(
        &self,
        body: &Value,
        fallbacks: bool,
    ) -> Result<Reply, (ClaudeError, Option<StatusCode>)> {
        let mut attempt = 0;
        loop {
            match self.send(body, fallbacks).await {
                Ok(response) => {
                    if response.status == StatusCode::OK {
                        return self
                            .parse_reply(&response.body)
                            .map_err(|error| (error, None));
                    }
                    let error = self.http_error(response.status, &response.body);
                    let retryable = response.status == StatusCode::TOO_MANY_REQUESTS
                        || response.status.is_server_error();
                    if !retryable || attempt == MAXIMUM_RETRIES {
                        return Err((error, Some(response.status)));
                    }
                    tokio::time::sleep(self.retry_delay(attempt, response.retry_after)).await;
                }
                Err(error) => {
                    let retryable = error.is_timeout()
                        || error.is_connect()
                        || error.is_request()
                        || error.is_body();
                    let message = redact(&error.to_string(), &self.key);
                    let failure = if error.is_timeout() {
                        ClaudeError::Timeout { message }
                    } else if error.is_builder() {
                        ClaudeError::InvalidRequest { message }
                    } else {
                        ClaudeError::Network { message }
                    };
                    if !retryable || attempt == MAXIMUM_RETRIES {
                        return Err((failure, None));
                    }
                    tokio::time::sleep(self.retry_delay(attempt, None)).await;
                }
            }
            attempt += 1;
        }
    }

    // The request as the API reads it. A `plain` body leaves out the
    // optional settings.
    fn request_body(&self, request: &Request, plain: bool) -> Value {
        let blocks = |blocks: &[Block]| -> Vec<Value> {
            blocks
                .iter()
                .map(|block| {
                    let mut value = json!({"type": "text", "text": block.text});
                    if let Some(cache) = block.cache.filter(|_| !plain) {
                        value["cache_control"] = json!({"type": "ephemeral", "ttl": cache});
                    }
                    value
                })
                .collect()
        };
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|message| json!({"role": message.role, "content": blocks(&message.content)}))
            .collect();
        let mut body = json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "system": blocks(&request.system),
            "messages": messages,
            "output_config": {
                "format": { "type": "json_schema", "schema": request.schema },
            },
        });
        if !plain {
            if let Some(effort) = request.effort {
                body["output_config"]["effort"] = json!(effort);
            }
            if self.fallbacks {
                body["fallbacks"] = json!("default");
            }
        }
        body
    }

    async fn send(&self, body: &Value, fallbacks: bool) -> Result<HttpReply, reqwest::Error> {
        let mut request = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("content-type", "application/json")
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01");
        if fallbacks {
            request = request.header("anthropic-beta", "server-side-fallback-2026-07-01");
        }
        let mut request = request.timeout(self.timeout).json(body).build()?;
        if let Some(key) = request.headers_mut().get_mut("x-api-key") {
            key.set_sensitive(true);
        }
        let response = self.client.execute(request).await?;
        let status = response.status();
        let retry_after = response
            .headers()
            .get(RETRY_AFTER)
            .and_then(|header| header.to_str().ok())
            .and_then(|seconds| seconds.trim().parse::<u64>().ok())
            .map(|seconds| Duration::from_secs(seconds).min(MAXIMUM_RETRY_DELAY));
        let body = match response.text().await {
            Ok(body) => body,
            // Preserve the HTTP classification, especially errors that must never be retried.
            Err(error) if status != StatusCode::OK => {
                format!("could not read API error body: {error}")
            }
            Err(error) => return Err(error),
        };
        Ok(HttpReply {
            status,
            retry_after,
            body,
        })
    }

    fn retry_delay(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(delay) = retry_after {
            return delay.min(MAXIMUM_RETRY_DELAY);
        }
        let backoff = self.backoff_base.saturating_mul(1 << attempt);
        let jitter_limit = (backoff / 10).min(Duration::from_millis(100));
        let random = RandomState::new().build_hasher().finish();
        let jitter = Duration::from_nanos(random % (jitter_limit.as_nanos() as u64 + 1));
        backoff.saturating_add(jitter).min(MAXIMUM_RETRY_DELAY)
    }

    fn http_error(&self, status: StatusCode, body: &str) -> ClaudeError {
        let envelope = serde_json::from_str::<Value>(body).ok();
        let message = envelope
            .as_ref()
            .and_then(|envelope| envelope.get("error"))
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or(body);
        let message = redact(message, &self.key);
        match status.as_u16() {
            400 | 413 => ClaudeError::InvalidRequest { message },
            401 => ClaudeError::Authentication { message },
            403 => ClaudeError::Permission { message },
            404 => ClaudeError::NotFound { message },
            429 => ClaudeError::RateLimited { message },
            500..=599 => ClaudeError::Overloaded {
                status: status.as_u16(),
                message,
            },
            _ => ClaudeError::Http {
                status: status.as_u16(),
                message,
            },
        }
    }

    fn parse_reply(&self, body: &str) -> Result<Reply, ClaudeError> {
        let response: MessageReply =
            serde_json::from_str(body).map_err(|error| ClaudeError::InvalidResponse {
                message: redact(&error.to_string(), &self.key),
            })?;
        let text: String = response
            .content
            .into_iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            })
            .collect();
        match response.stop_reason.as_str() {
            "end_turn" => {
                let value =
                    serde_json::from_str(&text).map_err(|error| ClaudeError::InvalidJson {
                        text: redact(&text, &self.key),
                        message: redact(&error.to_string(), &self.key),
                    })?;
                Ok(Reply::Json {
                    value,
                    model: response.model,
                    usage: response.usage,
                })
            }
            "refusal" => {
                let details = response.stop_details.unwrap_or_default();
                Ok(Reply::Refusal {
                    category: details.category,
                    explanation: details.explanation,
                    model: response.model,
                    usage: response.usage,
                })
            }
            "max_tokens" => Ok(Reply::CutOff {
                text,
                model: response.model,
                usage: response.usage,
            }),
            reason => Err(ClaudeError::InvalidResponse {
                message: redact(&format!("unexpected stop reason: {reason}"), &self.key),
            }),
        }
    }
}

impl fmt::Debug for Claude {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Claude")
            .field("key", &"[REDACTED]")
            .field("model", &redact(&self.model, &self.key))
            .field("base_url", &redact(&self.base_url, &self.key))
            .field("timeout", &self.timeout)
            .field("backoff_base", &self.backoff_base)
            .finish_non_exhaustive()
    }
}

/// Supplies the prompts, schema, and generation limits for a structured reply.
#[derive(Clone, Debug)]
pub struct Request {
    /// Instructions that apply to the whole conversation.
    pub system: Vec<Block>,
    /// Conversation history ending with a user message; assistant prefill is forbidden.
    pub messages: Vec<Message>,
    /// JSON Schema with `additionalProperties: false` and `required` on every object.
    /// Numeric and string bounds (`minimum`, `maximum`, `minLength`, `maxLength`) are unsupported.
    pub schema: Value,
    /// Maximum number of generated tokens, including adaptive thinking.
    pub max_tokens: u32,
    /// Amount of reasoning effort requested from the model, or `None` for
    /// the model's default.
    pub effort: Option<Effort>,
}

/// A text turn in the conversation sent to Claude.
#[derive(Clone, Debug)]
pub struct Message {
    /// Participant who supplied this turn.
    pub role: Role,
    pub content: Vec<Block>,
}

impl Message {
    /// A turn of plain text, not cached.
    pub fn text(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![Block::new(text)],
        }
    }
}

/// A run of prompt text. A block with `cache` set ends a prefix the API
/// keeps, so later requests that begin the same way read it at a tenth of
/// the input price.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub text: String,
    pub cache: Option<Cache>,
}

impl Block {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            cache: None,
        }
    }

    pub fn cached(text: impl Into<String>, cache: Cache) -> Self {
        Self {
            text: text.into(),
            cache: Some(cache),
        }
    }
}

/// How long a cached prompt prefix lives after it is last used. Writing an
/// hour-long entry costs twice the input price, a five-minute one 1.25 times.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Cache {
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
}

/// Identifies the participant for a conversation turn.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A prompt or follow-up from the caller.
    User,
    /// A previous response from the assistant.
    Assistant,
}

/// Controls the model's adaptive reasoning effort.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    /// Requests low reasoning effort.
    Low,
    /// Requests medium reasoning effort.
    Medium,
    /// Requests high reasoning effort.
    High,
    /// Requests extra-high reasoning effort.
    XHigh,
    /// Requests maximum reasoning effort.
    Max,
}

impl Effort {
    pub const ALL: [Self; 5] = [Self::Low, Self::Medium, Self::High, Self::XHigh, Self::Max];

    pub fn name(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::XHigh => "xhigh",
            Self::Max => "max",
        }
    }
}

/// Whether a model takes server-side fallbacks, by its name: the Haiku
/// models don't.
fn offers_fallbacks(model: &str) -> bool {
    !model.contains("haiku")
}

/// Token accounting reported by the serving model.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct Usage {
    /// Input tokens processed by the model.
    pub input_tokens: u64,
    /// Output tokens generated by the model.
    pub output_tokens: u64,
    /// Input tokens written into the prompt cache, when reported.
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    /// Input tokens read from the prompt cache, when reported.
    #[serde(default)]
    pub cache_read_input_tokens: u64,
}

/// The outcome of a completed Messages API call.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    /// A complete reply parsed as JSON.
    Json {
        /// The concatenated text blocks parsed with `serde_json`.
        value: Value,
        /// Actual serving model, which may be a fallback model.
        model: String,
        /// Token accounting for this call.
        usage: Usage,
    },
    /// A request declined even after server-side fallbacks.
    Refusal {
        /// Classifier category, when supplied.
        category: Option<String>,
        /// Human-readable refusal explanation, when supplied.
        explanation: Option<String>,
        /// Actual serving model.
        model: String,
        /// Token accounting for this call.
        usage: Usage,
    },
    /// A reply stopped at the output token limit, without attempting JSON parsing.
    CutOff {
        /// Concatenated text blocks retained for inspection.
        text: String,
        /// Actual serving model.
        model: String,
        /// Token accounting for this call.
        usage: Usage,
    },
}

/// A failed request or an unusable response, with credentials removed from diagnostics.
#[derive(Debug)]
pub enum ClaudeError {
    /// HTTP 401: the API rejected the credentials.
    Authentication { message: String },
    /// HTTP 403: the credentials lack permission.
    Permission { message: String },
    /// HTTP 404: the endpoint or model does not exist.
    NotFound { message: String },
    /// Invalid local input or HTTP 400/413, including the API's explanation.
    InvalidRequest { message: String },
    /// HTTP 429 persisted through the retry limit.
    RateLimited { message: String },
    /// HTTP 5xx, including 529, persisted through the retry limit.
    Overloaded { status: u16, message: String },
    /// The HTTP client could not complete the request.
    Network { message: String },
    /// The request timed out after exhausting retries.
    Timeout { message: String },
    /// An `end_turn` reply contained invalid JSON; text is retained for debugging.
    InvalidJson { text: String, message: String },
    /// The response envelope or stop reason was not supported.
    InvalidResponse { message: String },
    /// An HTTP status outside the documented API errors was received.
    Http { status: u16, message: String },
}

impl fmt::Display for ClaudeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Authentication { message } => {
                write!(formatter, "Claude authentication: {message}")
            }
            Self::Permission { message } => write!(formatter, "Claude permission: {message}"),
            Self::NotFound { message } => write!(formatter, "Claude not found: {message}"),
            Self::InvalidRequest { message } => {
                write!(formatter, "Claude invalid request: {message}")
            }
            Self::RateLimited { message } => {
                write!(formatter, "Claude rate limit after retries: {message}")
            }
            Self::Overloaded { status, message } => write!(
                formatter,
                "Claude overload after retries (HTTP {status}): {message}"
            ),
            Self::Network { message } => write!(formatter, "Claude network failure: {message}"),
            Self::Timeout { message } => {
                write!(formatter, "Claude timeout after retries: {message}")
            }
            Self::InvalidJson { message, .. } => {
                write!(formatter, "Claude reply is not JSON: {message}")
            }
            Self::InvalidResponse { message } => {
                write!(formatter, "Claude invalid response: {message}")
            }
            Self::Http { status, message } => write!(formatter, "Claude HTTP {status}: {message}"),
        }
    }
}

impl std::error::Error for ClaudeError {}

fn redact(text: &str, key: &str) -> String {
    if key.is_empty() {
        text.to_owned()
    } else {
        text.replace(key, "[REDACTED]")
    }
}

struct HttpReply {
    status: StatusCode,
    retry_after: Option<Duration>,
    body: String,
}

#[derive(Deserialize)]
struct MessageReply {
    model: String,
    content: Vec<ContentBlock>,
    stop_reason: String,
    stop_details: Option<StopDetails>,
    usage: Usage,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Default, Deserialize)]
struct StopDetails {
    category: Option<String>,
    explanation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::task::JoinHandle;

    const KEY: &str = "test-secret-api-key";

    struct CapturedRequest {
        request_line: String,
        headers: HashMap<String, String>,
        body: Value,
    }

    struct CannedResponse {
        status: u16,
        headers: Vec<(&'static str, String)>,
        body: String,
        delay: Duration,
        body_delay: Duration,
        disconnect: bool,
    }

    impl CannedResponse {
        fn json(status: u16, body: Value) -> Self {
            Self {
                status,
                headers: Vec::new(),
                body: body.to_string(),
                delay: Duration::ZERO,
                body_delay: Duration::ZERO,
                disconnect: false,
            }
        }

        fn error(status: u16, message: &str) -> Self {
            Self::json(
                status,
                json!({"type": "error", "error": {"type": "api_error", "message": message}}),
            )
        }

        fn retry_after(mut self, seconds: &str) -> Self {
            self.headers.push(("retry-after", seconds.to_owned()));
            self
        }

        fn delayed(mut self, delay: Duration) -> Self {
            self.delay = delay;
            self
        }

        fn disconnected() -> Self {
            Self {
                disconnect: true,
                ..Self::error(500, "")
            }
        }
    }

    struct Server {
        base_url: String,
        requests: Arc<Mutex<Vec<CapturedRequest>>>,
        task: JoinHandle<()>,
    }

    impl Server {
        async fn start(responses: Vec<CannedResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured = Arc::clone(&requests);
            let task = tokio::spawn(async move {
                let mut responses = responses.into_iter();
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let Ok(request) = read_request(&mut stream).await else {
                        continue;
                    };
                    captured.lock().unwrap().push(request);
                    // Keep accepting after the fixtures run out so an extra retry is observable.
                    let response = responses
                        .next()
                        .unwrap_or_else(|| CannedResponse::error(500, "unexpected extra request"));
                    if response.disconnect {
                        continue;
                    }
                    tokio::time::sleep(response.delay).await;
                    let mut headers = format!(
                        "HTTP/1.1 {} Test response\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
                        response.status,
                        response.body.len()
                    );
                    for (name, value) in response.headers {
                        headers.push_str(&format!("{name}: {value}\r\n"));
                    }
                    headers.push_str("\r\n");
                    let _ = stream.write_all(headers.as_bytes()).await;
                    tokio::time::sleep(response.body_delay).await;
                    let _ = stream.write_all(response.body.as_bytes()).await;
                    let _ = stream.shutdown().await;
                }
            });
            Self {
                base_url,
                requests,
                task,
            }
        }

        fn client(&self) -> Claude {
            Claude::new(KEY, "claude-opus-5-5")
                .unwrap()
                .with_base_url(format!("{}/", self.base_url))
                .with_timeout(Duration::from_secs(2))
                .with_backoff_base(Duration::ZERO)
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn read_request(stream: &mut TcpStream) -> io::Result<CapturedRequest> {
        let mut bytes = Vec::new();
        let mut buffer = [0; 1024];
        let header_end = loop {
            let count = stream.read(&mut buffer).await?;
            if count == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let head = String::from_utf8_lossy(&bytes[..header_end]);
        let mut lines = head.lines();
        let request_line = lines.next().unwrap().to_owned();
        let headers: HashMap<String, String> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let content_length = headers["content-length"].parse::<usize>().unwrap();
        while bytes.len() < header_end + content_length {
            let count = stream.read(&mut buffer).await?;
            if count == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        Ok(CapturedRequest {
            request_line,
            headers,
            body: serde_json::from_slice(&bytes[header_end..header_end + content_length]).unwrap(),
        })
    }

    fn request() -> Request {
        Request {
            system: vec![Block::new("Reply with a thought.")],
            messages: vec![
                Message::text(Role::User, "Hello"),
                Message::text(Role::Assistant, "Hello again"),
                Message::text(Role::User, "What are you thinking?"),
            ],
            schema: json!({
                "type": "object",
                "properties": {"thought": {"type": "string"}},
                "required": ["thought"],
                "additionalProperties": false,
            }),
            max_tokens: 4000,
            effort: Some(Effort::Low),
        }
    }

    fn message_reply(content: Value, stop_reason: &str) -> Value {
        json!({
            "id": "msg_test", "type": "message", "role": "assistant",
            "model": "claude-serving-fallback",
            "content": content,
            "stop_reason": stop_reason,
            "stop_details": null,
            "usage": {
                "input_tokens": 812, "output_tokens": 95,
                "cache_creation_input_tokens": 12, "cache_read_input_tokens": 34,
            },
        })
    }

    fn successful_response() -> CannedResponse {
        CannedResponse::json(
            200,
            message_reply(
                json!([
                    {"type": "thinking", "thinking": "", "signature": "signature"},
                    {"type": "text", "text": "{\"thought\":\"hello\"}"},
                ]),
                "end_turn",
            ),
        )
    }

    #[test]
    fn the_request_body_matches_the_messages_api_contract_exactly() {
        let client = Claude::new(KEY, "claude-opus-5-5").unwrap();
        let text = |text: &str| json!([{"type": "text", "text": text}]);
        assert_eq!(
            client.request_body(&request(), false),
            json!({
                "model": "claude-opus-5-5",
                "max_tokens": 4000,
                "system": text("Reply with a thought."),
                "messages": [
                    {"role": "user", "content": text("Hello")},
                    {"role": "assistant", "content": text("Hello again")},
                    {"role": "user", "content": text("What are you thinking?")},
                ],
                "output_config": {
                    "effort": "low",
                    "format": {
                        "type": "json_schema",
                        "schema": {
                            "type": "object",
                            "properties": {"thought": {"type": "string"}},
                            "required": ["thought"],
                            "additionalProperties": false,
                        },
                    },
                },
                "fallbacks": "default",
            })
        );
        assert_eq!(client.timeout, Duration::from_secs(120));
        assert_eq!(client.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn every_effort_serializes_to_its_documented_string() {
        for (effort, name) in [
            (Effort::Low, "low"),
            (Effort::Medium, "medium"),
            (Effort::High, "high"),
            (Effort::XHigh, "xhigh"),
            (Effort::Max, "max"),
        ] {
            assert_eq!(serde_json::to_value(effort).unwrap(), json!(name));
        }
    }

    #[tokio::test]
    async fn the_request_sends_the_endpoint_body_and_all_four_headers() {
        let server = Server::start(vec![successful_response()]).await;
        let client = server.client();
        client.structured(&request()).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let captured = &requests[0];
        assert_eq!(captured.request_line, "POST /v1/messages HTTP/1.1");
        assert_eq!(captured.headers["content-type"], "application/json");
        assert_eq!(captured.headers["x-api-key"], KEY);
        assert_eq!(captured.headers["anthropic-version"], "2023-06-01");
        assert_eq!(
            captured.headers["anthropic-beta"],
            "server-side-fallback-2026-07-01"
        );
        assert_eq!(captured.body, client.request_body(&request(), false));
    }

    #[test]
    fn cached_blocks_mark_the_end_of_a_prefix_with_their_lifetime() {
        let client = Claude::new(KEY, "claude-opus-5-5").unwrap();
        let mut cached = request();
        cached.system = vec![
            Block::new("Persona."),
            Block::cached("Memories.", Cache::OneHour),
        ];
        cached.messages[2]
            .content
            .insert(0, Block::cached("History.", Cache::FiveMinutes));
        let body = client.request_body(&cached, false);
        assert_eq!(
            body["system"],
            json!([
                {"type": "text", "text": "Persona."},
                {"type": "text", "text": "Memories.",
                    "cache_control": {"type": "ephemeral", "ttl": "1h"}},
            ])
        );
        assert_eq!(
            body["messages"][2]["content"][0]["cache_control"],
            json!({"type": "ephemeral", "ttl": "5m"})
        );
        let plain = client.request_body(&cached, true);
        assert!(!plain.to_string().contains("cache_control"));
    }

    #[tokio::test]
    async fn haiku_models_ask_for_no_fallbacks() {
        let server = Server::start(vec![successful_response()]).await;
        let client = server.client();
        let client = Claude {
            model: "claude-haiku-5-5".into(),
            fallbacks: offers_fallbacks("claude-haiku-5-5"),
            ..client
        };
        client.structured(&request()).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert!(!requests[0].headers.contains_key("anthropic-beta"));
        assert!(requests[0].body.get("fallbacks").is_none());
        assert_eq!(requests[0].body["output_config"]["effort"], "low");
    }

    #[tokio::test]
    async fn a_rejected_request_is_tried_once_more_without_optional_settings() {
        let server = Server::start(vec![
            CannedResponse::error(400, "effort is not supported"),
            successful_response(),
        ])
        .await;
        let client = server.client();
        let mut cached = request();
        cached.system[0].cache = Some(Cache::OneHour);
        client.structured(&cached).await.unwrap();
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        let retried = &requests[1];
        assert!(retried.body["output_config"].get("effort").is_none());
        assert!(retried.body.get("fallbacks").is_none());
        assert!(!retried.body.to_string().contains("cache_control"));
        assert!(!retried.headers.contains_key("anthropic-beta"));
        assert_eq!(
            retried.body["output_config"]["format"]["type"],
            "json_schema"
        );
    }

    #[tokio::test]
    async fn thinking_blocks_are_skipped_and_json_includes_the_serving_model_and_usage() {
        let server = Server::start(vec![successful_response()]).await;
        assert_eq!(
            server.client().structured(&request()).await.unwrap(),
            Reply::Json {
                value: json!({"thought": "hello"}),
                model: "claude-serving-fallback".into(),
                usage: Usage {
                    input_tokens: 812,
                    output_tokens: 95,
                    cache_creation_input_tokens: 12,
                    cache_read_input_tokens: 34,
                },
            }
        );
    }

    #[tokio::test]
    async fn unknown_blocks_are_skipped_and_text_blocks_are_concatenated_in_order() {
        let response = message_reply(
            json!([
                {"type": "redacted_thinking", "data": "hidden"},
                {"type": "text", "text": "{\"thought\":"},
                {"type": "fallback", "model": "other-model"},
                {"type": "future_block", "text": {"an": "unknown shape"}},
                {"type": "text", "text": "\"hello\"}"},
            ]),
            "end_turn",
        );
        let server = Server::start(vec![CannedResponse::json(200, response)]).await;
        assert!(matches!(
            server.client().structured(&request()).await.unwrap(),
            Reply::Json { value, .. } if value == json!({"thought": "hello"})
        ));
    }

    #[tokio::test]
    async fn a_refusal_returns_its_category_and_explanation_without_parsing_text() {
        let mut response = message_reply(
            json!([{"type": "text", "text": "I cannot comply."}]),
            "refusal",
        );
        response["stop_details"] = json!({
            "type": "refusal", "category": "cyber", "explanation": "Request declined",
        });
        let server = Server::start(vec![CannedResponse::json(200, response)]).await;
        assert!(matches!(
            server.client().structured(&request()).await.unwrap(),
            Reply::Refusal { category: Some(category), explanation: Some(explanation), .. }
                if category == "cyber" && explanation == "Request declined"
        ));
    }

    #[tokio::test]
    async fn a_refusal_allows_missing_details_and_a_null_category() {
        let mut without_details = message_reply(json!([]), "refusal");
        without_details
            .as_object_mut()
            .unwrap()
            .remove("stop_details");
        let mut null_category = message_reply(json!([]), "refusal");
        null_category["stop_details"] = json!({"type": "refusal", "category": null});
        let server = Server::start(vec![
            CannedResponse::json(200, without_details),
            CannedResponse::json(200, null_category),
        ])
        .await;
        for _ in 0..2 {
            assert!(matches!(
                server.client().structured(&request()).await.unwrap(),
                Reply::Refusal {
                    category: None,
                    explanation: None,
                    ..
                }
            ));
        }
    }

    #[tokio::test]
    async fn the_token_limit_returns_a_cut_off_reply_without_parsing_partial_json() {
        let response = message_reply(
            json!([{"type": "text", "text": "{\"thought\":"}]),
            "max_tokens",
        );
        let server = Server::start(vec![CannedResponse::json(200, response)]).await;
        assert!(matches!(
            server.client().structured(&request()).await.unwrap(),
            Reply::CutOff { text, .. } if text == "{\"thought\":"
        ));
    }

    #[tokio::test]
    async fn an_overloaded_response_is_retried_and_then_succeeds() {
        let server = Server::start(vec![
            CannedResponse::error(529, "overloaded"),
            successful_response(),
        ])
        .await;
        assert!(matches!(
            server.client().structured(&request()).await.unwrap(),
            Reply::Json { .. }
        ));
        assert_eq!(server.request_count(), 2);
    }

    #[tokio::test]
    async fn a_rate_limit_honors_zero_retry_after_instead_of_the_backoff() {
        let server = Server::start(vec![
            CannedResponse::error(429, "rate limited").retry_after("0"),
            successful_response(),
        ])
        .await;
        let client = server.client().with_backoff_base(Duration::from_secs(60));
        tokio::time::timeout(Duration::from_secs(1), client.structured(&request()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(server.request_count(), 2);
    }

    #[tokio::test]
    async fn internal_errors_and_other_server_errors_are_retried() {
        let server = Server::start(vec![
            CannedResponse::error(500, "internal error"),
            CannedResponse::error(502, "bad gateway"),
            successful_response(),
        ])
        .await;
        server.client().structured(&request()).await.unwrap();
        assert_eq!(server.request_count(), 3);
    }

    #[tokio::test]
    async fn authentication_errors_fail_after_exactly_one_request() {
        let server = Server::start(vec![CannedResponse::error(401, "bad credentials")]).await;
        assert!(matches!(
            server.client().structured(&request()).await.unwrap_err(),
            ClaudeError::Authentication { message } if message == "bad credentials"
        ));
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn authentication_errors_are_never_retried_even_when_the_error_body_times_out() {
        let mut response = CannedResponse::error(401, "bad credentials");
        response.body_delay = Duration::from_secs(1);
        let server = Server::start(vec![response]).await;
        let client = server.client().with_timeout(Duration::from_millis(100));
        assert!(matches!(
            client.structured(&request()).await.unwrap_err(),
            ClaudeError::Authentication { .. }
        ));
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn invalid_requests_surface_the_api_error_message_after_the_plain_attempt() {
        let server = Server::start(vec![
            CannedResponse::error(400, "Unsupported schema keyword"),
            CannedResponse::error(400, "Unsupported schema keyword"),
        ])
        .await;
        let error = server.client().structured(&request()).await.unwrap_err();
        assert!(matches!(error, ClaudeError::InvalidRequest { .. }));
        assert!(error.to_string().contains("Unsupported schema keyword"));
        assert_eq!(server.request_count(), 2);
    }

    #[tokio::test]
    async fn a_plain_request_that_is_rejected_is_not_sent_again() {
        let server = Server::start(vec![CannedResponse::error(400, "bad schema")]).await;
        let client = server.client().with_fallbacks(false);
        let mut plain = request();
        plain.effort = None;
        assert!(client.structured(&plain).await.is_err());
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn permission_not_found_and_too_large_errors_are_distinct_and_never_retried() {
        for status in [403, 404, 413] {
            let server =
                Server::start(vec![CannedResponse::error(status, "request rejected")]).await;
            let error = server.client().structured(&request()).await.unwrap_err();
            assert!(matches!(
                (status, error),
                (403, ClaudeError::Permission { .. })
                    | (404, ClaudeError::NotFound { .. })
                    | (413, ClaudeError::InvalidRequest { .. })
            ));
            assert_eq!(server.request_count(), 1);
        }
    }

    #[tokio::test]
    async fn overload_and_rate_limit_retries_stop_after_three_total_attempts() {
        for status in [429, 500, 529] {
            let server = Server::start(
                (0..3)
                    .map(|_| CannedResponse::error(status, "still busy").retry_after("0"))
                    .collect(),
            )
            .await;
            let error = server.client().structured(&request()).await.unwrap_err();
            match error {
                ClaudeError::RateLimited { message } => {
                    assert_eq!(status, 429);
                    assert_eq!(message, "still busy");
                }
                ClaudeError::Overloaded {
                    status: actual_status,
                    message,
                } => {
                    assert_eq!(actual_status, status);
                    assert_eq!(message, "still busy");
                }
                other => panic!("unexpected error: {other}"),
            }
            assert_eq!(server.request_count(), 3);
        }
    }

    #[tokio::test]
    async fn non_json_text_is_preserved_in_the_parse_error_without_retrying() {
        let response = message_reply(json!([{"type": "text", "text": "not JSON"}]), "end_turn");
        let server = Server::start(vec![CannedResponse::json(200, response)]).await;
        let error = server.client().structured(&request()).await.unwrap_err();
        assert!(matches!(
            error,
            ClaudeError::InvalidJson { text, message } if text == "not JSON" && !message.is_empty()
        ));
        assert_eq!(server.request_count(), 1);
    }

    #[tokio::test]
    async fn an_empty_conversation_or_assistant_prefill_is_rejected_before_sending() {
        let server = Server::start(Vec::new()).await;
        let mut invalid = request();
        invalid.messages.push(Message::text(Role::Assistant, "{"));
        assert!(matches!(
            server.client().structured(&invalid).await.unwrap_err(),
            ClaudeError::InvalidRequest { .. }
        ));
        invalid.messages.clear();
        assert!(matches!(
            server.client().structured(&invalid).await.unwrap_err(),
            ClaudeError::InvalidRequest { .. }
        ));
        assert_eq!(server.request_count(), 0);
    }

    #[tokio::test]
    async fn a_timed_out_request_is_retried_and_then_succeeds() {
        let server = Server::start(vec![
            successful_response().delayed(Duration::from_millis(150)),
            successful_response(),
        ])
        .await;
        let client = server.client().with_timeout(Duration::from_millis(100));
        assert!(matches!(
            client.structured(&request()).await.unwrap(),
            Reply::Json { .. }
        ));
        assert_eq!(server.request_count(), 2);
    }

    #[tokio::test]
    async fn timeouts_exhaust_the_retry_limit_and_return_a_timeout_error() {
        let server = Server::start(
            (0..3)
                .map(|_| successful_response().delayed(Duration::from_millis(120)))
                .collect(),
        )
        .await;
        let client = server.client().with_timeout(Duration::from_millis(100));
        assert!(matches!(
            client.structured(&request()).await.unwrap_err(),
            ClaudeError::Timeout { .. }
        ));
        assert_eq!(server.request_count(), 3);
    }

    #[tokio::test]
    async fn connection_failures_return_a_network_error_after_retries() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let client = Claude::new(KEY, "claude-opus-5-5")
            .unwrap()
            .with_base_url(format!("http://{address}"))
            .with_timeout(Duration::from_millis(100))
            .with_backoff_base(Duration::ZERO);
        assert!(matches!(
            client.structured(&request()).await.unwrap_err(),
            ClaudeError::Network { .. }
        ));
    }

    #[tokio::test]
    async fn a_connection_closed_before_the_reply_is_retried_and_then_succeeds() {
        let server =
            Server::start(vec![CannedResponse::disconnected(), successful_response()]).await;
        server.client().structured(&request()).await.unwrap();
        assert_eq!(server.request_count(), 2);
    }

    #[tokio::test]
    async fn connection_failures_stop_after_three_total_attempts() {
        let server = Server::start((0..3).map(|_| CannedResponse::disconnected()).collect()).await;
        assert!(matches!(
            server.client().structured(&request()).await.unwrap_err(),
            ClaudeError::Network { .. }
        ));
        assert_eq!(server.request_count(), 3);
    }

    #[test]
    fn retry_delays_grow_exponentially_with_small_jitter_and_are_capped() {
        let client = Claude::new(KEY, "claude-opus-5-5").unwrap();
        for attempt in 0..2 {
            let delay = client.retry_delay(attempt, None);
            let expected = Duration::from_secs(1 << attempt);
            assert!(delay >= expected && delay <= expected + Duration::from_millis(100));
        }
        assert_eq!(
            client.retry_delay(0, Some(Duration::from_secs(90))),
            Duration::from_secs(60)
        );
        assert_eq!(
            client.retry_delay(0, Some(Duration::from_secs(7))),
            Duration::from_secs(7)
        );
        assert_eq!(client.retry_delay(0, Some(Duration::ZERO)), Duration::ZERO);
    }

    #[test]
    fn malformed_envelopes_and_unexpected_stop_reasons_return_protocol_errors() {
        let client = Claude::new(KEY, "claude-opus-5-5").unwrap();
        assert!(matches!(
            client.parse_reply("not a response").unwrap_err(),
            ClaudeError::InvalidResponse { .. }
        ));
        let response = message_reply(json!([]), "future_stop_reason");
        assert!(matches!(
            client.parse_reply(&response.to_string()).unwrap_err(),
            ClaudeError::InvalidResponse { .. }
        ));
    }

    #[tokio::test]
    async fn debug_and_error_diagnostics_never_include_the_api_key() {
        let client = Claude::new(KEY, format!("model-{KEY}"))
            .unwrap()
            .with_base_url(format!("not a URL {KEY}"));
        assert!(!format!("{client:?}").contains(KEY));
        assert!(format!("{client:?}").contains("[REDACTED]"));
        let network_error = client.structured(&request()).await.unwrap_err();
        let response = message_reply(json!([{"type": "text", "text": KEY}]), "end_turn");
        let parse_error = client.parse_reply(&response.to_string()).unwrap_err();
        let api_error = client.http_error(
            StatusCode::BAD_REQUEST,
            &json!({"error": {"message": format!("invalid API key: {KEY}")}}).to_string(),
        );
        for error in [network_error, parse_error, api_error] {
            assert!(!error.to_string().contains(KEY));
            assert!(!format!("{error:?}").contains(KEY));
            assert!(std::error::Error::source(&error).is_none());
        }
    }
}
