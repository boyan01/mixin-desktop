//! GIF discovery through the desktop network service and its proxy configuration.

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use url::Url;

use super::{desktop::DesktopRuntime, model::GifItem};

impl DesktopRuntime {
    pub fn gif_search_available(&self) -> bool {
        api_key().is_some()
    }

    pub async fn search_gifs(
        &self,
        query: String,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<GifItem>> {
        let key = api_key().ok_or_else(|| anyhow!("GIF search requires MIXIN_GIPHY_API_KEY"))?;
        let endpoint = if query.trim().is_empty() {
            "trending"
        } else {
            "search"
        };
        let mut url = Url::parse(&format!("https://api.giphy.com/v1/gifs/{endpoint}"))?;
        url.query_pairs_mut()
            .append_pair("api_key", &key)
            .append_pair("limit", &limit.clamp(1, 50).to_string())
            .append_pair("offset", &offset.to_string());
        if endpoint == "search" {
            url.query_pairs_mut().append_pair("q", query.trim());
        }
        // Transport errors can contain their URL; never expose the key to logs or UI.
        let response = self
            .http_request(
                "GET",
                url.as_str(),
                HashMap::new(),
                None,
                Some(15_000),
                Some(4 * 1024 * 1024),
            )
            .await
            .map_err(|_| anyhow!("The GIF service could not be reached"))?;
        if !(200..300).contains(&response.status_code) {
            return Err(anyhow!(
                "The GIF service returned HTTP {}",
                response.status_code
            ));
        }
        parse(&response.body)
    }
}

fn api_key() -> Option<String> {
    std::env::var("MIXIN_GIPHY_API_KEY")
        .ok()
        .or_else(|| option_env!("MIXIN_GIPHY_API_KEY").map(str::to_owned))
        .filter(|key| !key.trim().is_empty())
}

fn parse(bytes: &[u8]) -> Result<Vec<GifItem>> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let rows = value
        .get("data")
        .and_then(|data| data.as_array())
        .ok_or_else(|| anyhow!("Invalid GIF service response"))?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            let id = row.get("id")?.as_str()?.to_owned();
            let images = row.get("images")?;
            let send = images.get("fixed_width")?;
            let preview = images.get("fixed_width_downsampled").unwrap_or(send);
            let positive = |key| {
                send.get(key)
                    .and_then(|value| value.as_str())
                    .and_then(|value| value.parse::<i32>().ok())
                    .filter(|value| *value > 0)
            };
            let url = send.get("url")?.as_str()?;
            let preview_url = preview.get("url")?.as_str()?;
            for candidate in [url, preview_url] {
                if Url::parse(candidate).ok()?.scheme() != "https" {
                    return None;
                }
            }
            Some(GifItem {
                id,
                url: url.into(),
                preview_url: preview_url.into(),
                width: positive("width")?,
                height: positive("height")?,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn gif_results_keep_send_and_downsampled_preview_separate_and_skip_invalid_media() {
        let payload = serde_json::json!({"data":[
            {"id":"good", "images":{"fixed_width":{"url":"https://media.giphy.com/send.gif","width":"200","height":"100"},"fixed_width_downsampled":{"url":"https://media.giphy.com/preview.gif"}}},
            {"id":"bad", "images":{"fixed_width":{"url":"file:///secret","width":"200","height":"100"}}},
            {"id":"zero", "images":{"fixed_width":{"url":"https://media.giphy.com/zero.gif","width":"0","height":"100"}}}
        ]});
        let rows = parse(&serde_json::to_vec(&payload).unwrap()).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "good");
        assert_eq!(rows[0].url, "https://media.giphy.com/send.gif");
        assert_eq!(rows[0].preview_url, "https://media.giphy.com/preview.gif");
        assert_eq!((rows[0].width, rows[0].height), (200, 100));
        assert!(parse(br#"{"error":"unavailable"}"#).is_err());
    }
}
