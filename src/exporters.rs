use reqwest::Request;

fn shell_quote(s: &str) -> String {
  format!("'{}'", s.replace('\'', r"'\''"))
}

fn body_string(req: &Request) -> Option<String> {
  let body = req.body()?;
  let bytes = body.as_bytes()?;
  Some(String::from_utf8_lossy(bytes).into_owned())
}

pub fn to_curl(req: &Request) -> String {
  let mut parts = vec!["curl".to_string()];
  parts.push("-X".to_string());
  parts.push(req.method().as_str().to_string());
  parts.push(shell_quote(req.url().as_str()));

  for (name, value) in req.headers() {
    let v = value.to_str().unwrap_or("");
    parts.push("-H".to_string());
    parts.push(shell_quote(&format!("{}: {v}", name.as_str())));
  }

  if let Some(body) = body_string(req) {
    parts.push("--data-raw".to_string());
    parts.push(shell_quote(&body));
  }

  parts.join(" ")
}

pub fn to_httpie(req: &Request) -> String {
  let body = body_string(req);

  let mut parts = Vec::new();
  if body.is_some() {
    // httpie reads request body from stdin when piped.
    parts.push(format!("echo {} |", shell_quote(body.as_deref().unwrap_or(""))));
  }
  parts.push("http".to_string());
  parts.push(req.method().as_str().to_string());
  parts.push(shell_quote(req.url().as_str()));

  for (name, value) in req.headers() {
    let v = value.to_str().unwrap_or("");
    parts.push(shell_quote(&format!("{}:{v}", name.as_str())));
  }

  parts.join(" ")
}

#[cfg(test)]
mod tests {
  use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

  use super::*;

  fn build(method: reqwest::Method, url: &str, headers: HeaderMap, body: Option<&[u8]>) -> Request {
    let mut req = reqwest::Client::new().request(method, url).headers(headers);
    if let Some(b) = body {
      req = req.body(b.to_vec());
    }
    req.build().unwrap()
  }

  fn auth_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer tok"));
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h
  }

  #[test]
  fn curl_get_no_body() {
    let req = build(reqwest::Method::GET, "https://api.example.com/v1/pets?limit=2", auth_headers(), None);
    let out = to_curl(&req);
    assert!(out.starts_with("curl -X GET 'https://api.example.com/v1/pets?limit=2'"));
    assert!(out.contains("-H 'authorization: Bearer tok'"));
    assert!(out.contains("-H 'content-type: application/json'"));
    assert!(!out.contains("--data-raw"));
  }

  #[test]
  fn curl_post_with_body_escapes_quote() {
    let req =
      build(reqwest::Method::POST, "https://api.example.com/v1/pets", auth_headers(), Some(br#"{"name":"O'Reilly"}"#));
    let out = to_curl(&req);
    assert!(out.contains("-X POST"));
    assert!(out.contains(r#"--data-raw '{"name":"O'\''Reilly"}'"#));
  }

  #[test]
  fn httpie_get_no_body() {
    let req = build(reqwest::Method::GET, "https://api.example.com/v1/pets", auth_headers(), None);
    let out = to_httpie(&req);
    assert!(out.starts_with("http GET 'https://api.example.com/v1/pets'"));
    assert!(out.contains("'authorization:Bearer tok'"));
    assert!(!out.contains("echo"));
  }

  #[test]
  fn httpie_post_pipes_body() {
    let req =
      build(reqwest::Method::POST, "https://api.example.com/v1/pets", auth_headers(), Some(br#"{"name":"Rex"}"#));
    let out = to_httpie(&req);
    assert!(out.starts_with("echo '{\"name\":\"Rex\"}' | http POST"));
  }
}
