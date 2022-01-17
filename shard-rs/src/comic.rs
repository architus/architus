use anyhow::Context;
use scraper::{Html, Selector};
use serde::Deserialize;
use twilight_model::channel::embed::{Embed, EmbedFooter, EmbedImage};

#[allow(clippy::module_name_repetitions)]
pub struct ComicData {
    title: String,
    description: String,
    url: String,
    image: String,
    color: u32,
    footer: String,
}

impl From<ComicData> for Embed {
    fn from(c: ComicData) -> Self {
        Self {
            author: None,
            color: Some(c.color),
            description: Some(c.description),
            fields: vec![],
            footer: Some(EmbedFooter {
                icon_url: None,
                proxy_icon_url: None,
                text: c.footer,
            }),
            image: Some(EmbedImage {
                height: None,
                proxy_url: None,
                url: c.image,
                width: None,
            }),
            kind: String::from("rich"),
            provider: None,
            thumbnail: None,
            timestamp: None,
            title: Some(c.title),
            url: Some(c.url),
            video: None,
        }
    }
}

const SMBC_URL: &str = "https://smbc-comics.com";

pub async fn get_latest_smbc() -> anyhow::Result<ComicData> {
    let body = reqwest::get(SMBC_URL).await?.text().await?;
    let document = Html::parse_document(&body);
    let img_selector = Selector::parse("#cc-comic").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let title_selector = Selector::parse("head > title").map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let img = document
        .select(&img_selector)
        .next()
        .with_context(|| format!("failed to retrieve img from {}", SMBC_URL))?;
    let title = document
        .select(&title_selector)
        .next()
        .with_context(|| format!("failed to retrieve title from {}", SMBC_URL))?
        .inner_html();
    let src = img
        .value()
        .attr("src")
        .ok_or_else(|| anyhow::anyhow!("no src on thing"))?;
    let img_title = img
        .value()
        .attr("title")
        .ok_or_else(|| anyhow::anyhow!("no title on thing"))?;

    Ok(ComicData {
        title: String::from("Today's SMBC Comic"),
        description: title,
        url: String::from(SMBC_URL),
        image: String::from(src),
        color: 0x7b_8f_b7,
        footer: String::from(img_title),
    })
}

#[derive(Deserialize)]
struct XkcdResponse {
    safe_title: String,
    img: String,
    alt: String,
}

const XKCD_URL: &str = "https://xkcd.com/info.0.json";

pub async fn get_latest_xkcd() -> anyhow::Result<ComicData> {
    // Download and parse the JSON
    let response_future = reqwest::get(String::from(XKCD_URL));
    let response = response_future
        .await
        .with_context(|| format!("failed to retrieve XKCD info from {}", XKCD_URL))?;
    let body_future = response.text();
    let body = body_future
        .await
        .with_context(|| format!("failed to retrieve XKCD info from {}", XKCD_URL))?;

    let response = serde_json::from_str::<XkcdResponse>(&body)
        .with_context(|| format!("Failed to parse XKCD info retrieved from {}", XKCD_URL))?;

    Ok(ComicData {
        title: String::from("Today's XKCD Comic"),
        description: response.safe_title,
        url: String::from("https://xkcd.com"),
        image: response.img,
        color: 0x7b_8f_b7,
        footer: response.alt,
    })
}
