//! Typed text questions and an asynchronous client for TypeSafe's System One API.

use std::collections::BTreeMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{CONTENT_TYPE, HeaderMap, RETRY_AFTER};
use reqwest::{Client, StatusCode};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

const MAXIMUM_RETRIES: u32 = 3;

/// A yes/no question, answered with the probability of yes.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename = "noul")]
pub struct Noul {
    /// The question or statement, as text, an object, an array, or null.
    pub instructions: Value,
    /// Optional guidance defining yes and no.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<NoulCriteria>,
}

impl Noul {
    /// Creates a yes/no question without additional criteria.
    pub fn new(instructions: impl Into<Value>) -> Self {
        Self {
            instructions: instructions.into(),
            criteria: None,
        }
    }

    /// Adds descriptions of what counts as yes and no.
    pub fn with_criteria(mut self, criteria: NoulCriteria) -> Self {
        self.criteria = Some(criteria);
        self
    }
}

/// Optional descriptions clarifying the two outcomes of a Noul question.
#[derive(Clone, Debug, Default, Serialize)]
pub struct NoulCriteria {
    /// Describes a yes answer; serialized under the key `true`.
    #[serde(rename = "true", skip_serializing_if = "Option::is_none")]
    pub when_true: Option<Value>,
    /// Describes a no answer; serialized under the key `false`.
    #[serde(rename = "false", skip_serializing_if = "Option::is_none")]
    pub when_false: Option<Value>,
}

impl NoulCriteria {
    /// Defines both outcomes using text or structured JSON descriptions.
    pub fn new(when_true: impl Into<Value>, when_false: impl Into<Value>) -> Self {
        Self {
            when_true: Some(when_true.into()),
            when_false: Some(when_false.into()),
        }
    }
}

/// A question selecting one of 1 to 255 named options.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename = "choice")]
pub struct Choice {
    /// What to decide, as text, an object, an array, or null.
    pub instructions: Value,
    /// Option names and descriptions, serialized as an object in this order.
    #[serde(serialize_with = "serialize_choice_criteria")]
    pub criteria: Vec<(String, Value)>,
}

impl Choice {
    /// Creates a choice while retaining the iterator's option order.
    ///
    /// Descriptions may be text, objects, arrays, or null for a name alone.
    pub fn new<Name, Description>(
        instructions: impl Into<Value>,
        criteria: impl IntoIterator<Item = (Name, Description)>,
    ) -> Self
    where
        Name: Into<String>,
        Description: Into<Value>,
    {
        Self {
            instructions: instructions.into(),
            criteria: criteria
                .into_iter()
                .map(|(name, description)| (name.into(), description.into()))
                .collect(),
        }
    }
}

fn serialize_choice_criteria<Serialization>(
    criteria: &[(String, Value)],
    serializer: Serialization,
) -> Result<Serialization::Ok, Serialization::Error>
where
    Serialization: Serializer,
{
    // Going through serde_json::Value would sort these semantically ordered options.
    let mut entries = serializer.serialize_map(Some(criteria.len()))?;
    for (name, description) in criteria {
        entries.serialize_entry(name, description)?;
    }
    entries.end()
}

/// A rating question with 1 to 10 ordered level descriptions, starting at zero.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename = "score")]
pub struct Score {
    /// What to rate, as text, an object, an array, or null.
    pub instructions: Value,
    /// Level descriptions as text, objects, or arrays, from lowest to highest.
    pub criteria: Vec<Value>,
}

impl Score {
    /// Creates a rating question, preserving the order of its levels.
    pub fn new<Description>(
        instructions: impl Into<Value>,
        criteria: impl IntoIterator<Item = Description>,
    ) -> Self
    where
        Description: Into<Value>,
    {
        Self {
            instructions: instructions.into(),
            criteria: criteria.into_iter().map(Into::into).collect(),
        }
    }
}

/// A supported System One question to include in a named collection.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Question {
    /// Asks whether a statement is true.
    Noul(Noul),
    /// Selects one named option.
    Choice(Choice),
    /// Rates content against ordered levels.
    Score(Score),
}

impl From<Noul> for Question {
    fn from(question: Noul) -> Self {
        Self::Noul(question)
    }
}

impl From<Choice> for Question {
    fn from(question: Choice) -> Self {
        Self::Choice(question)
    }
}

impl From<Score> for Question {
    fn from(question: Score) -> Self {
        Self::Score(question)
    }
}

/// Named questions evaluated together against the same state.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(transparent)]
pub struct Questions(BTreeMap<String, Question>);

impl Questions {
    /// Creates an empty collection; add at least one question before asking.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a question, returning any previous question with that name.
    pub fn insert(
        &mut self,
        name: impl Into<String>,
        question: impl Into<Question>,
    ) -> Option<Question> {
        self.0.insert(name.into(), question.into())
    }

    /// Adds or replaces a question for chained construction of a collection.
    pub fn with(mut self, name: impl Into<String>, question: impl Into<Question>) -> Self {
        self.insert(name, question);
        self
    }
}

