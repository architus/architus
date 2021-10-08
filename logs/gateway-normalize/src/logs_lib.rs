//! TODO move to own crate

use fancy_regex::Regex;
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use std::fmt::{self, Write};
use url::Url;

/// Helper function that collects all mention regex matches
/// in the given content string.
/// Each mention regex should have a single capture group
/// that contains the numeric ID.
fn find_mentions(content: &str, regex: &Regex) -> Vec<u64> {
    // Use a set to de-duplicate mentions
    let mut mention_set = BTreeSet::<u64>::new();
    for maybe_capture in regex.captures_iter(content) {
        if let Ok(capture) = maybe_capture {
            if let Some(id_capture) = capture.get(1) {
                if let Ok(id) = id_capture.as_str().parse::<u64>() {
                    mention_set.insert(id);
                }
            }
        }
    }

    mention_set.into_iter().collect::<Vec<_>>()
}

/// Gets all Discord user mentions in the given content string.
/// User mentions look like `<@448546825532866560>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
///
/// ### Note
/// This function finds mentions in code blocks as well as in the rest of the contents.
/// This differs from how Discord normally renders mentions
/// (they are ignored if they are in a code block),
/// so this somewhat deviates from Discord's behavior.
/// When it comes to the Architus logs feature, though, we prefer indexing events
/// as having a mention even if it is in a code block,
/// simply because it is easier to implement
/// and results in marginally more complete indexed log data.
pub fn find_user_mentions(content: &str) -> Vec<u64> {
    lazy_static! {
        static ref USER_MENTION_REGEX: Regex = Regex::new(r#"<@([0-9]+)>"#).unwrap();
    }

    find_mentions(content, &USER_MENTION_REGEX)
}

/// Gets all Discord role mentions in the given content string.
/// Role mentions look like `<@&607639217840848910>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
///
/// ### Note
/// This function finds mentions in code blocks as well as in the rest of the contents.
/// This differs from how Discord normally renders mentions
/// (they are ignored if they are in a code block),
/// so this somewhat deviates from Discord's behavior.
/// When it comes to the Architus logs feature, though, we prefer indexing events
/// as having a mention even if it is in a code block,
/// simply because it is easier to implement
/// and results in marginally more complete indexed log data.
pub fn find_role_mentions(content: &str) -> Vec<u64> {
    lazy_static! {
        static ref ROLE_MENTION_REGEX: Regex = Regex::new(r#"<@&([0-9]+)>"#).unwrap();
    }

    find_mentions(content, &ROLE_MENTION_REGEX)
}

/// Gets all Discord channel mentions in the given content string.
/// Channel mentions look like `<#641064458843586562>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
///
/// ### Note
/// This function finds mentions in code blocks as well as in the rest of the contents.
/// This differs from how Discord normally renders mentions
/// (they are ignored if they are in a code block),
/// so this somewhat deviates from Discord's behavior.
/// When it comes to the Architus logs feature, though, we prefer indexing events
/// as having a mention even if it is in a code block,
/// simply because it is easier to implement
/// and results in marginally more complete indexed log data.
pub fn find_channel_mentions(content: &str) -> Vec<u64> {
    lazy_static! {
        static ref CHANNEL_MENTION_REGEX: Regex = Regex::new(r#"<#([0-9]+)>"#).unwrap();
    }

    find_mentions(content, &CHANNEL_MENTION_REGEX)
}

pub struct CustomEmojiUsages<'a> {
    pub ids: Vec<u64>,
    pub names: Vec<&'a str>,
}

/// Scans a given Discord content string for all custom emoji uses.
/// Returns a de-duplicated list of both the IDs of the emojis used and their names.
/// Custom emoji uses look like `<:architus:792017989583110154>` in Discord rich content,
/// which get converted to the interact-able emoji element
/// in both the Discord client and the Architus logs web dashboard.
/// See https://docs.archit.us/internal/modules/logs/rich-content/
///
/// ### Note
/// This function finds custom emoji uses in code blocks as well as in the rest of the contents.
/// This differs from how Discord normally renders custom emoji
/// (they are ignored if they are in a code block),
/// so this somewhat deviates from Discord's behavior.
/// When it comes to the Architus logs feature, though, we prefer indexing events
/// as having a custom emoji even if it is in a code block,
/// simply because it is easier to implement
/// and results in marginally more complete indexed log data.
pub fn find_custom_emoji_uses<'a>(content: &'a str) -> CustomEmojiUsages<'a> {
    lazy_static! {
        static ref CUSTOM_EMOJI_MENTION_REGEX: Regex =
            Regex::new(r#"<a?:([A-Za-z0-9_-]+):([0-9]+)>"#).unwrap();
    }

    // Use sets to de-duplicate ids/names
    let mut mentioned_ids = BTreeSet::<u64>::new();
    let mut mentioned_names = BTreeSet::<&'a str>::new();
    for maybe_capture in CUSTOM_EMOJI_MENTION_REGEX.captures_iter(content) {
        if let Ok(capture) = maybe_capture {
            if let Some(id_capture) = capture.get(2) {
                if let Ok(id) = id_capture.as_str().parse::<u64>() {
                    mentioned_ids.insert(id);
                }
            }

            if let Some(name_capture) = capture.get(1) {
                mentioned_names.insert(name_capture.as_str());
            }
        }
    }

    CustomEmojiUsages {
        ids: mentioned_ids.into_iter().collect::<Vec<_>>(),
        names: mentioned_names.into_iter().collect::<Vec<_>>(),
    }
}

/// Scans a content string for alL URL-like strings.
/// This includes:
/// - www.google.com
/// - https://docs.archit.us/
/// - http://archit.us/app?code=XXX
pub fn find_urls(content: &str) -> Vec<&str> {
    // Thank you Stack Overflow :)
    // https://stackoverflow.com/a/17773849/13192375
    const URL_REGEX_RAW: &'static str = r#"(https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|www\.[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9]+\.[^\s]{2,}|www\.[a-zA-Z0-9]+\.[^\s]{2,})"#;

    lazy_static! {
        static ref URL_REGEX: Regex = Regex::new(URL_REGEX_RAW).unwrap();
    }

    // Use sets to de-duplicate urls
    let mut urls = BTreeSet::<&str>::new();
    for maybe_capture in URL_REGEX.captures_iter(content) {
        if let Ok(capture) = maybe_capture {
            if let Some(whole_match) = capture.get(0) {
                urls.insert(whole_match.as_str());
            }
        }
    }

    urls.into_iter().collect::<Vec<_>>()
}

/// Collects a list of URL-like strings into a de-duplicated list of "url stems".
/// These are substrings of the URL's domain
/// where each possible hierarchical subdomain is considered as a url stem.
/// For example, the following URLs:
/// - www.google.com
/// - https://docs.archit.us/
/// - http://archit.us/app?code=XXX
///
/// will return a vector that contains:
/// `["www.google.com", "google.com", "docs.archit.us", "archit.us"]`
pub fn collect_url_stems<'a>(urls: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    // Use sets to de-duplicate url stems
    let mut url_stems = BTreeSet::<String>::new();
    for url in urls {
        let stems_option = if url.starts_with("https://") || url.starts_with("http://") {
            get_url_stems(url)
        } else {
            get_url_stems(String::from("https://") + url)
        };

        if let Some(stems) = stems_option {
            url_stems.extend(stems.into_iter());
        }
    }

    url_stems.into_iter().collect::<Vec<_>>()
}

