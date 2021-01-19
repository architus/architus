use anyhow::{Context, Result};
use std::collections::HashMap;

/// Represents a database of emoji shortcodes
/// that can be used to forward and reverse resolve emojis at runtime
#[derive(Debug, Clone)]
pub struct EmojiDb {
    shortcodes_to_emojis: HashMap<String, String>,
    emojis_to_shortcodes: HashMap<String, Vec<String>>,
}

impl EmojiDb {
    /// Loads the emoji database from the URL,
    /// which should contain a JSON document of shortcode->emoji mappings
    pub async fn load(url: impl AsRef<str>) -> Result<Self> {
        // Download and parse the JSON
        let url = url.as_ref();
        let body = reqwest::get(url)
            .await
            .with_context(|| format!("Failed to retrieve emojis from {}", url))?
            .text()
            .await
            .with_context(|| format!("Failed to retrieve emojis from {}", url))?;
        let mut shortcodes_to_emojis = serde_json::from_str::<HashMap<String, String>>(&body)
            .with_context(|| format!("Failed to parse emojis retrieved from {}", url))?;

        // Assemble the reverse mapping
        let mut emojis_to_shortcodes = HashMap::new();
        for (shortcode, emoji) in &shortcodes_to_emojis {
            emojis_to_shortcodes
                .entry(emoji.clone())
                .or_insert_with(Vec::new)
                .push(shortcode.clone());
        }

        // Compact each data structure
        for (_, shortcodes) in emojis_to_shortcodes.iter_mut() {
            shortcodes.shrink_to_fit();
        }
        shortcodes_to_emojis.shrink_to_fit();
        emojis_to_shortcodes.shrink_to_fit();

        Ok(Self {
            shortcodes_to_emojis,
            emojis_to_shortcodes,
        })
    }

    /// Attempts to resolve the shortcodes for a given emoji
    pub fn to_shortcodes(&self, emoji: impl AsRef<str>) -> Option<&Vec<String>> {
        self.emojis_to_shortcodes.get(emoji.as_ref())
    }

    /// Attempts to resolve the emoji from a shortcode
    pub fn from_shortcode(&self, shortcode: impl AsRef<str>) -> Option<&str> {
        self.shortcodes_to_emojis
            .get(shortcode.as_ref())
            .map(String::as_str)
    }
}