/// The probability that a Noul question's answer is yes or true.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NoulAnswer {
    /// Probability from zero to one; values near 0.5 indicate uncertainty.
    pub noul: f64,
}

/// A selected option together with its confidence and full distribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceAnswer {
    /// The name of the selected option.
    pub choice: String,
    /// The API's confidence in the selection, from zero to one.
    pub confidence: f64,
    /// Probabilities keyed by option name.
    pub probabilities: BTreeMap<String, f64>,
}

/// An expected score together with its rubric and probability distribution.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreAnswer {
    /// Probability-weighted mean of the zero-based level numbers.
    pub score: f64,
    /// The API's confidence in the score, from zero to one.
    pub confidence: f64,
    /// Original level descriptions keyed by level number as a string.
    pub legend: BTreeMap<String, Value>,
    /// Probabilities keyed by level number as a string.
    pub probabilities: BTreeMap<String, f64>,
}

/// A response value whose tag identifies the question type.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Answer {
    /// A yes/no probability.
    Noul(NoulAnswer),
    /// A named selection.
    Choice(ChoiceAnswer),
    /// A rating against ordered levels.
    Score(ScoreAnswer),
}

/// Token counts reported for an evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens used to evaluate the request.
    pub input_tokens: u64,
    /// Output tokens used for the answers.
    pub output_tokens: u64,
}

/// Parsed System One response, including the resolved model and token usage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Answers {
    /// The model that answered, which can differ from the requested alias.
    pub model: String,
    /// Answers under the names supplied in the request.
    pub answers: BTreeMap<String, Answer>,
    /// Token counts for this evaluation.
    pub usage: Usage,
}

impl Answers {
    /// Looks up a yes/no answer, returning none for missing names or other types.
    pub fn noul(&self, name: &str) -> Option<&NoulAnswer> {
        match self.answers.get(name) {
            Some(Answer::Noul(answer)) => Some(answer),
            _ => None,
        }
    }

    /// Looks up a choice answer, returning none for missing names or other types.
    pub fn choice(&self, name: &str) -> Option<&ChoiceAnswer> {
        match self.answers.get(name) {
            Some(Answer::Choice(answer)) => Some(answer),
            _ => None,
        }
    }

    /// Looks up a score answer, returning none for missing names or other types.
    pub fn score(&self, name: &str) -> Option<&ScoreAnswer> {
        match self.answers.get(name) {
            Some(Answer::Score(answer)) => Some(answer),
            _ => None,
        }
    }
}

/// An evaluation failure, with credentials removed from diagnostic messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JevError {
    /// The API rejected the credentials with HTTP 401.
    Unauthorized,
    /// The API rejected the request without retrying it.
    Rejected {
        /// The HTTP status, including 400, 413, 415, or 422.
        status: u16,
        /// The response's message or validation detail.
        message: String,
    },
    /// HTTP 429 persisted through the three retries.
    RateLimited {
        /// The final response's explanation.
        message: String,
    },
    /// HTTP 529 or another server error persisted through the three retries.
    Overloaded {
        /// The last server error's HTTP status.
        status: u16,
        /// The final response's explanation.
        message: String,
    },
    /// A request could not be serialized before sending.
    InvalidRequest {
        /// The serialization failure.
        message: String,
    },
    /// A transport or client configuration error prevented the request.
    Network {
        /// A diagnostic with the API key and request URL removed.
        message: String,
    },
    /// The final attempt exceeded the configured request timeout.
    Timeout,
    /// A successful response could not be decoded into supported answers.
    Decode {
        /// The JSON or response-schema failure.
        message: String,
    },
}

impl fmt::Display for JevError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unauthorized => formatter.write_str("Jev rejected the API key (HTTP 401)"),
            Self::Rejected { status, message } => {
                write!(
                    formatter,
                    "Jev rejected the request (HTTP {status}): {message}"
                )
            }
            Self::RateLimited { message } => {
                write!(
                    formatter,
                    "Jev rate limit persisted after three retries: {message}"
                )
            }
            Self::Overloaded { status, message } => write!(
                formatter,
                "Jev server error persisted after three retries (HTTP {status}): {message}"
            ),
            Self::InvalidRequest { message } => write!(formatter, "Invalid Jev request: {message}"),
            Self::Network { message } => write!(formatter, "Jev network failure: {message}"),
            Self::Timeout => formatter.write_str("Jev request timed out"),
            Self::Decode { message } => {
                write!(formatter, "Could not decode Jev response: {message}")
            }
        }
    }
}

impl std::error::Error for JevError {}

/// A reusable asynchronous System One client with bounded transient retries.
#[derive(Clone)]
pub struct Jev {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    timeout: Duration,
    backoff_base: Duration,
}

impl fmt::Debug for Jev {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Jev")
            .field("client", &self.client)
            .field("api_key", &"[REDACTED]")
            .field("model", &redact(&self.model, &self.api_key))
            .field("base_url", &redact(&self.base_url, &self.api_key))
            .field("timeout", &self.timeout)
            .field("backoff_base", &self.backoff_base)
            .finish()
    }
}

