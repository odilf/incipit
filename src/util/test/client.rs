use color_eyre::eyre;
use reqwest::RequestBuilder;

pub fn builder(host: &str, path: &str) -> RequestBuilder {
    let client = reqwest::Client::new();

    let path = path.trim_start_matches('/');

    client
        .get(format!("http://localhost/{path}"))
        .header("Host", host)
}

/// Fetches a url. The host header is set to the host parameter, but it always fetches localhost.
pub async fn fetch(host: &str, path: &str) -> eyre::Result<String> {
    let response = builder(host, path).send().await?;

    Ok(response.text().await?)
}
