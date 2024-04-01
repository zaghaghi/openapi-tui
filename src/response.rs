pub struct Response {
  pub status: reqwest::StatusCode,
  pub version: reqwest::Version,
  pub headers: reqwest::header::HeaderMap,
  pub content_length: Option<u64>,
  pub body: String,
}