#[derive(Serialize)]
struct SystemOneRequest<'request> {
    model: &'request str,
    state: &'request Value,
    questions: &'request Questions,
}

struct RequestFailure {
    error: JevError,
    retryable: bool,
    retry_after: Option<Duration>,
}

impl Jev {
    /// Creates a client for `https://api.typesafe.ai` with a ten-second timeout.
    ///
    /// Returns an error if the underlying HTTP client cannot be initialized.
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Result<Self, JevError> {
        let api_key = api_key.into();
        let client = Client::builder()
            // Application retries must remain the only source of extra attempts.
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| network_error(error, &api_key))?;
        Ok(Self {
            client,
            api_key,
            model: model.into(),
            base_url: "https://api.typesafe.ai".into(),
            timeout: Duration::from_secs(10),
            backoff_base: Duration::from_millis(500),
        })
    }

    /// Changes the API origin, for example to a local HTTP test server.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the timeout for each attempt, including reading its response body.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the first retry delay, doubled on each subsequent retry.
    ///
    /// Zero disables fallback delays and jitter. A valid `retry-after` header
    /// takes precedence and is capped at thirty seconds.
    pub fn with_backoff_base(mut self, backoff_base: Duration) -> Self {
        self.backoff_base = backoff_base;
        self
    }

    /// Evaluates named questions against a JSON string, object, or array.
    ///
    /// Retries transient HTTP and transport failures at most three times after
    /// the initial attempt. Invalid requests and undecodable answers are final.
    pub async fn ask(&self, state: &Value, questions: &Questions) -> Result<Answers, JevError> {
        let body = serde_json::to_vec(&SystemOneRequest {
            model: &self.model,
            state,
            questions,
        })
        .map_err(|error| JevError::InvalidRequest {
            message: redact(&error.to_string(), &self.api_key),
        })?;

        let mut attempt = 0;
        loop {
            match self.request(body.clone()).await {
                Ok(answers) => return Ok(answers),
                Err(failure) if failure.retryable && attempt < MAXIMUM_RETRIES => {
                    tokio::time::sleep(self.retry_delay(attempt, failure.retry_after)).await;
                    attempt += 1;
                }
                Err(failure) => return Err(failure.error),
            }
        }
    }

    async fn request(&self, body: Vec<u8>) -> Result<Answers, RequestFailure> {
        let response = self
            .client
            .post(format!(
                "{}/v1/systemone",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.api_key)
            .header(CONTENT_TYPE, "application/json")
            .timeout(self.timeout)
            .body(body)
            .send()
            .await
            .map_err(|error| self.transport_failure(error))?;

        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err(RequestFailure {
                error: JevError::Unauthorized,
                retryable: false,
                retry_after: None,
            });
        }

        let retryable = status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
        let retry_after = retry_after(response.headers());
        let body = match response.bytes().await {
            Ok(body) => body,
            Err(error) if !status.is_success() => {
                // A known rejection stays final even if reading its body fails.
                return Err(RequestFailure {
                    error: self.status_error(status, &error.to_string()),
                    retryable,
                    retry_after,
                });
            }
            Err(error) => return Err(self.transport_failure(error)),
        };

        if !status.is_success() {
            return Err(RequestFailure {
                error: self.status_error(status, &response_message(&body)),
                retryable,
                retry_after,
            });
        }

        serde_json::from_slice(&body).map_err(|error| RequestFailure {
            error: JevError::Decode {
                message: redact(&error.to_string(), &self.api_key),
            },
            retryable: false,
            retry_after: None,
        })
    }

    fn transport_failure(&self, error: reqwest::Error) -> RequestFailure {
        let retryable =
            error.is_timeout() || error.is_connect() || error.is_request() || error.is_body();
        RequestFailure {
            error: network_error(error, &self.api_key),
            retryable,
            retry_after: None,
        }
    }

    fn status_error(&self, status: StatusCode, message: &str) -> JevError {
        let message = if message.trim().is_empty() {
            status
                .canonical_reason()
                .unwrap_or("Unexpected HTTP status")
        } else {
            message
        };
        let message = redact(message, &self.api_key);
        if status == StatusCode::TOO_MANY_REQUESTS {
            JevError::RateLimited { message }
        } else if status.is_server_error() {
            JevError::Overloaded {
                status: status.as_u16(),
                message,
            }
        } else {
            JevError::Rejected {
                status: status.as_u16(),
                message,
            }
        }
    }

    fn retry_delay(&self, attempt: u32, retry_after: Option<Duration>) -> Duration {
        if let Some(delay) = retry_after {
            return delay.min(Duration::from_secs(30));
        }
        let jitter_limit = self.backoff_base.min(Duration::from_millis(100)).as_nanos() as u32;
        let nanoseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.subsec_nanos());
        let jitter = Duration::from_nanos(u64::from(nanoseconds % (jitter_limit + 1)));
        self.backoff_base
            .saturating_mul(1 << attempt)
            .saturating_add(jitter)
    }
}

fn retry_after(headers: &HeaderMap) -> Option<Duration> {
    let seconds = headers
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()?;
    Some(Duration::from_secs(seconds.min(30)))
}