/// Gets all URL stems from a valid URL string.
/// These are substrings of the URL's domain
/// where each possible hierarchical subdomain is considered as a url stem.
/// For example, an input url of `https://api.develop.archit.us/guild_count`
/// would return `Some(["api.develop.archit.us", "develop.archit.us", "archit.us"])`
pub fn get_url_stems(raw_url: impl AsRef<str>) -> Option<Vec<String>> {
    let parsed_url = Url::parse(raw_url.as_ref()).ok()?;
    let domain = parsed_url.host_str()?;

    let mut segments_reversed = domain.split(".").collect::<Vec<_>>();
    segments_reversed.reverse();
    if segments_reversed.len() < 2 {
        return None;
    }

    let (first, rest) = segments_reversed.as_slice().split_at(1);
    let mut accum = first.into_iter().collect::<Vec<_>>();
    let mut url_stems = Vec::<String>::new();
    for segment in rest {
        accum.push(segment);
        url_stems.push(
            accum
                .iter()
                .rev()
                .map(|s| String::from(**s))
                .collect::<Vec<String>>()
                .join("."),
        );
    }

    Some(url_stems)
}

/// Writes a user mention that will be displayed using rich formatting.
/// User mentions look like `<@448546825532866560>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
pub fn write_user_mention(writer: &mut impl Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<@{}>", id)
}

/// Writes a role mention that will be displayed using rich formatting.
/// Role mentions look like `<@&607639217840848910>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
pub fn write_role_mention(writer: &mut impl Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<@&{}>", id)
}

/// Writes a channel mention that will be displayed using rich formatting.
/// Channel mentions look like `<#641064458843586562>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
pub fn write_channel_mention(writer: &mut impl Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<#{}>", id)
}

/// Writes a channel mention that will be displayed using rich formatting.
/// Channel mentions look like `<#641064458843586562>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See https://docs.archit.us/internal/modules/logs/rich-content/
pub fn write_custom_emoji(writer: &mut impl Write, id: u64, name: Option<&str>, animated: bool) -> Result<(), fmt::Error> {
    let animated_prefix = if animated { "a" } else { "" };
    let maybe_name = name.unwrap_or("");
    write!(writer, "<{}:{}:{}>", animated_prefix, maybe_name, id)
}

// TODO add some tests