fn network_error(error: reqwest::Error, api_key: &str) -> JevError {
    if error.is_timeout() {
        JevError::Timeout
    } else {
        // Keeping the original error as a source could expose a URL or credentials.
        JevError::Network {
            message: redact(&error.without_url().to_string(), api_key),
        }
    }
}

fn redact(message: &str, api_key: &str) -> String {
    if api_key.is_empty() {
        message.to_owned()
    } else {
        message.replace(api_key, "[REDACTED]")
    }
}

fn response_message(body: &[u8]) -> String {
    if let Ok(value) = serde_json::from_slice::<Value>(body)
        && let Some(message) = value
            .get("message")
            .or_else(|| value.get("detail"))
            .or_else(|| value.pointer("/error/message"))
            .or_else(|| value.get("error"))
    {
        return match message {
            Value::String(message) => message.clone(),
            message => message.to_string(),
        };
    }
    String::from_utf8_lossy(body).trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::Mutex;
    use tokio::task::JoinHandle;

    const API_KEY: &str = "test-secret-api-key";
    const DOCUMENTED_STATE: &str =
        "I was charged twice for my subscription this month. Please refund the extra charge.";
    const NOUL_RESPONSE: &str = r#"{
        "model": "jev-latest",
        "answers": {"billing": {"type": "noul", "noul": 0.98}},
        "usage": {"input_tokens": 120, "output_tokens": 12}
    }"#;
    const CHOICE_RESPONSE: &str = r#"{
        "model": "jev-latest",
        "answers": {"tone": {
            "type": "choice", "choice": "calm", "confidence": 0.7,
            "probabilities": {"angry": 0.15, "calm": 0.8, "excited": 0.05}
        }},
        "usage": {"input_tokens": 120, "output_tokens": 12}
    }"#;
    const SCORE_RESPONSE: &str = r#"{
        "model": "jev-latest",
        "answers": {"urgency": {
            "type": "score", "score": 1.7, "confidence": 0.55,
            "legend": {"0": "Can wait", "1": "Needs attention this week", "2": "Needs attention today"},
            "probabilities": {"0": 0.1, "1": 0.1, "2": 0.8}
        }},
        "usage": {"input_tokens": 120, "output_tokens": 12}
    }"#;

    fn documented_request(questions: &Questions) -> Value {
        // Serialize directly first: converting Choice through Value loses its order.
        let serialized = serde_json::to_string(&SystemOneRequest {
            model: "jev-latest",
            state: &Value::from(DOCUMENTED_STATE),
            questions,
        })
        .unwrap();
        serde_json::from_str(&serialized).unwrap()
    }

    fn documented_response(response: &str) -> Answers {
        let answers: Answers = serde_json::from_str(response).unwrap();
        let documented: Value = serde_json::from_str(response).unwrap();
        assert_eq!(serde_json::to_value(&answers).unwrap(), documented);
        assert_eq!(answers.model, "jev-latest");
        assert_eq!(answers.usage.input_tokens, 120);
        assert_eq!(answers.usage.output_tokens, 12);
        answers
    }

    #[test]
    fn the_documented_noul_request_serializes_to_the_wire_format() {
        let questions =
            Questions::new().with("billing", Noul::new("Is this message about billing?"));
        assert_eq!(
            documented_request(&questions),
            serde_json::json!({
                "model": "jev-latest",
                "state": DOCUMENTED_STATE,
                "questions": {"billing": {
                    "type": "noul", "instructions": "Is this message about billing?"
                }}
            })
        );
    }

    #[test]
    fn the_documented_choice_request_serializes_to_the_wire_format() {
        let questions = Questions::new().with(
            "tone",
            Choice::new(
                "What is the tone of this message?",
                [
                    ("angry", "An upset or hostile message"),
                    ("calm", "A neutral or polite message"),
                    ("excited", "An enthusiastic or eager message"),
                ],
            ),
        );
        assert_eq!(
            documented_request(&questions),
            serde_json::json!({
                "model": "jev-latest",
                "state": DOCUMENTED_STATE,
                "questions": {"tone": {
                    "type": "choice", "instructions": "What is the tone of this message?",
                    "criteria": {
                        "angry": "An upset or hostile message",
                        "calm": "A neutral or polite message",
                        "excited": "An enthusiastic or eager message"
                    }
                }}
            })
        );
    }

    #[test]
    fn the_documented_score_request_serializes_to_the_wire_format() {
        let questions = Questions::new().with(
            "urgency",
            Score::new(
                "How urgent is this message?",
                [
                    "Can wait",
                    "Needs attention this week",
                    "Needs attention today",
                ],
            ),
        );
        assert_eq!(
            documented_request(&questions),
            serde_json::json!({
                "model": "jev-latest",
                "state": DOCUMENTED_STATE,
                "questions": {"urgency": {
                    "type": "score", "instructions": "How urgent is this message?",
                    "criteria": ["Can wait", "Needs attention this week", "Needs attention today"]
                }}
            })
        );
    }

    #[test]
    fn the_documented_noul_response_round_trips_as_a_typed_answer() {
        let answers = documented_response(NOUL_RESPONSE);
        assert_eq!(answers.noul("billing").unwrap().noul, 0.98);
        assert!(answers.choice("billing").is_none());
        assert!(answers.score("billing").is_none());
        assert!(answers.noul("missing").is_none());
    }

    #[test]
    fn the_documented_choice_response_round_trips_as_a_typed_answer() {
        let answers = documented_response(CHOICE_RESPONSE);
        let answer = answers.choice("tone").unwrap();
        assert_eq!(answer.choice, "calm");
        assert_eq!(answer.confidence, 0.7);
        assert_eq!(answer.probabilities["angry"], 0.15);
        assert_eq!(answer.probabilities["calm"], 0.8);
        assert_eq!(answer.probabilities["excited"], 0.05);
        assert!(answers.noul("tone").is_none());
        assert!(answers.score("tone").is_none());
        assert!(answers.choice("missing").is_none());
    }

    #[test]
    fn the_documented_score_response_round_trips_as_a_typed_answer() {
        let answers = documented_response(SCORE_RESPONSE);
        let answer = answers.score("urgency").unwrap();
        assert_eq!(answer.score, 1.7);
        assert_eq!(answer.confidence, 0.55);
        assert_eq!(answer.legend["0"], "Can wait");
        assert_eq!(answer.legend["1"], "Needs attention this week");
        assert_eq!(answer.legend["2"], "Needs attention today");
        assert_eq!(answer.probabilities["0"], 0.1);
        assert_eq!(answer.probabilities["1"], 0.1);
        assert_eq!(answer.probabilities["2"], 0.8);
        assert!(answers.noul("urgency").is_none());
        assert!(answers.choice("urgency").is_none());
        assert!(answers.score("missing").is_none());
    }

    #[test]
    fn choice_criteria_keep_their_order_even_when_names_sort_differently() {
        let choice = Choice::new(
            "Which activity?",
            [
                ("zebra", "Last alphabetically"),
                ("alpha", "First alphabetically"),
                ("middle", "Between"),
            ],
        );
        assert_eq!(
            serde_json::to_string(&choice).unwrap(),
            r#"{"type":"choice","instructions":"Which activity?","criteria":{"zebra":"Last alphabetically","alpha":"First alphabetically","middle":"Between"}}"#
        );
    }

    #[test]
    fn questions_preserve_structured_instructions_and_criteria() {
        let questions = Questions::new()
            .with(
                "billing",
                Noul::new(serde_json::json!({"question": "Billing?"})).with_criteria(
                    NoulCriteria::new(
                        serde_json::json!({"examples": ["Refund"]}),
                        serde_json::json!(["Not a payment"]),
                    ),
                ),
            )
            .with(
                "tone",
                Choice::new(
                    serde_json::json!(["Tone?"]),
                    [
                        ("angry", Value::Null),
                        ("calm", serde_json::json!({"what": "Polite"})),
                    ],
                ),
            )
            .with(
                "urgency",
                Score::new(Value::Null, [serde_json::json!({"what": "Can wait"})]),
            );
        let serialized = documented_request(&questions);
        assert_eq!(
            serialized["questions"]["billing"],
            serde_json::json!({
                "type": "noul", "instructions": {"question": "Billing?"},
                "criteria": {"true": {"examples": ["Refund"]}, "false": ["Not a payment"]}
            })
        );
        assert_eq!(
            serialized["questions"]["tone"],
            serde_json::json!({
                "type": "choice", "instructions": ["Tone?"],
                "criteria": {"angry": null, "calm": {"what": "Polite"}}
            })
        );
        assert_eq!(
            serialized["questions"]["urgency"],
            serde_json::json!({
                "type": "score", "instructions": null, "criteria": [{"what": "Can wait"}]
            })
        );
    }

    #[test]
    fn a_response_can_mix_answer_types_and_structured_score_legends() {
        let response = serde_json::json!({
            "model": "jev-1.14.0",
            "answers": {
                "billing": {"type": "noul", "noul": 0.98},
                "tone": {"type": "choice", "choice": "calm", "confidence": 1.0,
                    "probabilities": {"calm": 1.0}},
                "urgency": {"type": "score", "score": 0.25, "confidence": 0.5,
                    "legend": {"0": {"what": "Can wait"}, "1": ["Today"]},
                    "probabilities": {"0": 0.75, "1": 0.25}}
            },
            "usage": {"input_tokens": 100, "output_tokens": 20}
        });
        let answers: Answers = serde_json::from_value(response.clone()).unwrap();
        assert_eq!(answers.model, "jev-1.14.0");
        assert!(answers.noul("billing").is_some());
        assert!(answers.choice("tone").is_some());
        assert_eq!(
            answers.score("urgency").unwrap().legend["0"],
            serde_json::json!({"what": "Can wait"})
        );
        assert_eq!(serde_json::to_value(answers).unwrap(), response);
    }

    enum Reply {
        Http {
            status: u16,
            headers: String,
            body: String,
        },
        Disconnect,
        Stall {
            send_headers: bool,
        },
    }

    fn response(status: u16, body: impl Into<String>) -> Reply {
        Reply::Http {
            status,
            headers: String::new(),
            body: body.into(),
        }
    }

    fn retry_response(status: u16, retry_after: &str) -> Reply {
        Reply::Http {
            status,
            headers: format!("Retry-After: {retry_after}\r\n"),
            body: r#"{"message":"Try again"}"#.into(),
        }
    }

    #[derive(Clone, Debug)]
    struct CapturedRequest {
        request_line: String,
        headers: BTreeMap<String, String>,
        body: String,
    }

    struct TestServer {
        base_url: String,
        requests: Arc<Mutex<Vec<CapturedRequest>>>,
        task: JoinHandle<()>,
    }

    impl TestServer {
        async fn start(replies: Vec<Reply>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured = Arc::clone(&requests);
            let task = tokio::spawn(async move {
                let mut replies = VecDeque::from(replies);
                let mut stalled_connections = Vec::new();
                loop {
                    let (mut stream, _) = listener.accept().await.unwrap();
                    let request = read_request(&mut stream).await;
                    captured.lock().await.push(request);
                    match replies
                        .pop_front()
                        .unwrap_or_else(|| response(200, NOUL_RESPONSE))
                    {
                        Reply::Http {
                            status,
                            headers,
                            body,
                        } => {
                            let response = format!(
                                "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
                                body.len()
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                        }
                        Reply::Disconnect => {}
                        Reply::Stall { send_headers } => {
                            if send_headers {
                                stream
                                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n")
                                    .await
                                    .unwrap();
                            }
                            // Keep this attempt open while accepting the next timeout retry.
                            stalled_connections.push(stream);
                        }
                    }
                }
            });
            Self {
                base_url,
                requests,
                task,
            }
        }

        fn client(&self) -> Jev {
            Jev::new(API_KEY, "jev-latest")
                .unwrap()
                .with_base_url(&self.base_url)
                .with_timeout(Duration::from_secs(1))
                .with_backoff_base(Duration::ZERO)
        }

        async fn captured_requests(&self) -> Vec<CapturedRequest> {
            assert!(
                !self.task.is_finished(),
                "The local HTTP server exited unexpectedly"
            );
            self.requests.lock().await.clone()
        }
    }

    impl Drop for TestServer {
        fn drop(&mut self) {
            self.task.abort();
        }
    }

    async fn read_request(stream: &mut TcpStream) -> CapturedRequest {
        let mut bytes = Vec::new();
        let headers_end = loop {
            let mut chunk = [0; 4096];
            let count = stream.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0, "Connection closed before request headers");
            bytes.extend_from_slice(&chunk[..count]);
            if let Some(position) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                break position + 4;
            }
        };
        let headers = String::from_utf8(bytes[..headers_end].to_vec()).unwrap();
        let mut lines = headers.lines();
        let request_line = lines.next().unwrap().to_owned();
        let headers: BTreeMap<String, String> = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.to_ascii_lowercase(), value.trim().to_owned()))
            .collect();
        let content_length = headers["content-length"].parse::<usize>().unwrap();
        while bytes.len() < headers_end + content_length {
            let mut chunk = [0; 4096];
            let count = stream.read(&mut chunk).await.unwrap();
            assert_ne!(count, 0, "Connection closed before request body");
            bytes.extend_from_slice(&chunk[..count]);
        }
        CapturedRequest {
            request_line,
            headers,
            body: String::from_utf8(bytes[headers_end..headers_end + content_length].to_vec())
                .unwrap(),
        }
    }

    async fn ask_billing(client: &Jev) -> Result<Answers, JevError> {
        let questions =
            Questions::new().with("billing", Noul::new("Is this message about billing?"));
        tokio::time::timeout(
            Duration::from_secs(2),
            client.ask(&Value::from(DOCUMENTED_STATE), &questions),
        )
        .await
        .expect("The client exceeded the test deadline")
    }

    #[tokio::test]
    async fn requests_send_bearer_authorization_and_the_documented_json_body() {
        let server = TestServer::start(vec![response(200, NOUL_RESPONSE)]).await;
        let answers = ask_billing(
            &server
                .client()
                .with_base_url(format!("{}/", server.base_url)),
        )
        .await
        .unwrap();
        assert_eq!(answers.noul("billing").unwrap().noul, 0.98);
        let requests = server.captured_requests().await;
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.request_line, "POST /v1/systemone HTTP/1.1");
        assert_eq!(
            request.headers["authorization"],
            format!("Bearer {API_KEY}")
        );
        assert_eq!(request.headers["content-type"], "application/json");
        let body: Value = serde_json::from_str(&request.body).unwrap();
        assert_eq!(
            body,
            documented_request(
                &Questions::new().with("billing", Noul::new("Is this message about billing?"))
            )
        );
    }

    #[tokio::test]
    async fn object_and_array_states_and_choice_order_survive_http_serialization() {
        for state in [
            serde_json::json!({"message": "Please help"}),
            serde_json::json!(["Please", "help"]),
        ] {
            let server = TestServer::start(vec![response(200, CHOICE_RESPONSE)]).await;
            let questions = Questions::new().with(
                "tone",
                Choice::new("Tone?", [("zebra", "First"), ("alpha", "Second")]),
            );
            let answers = server.client().ask(&state, &questions).await.unwrap();
            assert_eq!(answers.choice("tone").unwrap().choice, "calm");
            let requests = server.captured_requests().await;
            assert_eq!(requests.len(), 1);
            assert_eq!(
                serde_json::from_str::<Value>(&requests[0].body).unwrap()["state"],
                state
            );
            assert!(
                requests[0]
                    .body
                    .contains(r#""criteria":{"zebra":"First","alpha":"Second"}"#)
            );
        }
    }

    #[tokio::test]
    async fn a_rate_limit_with_zero_retry_after_then_a_success_succeeds() {
        let server =
            TestServer::start(vec![retry_response(429, "0"), response(200, NOUL_RESPONSE)]).await;
        let client = server.client().with_backoff_base(Duration::from_secs(30));
        assert!(ask_billing(&client).await.is_ok());
        let requests = server.captured_requests().await;
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].body, requests[1].body);
        assert_eq!(
            requests[0].headers["authorization"],
            requests[1].headers["authorization"]
        );
    }

    #[tokio::test]
    async fn overload_and_other_server_errors_are_retried_before_success() {
        for status in [529, 500, 502, 503] {
            let server = TestServer::start(vec![
                response(status, r#"{"message":"Overloaded"}"#),
                response(200, NOUL_RESPONSE),
            ])
            .await;
            assert!(ask_billing(&server.client()).await.is_ok());
            assert_eq!(server.captured_requests().await.len(), 2, "status {status}");
        }
    }

    #[tokio::test]
    async fn an_unauthorized_key_fails_after_exactly_one_request() {
        let server = TestServer::start(vec![
            response(401, format!(r#"{{"message":"Bad key {API_KEY}"}}"#)),
            response(200, NOUL_RESPONSE),
        ])
        .await;
        let error = ask_billing(&server.client()).await.unwrap_err();
        assert_eq!(error, JevError::Unauthorized);
        assert_eq!(server.captured_requests().await.len(), 1);
        assert!(!error.to_string().contains(API_KEY));
        assert!(!format!("{error:?}").contains(API_KEY));
    }

    #[tokio::test]
    async fn validation_errors_surface_the_documented_validation_detail() {
        let server = TestServer::start(vec![response(
            422,
            r#"{"detail":[{"loc":["body","state"],"msg":"Field required","type":"missing"}]}"#,
        )])
        .await;
        let error = ask_billing(&server.client()).await.unwrap_err();
        assert!(matches!(error, JevError::Rejected { status: 422, .. }));
        assert!(error.to_string().contains("Field required"));
        assert!(error.to_string().contains("state"));
        assert_eq!(server.captured_requests().await.len(), 1);
    }

    #[tokio::test]
    async fn rejected_requests_are_never_retried_and_keep_their_messages() {
        for status in [400, 413, 415, 422] {
            let server = TestServer::start(vec![
                response(status, r#"{"message":"Invalid question criteria"}"#),
                response(200, NOUL_RESPONSE),
            ])
            .await;
            let error = ask_billing(&server.client()).await.unwrap_err();
            assert_eq!(
                error,
                JevError::Rejected {
                    status,
                    message: "Invalid question criteria".into()
                }
            );
            assert_eq!(server.captured_requests().await.len(), 1);
        }
    }

    #[tokio::test]
    async fn rate_limits_and_server_errors_stop_after_three_retries() {
        for status in [429, 529, 500, 503] {
            let server =
                TestServer::start((0..5).map(|_| retry_response(status, "0")).collect()).await;
            let error = ask_billing(&server.client()).await.unwrap_err();
            if status == 429 {
                assert!(matches!(error, JevError::RateLimited { .. }));
            } else {
                assert!(
                    matches!(error, JevError::Overloaded { status: actual, .. } if actual == status)
                );
            }
            assert!(error.to_string().contains("Try again"));
            assert_eq!(server.captured_requests().await.len(), 4);
        }
    }

    #[tokio::test]
    async fn a_connection_closed_without_a_response_is_retried() {
        let server = TestServer::start(vec![Reply::Disconnect, response(200, NOUL_RESPONSE)]).await;
        assert!(ask_billing(&server.client()).await.is_ok());
        assert_eq!(server.captured_requests().await.len(), 2);
    }

    #[tokio::test]
    async fn persistent_network_failures_stop_after_three_retries() {
        let server = TestServer::start((0..5).map(|_| Reply::Disconnect).collect()).await;
        let error = ask_billing(&server.client()).await.unwrap_err();
        assert!(matches!(error, JevError::Network { .. }));
        assert_eq!(server.captured_requests().await.len(), 4);
        assert!(!format!("{error:?}").contains(API_KEY));
    }

    #[tokio::test]
    async fn timeouts_before_headers_and_during_bodies_stop_after_three_retries() {
        for send_headers in [false, true] {
            let server =
                TestServer::start((0..5).map(|_| Reply::Stall { send_headers }).collect()).await;
            let client = server.client().with_timeout(Duration::from_millis(100));
            assert_eq!(ask_billing(&client).await.unwrap_err(), JevError::Timeout);
            assert_eq!(server.captured_requests().await.len(), 4);
        }
    }

    #[tokio::test]
    async fn a_timeout_can_be_followed_by_a_successful_retry() {
        let server = TestServer::start(vec![
            Reply::Stall {
                send_headers: false,
            },
            response(200, NOUL_RESPONSE),
        ])
        .await;
        let client = server.client().with_timeout(Duration::from_millis(100));
        assert!(ask_billing(&client).await.is_ok());
        assert_eq!(server.captured_requests().await.len(), 2);
    }

    #[tokio::test]
    async fn undecodable_successful_responses_fail_without_retrying() {
        for body in [
            "not JSON",
            r#"{"model":"jev-latest","answers":{"billing":{"type":"noul"}},"usage":{"input_tokens":1,"output_tokens":1}}"#,
            r#"{"model":"jev-latest","answers":{"billing":{"type":"unknown"}},"usage":{"input_tokens":1,"output_tokens":1}}"#,
        ] {
            let server =
                TestServer::start(vec![response(200, body), response(200, NOUL_RESPONSE)]).await;
            assert!(matches!(
                ask_billing(&server.client()).await.unwrap_err(),
                JevError::Decode { .. }
            ));
            assert_eq!(server.captured_requests().await.len(), 1);
        }
    }

    #[tokio::test]
    async fn echoed_credentials_are_removed_from_http_and_decode_errors() {
        for status in [400, 422, 429, 529] {
            let body = format!(r#"{{"message":"Invalid key {API_KEY}"}}"#);
            let server = TestServer::start((0..4).map(|_| response(status, &body)).collect()).await;
            let error = ask_billing(&server.client()).await.unwrap_err();
            assert!(!error.to_string().contains(API_KEY));
            assert!(!format!("{error:?}").contains(API_KEY));
            assert!(error.to_string().contains("[REDACTED]"));
        }
        let body = format!(
            r#"{{"model":"jev-latest","answers":{{"billing":{{"type":"{API_KEY}"}}}},"usage":{{"input_tokens":1,"output_tokens":1}}}}"#
        );
        let server = TestServer::start(vec![response(200, body)]).await;
        let error = ask_billing(&server.client()).await.unwrap_err();
        assert!(matches!(error, JevError::Decode { .. }));
        assert!(!error.to_string().contains(API_KEY));
        assert!(!format!("{error:?}").contains(API_KEY));
    }

    #[test]
    fn client_debug_masks_the_key_even_in_other_configuration_fields() {
        let client = Jev::new(API_KEY, format!("model-{API_KEY}"))
            .unwrap()
            .with_base_url(format!("http://localhost/{API_KEY}"));
        let debug = format!("{client:?}");
        assert!(!debug.contains(API_KEY));
        assert!(debug.contains("[REDACTED]"));
    }

    #[test]
    fn default_timeouts_and_backoff_follow_the_documented_retry_policy() {
        let client = Jev::new(API_KEY, "jev-latest").unwrap();
        assert_eq!(client.timeout, Duration::from_secs(10));
        assert_eq!(client.base_url, "https://api.typesafe.ai");
        for (attempt, milliseconds) in [500, 1000, 2000].into_iter().enumerate() {
            let delay = client.retry_delay(attempt as u32, None);
            assert!(delay >= Duration::from_millis(milliseconds));
            assert!(delay <= Duration::from_millis(milliseconds + 100));
        }
        assert_eq!(client.retry_delay(0, Some(Duration::ZERO)), Duration::ZERO);
        assert_eq!(
            client.retry_delay(0, Some(Duration::from_secs(100))),
            Duration::from_secs(30)
        );
        assert_eq!(
            client
                .with_backoff_base(Duration::ZERO)
                .retry_delay(0, None),
            Duration::ZERO
        );
    }

    #[test]
    fn retry_after_uses_integer_seconds_and_caps_the_server_delay() {
        for (value, expected) in [
            ("0", Some(Duration::ZERO)),
            ("2", Some(Duration::from_secs(2))),
            ("300", Some(Duration::from_secs(30))),
            ("not seconds", None),
            ("-1", None),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(RETRY_AFTER, value.parse().unwrap());
            assert_eq!(retry_after(&headers), expected);
        }
        assert_eq!(retry_after(&HeaderMap::new()), None);
    }

    #[test]
    fn response_messages_support_detail_nested_errors_and_plain_text() {
        assert_eq!(
            response_message(br#"{"detail":"Too much nesting"}"#),
            "Too much nesting"
        );
        assert_eq!(
            response_message(br#"{"error":{"message":"Model not found"}}"#),
            "Model not found"
        );
        assert_eq!(
            response_message(b"  Service unavailable  "),
            "Service unavailable"
        );
    }
}
