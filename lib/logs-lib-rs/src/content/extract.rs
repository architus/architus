use fancy_regex::Regex;
use lazy_static::lazy_static;
use std::collections::BTreeSet;
use url::Url;

/// Helper function that collects all mention regex matches
/// in the given content string.
/// Each mention regex should have a single capture group
/// that contains the numeric ID.
#[must_use]
fn find_mentions(content: &str, regex: &Regex) -> Vec<u64> {
    // Use a set to de-duplicate mentions
    let mut mention_set = BTreeSet::<u64>::new();
    for capture in regex.captures_iter(content).flatten() {
        if let Some(id_capture) = capture.get(1) {
            if let Ok(id) = id_capture.as_str().parse::<u64>() {
                mention_set.insert(id);
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
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
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
#[must_use]
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
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
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
#[must_use]
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
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
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
#[must_use]
pub fn find_channel_mentions(content: &str) -> Vec<u64> {
    lazy_static! {
        static ref CHANNEL_MENTION_REGEX: Regex = Regex::new(r#"<#([0-9]+)>"#).unwrap();
    }

    find_mentions(content, &CHANNEL_MENTION_REGEX)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CustomEmojiUsages<'a> {
    pub ids: Vec<u64>,
    pub names: Vec<&'a str>,
}

/// Scans a given Discord content string for all custom emoji uses.
/// Returns a de-duplicated list of both the IDs of the emojis used and their names.
/// Custom emoji uses look like `<:architus:792017989583110154>` in Discord rich content,
/// which get converted to the interact-able emoji element
/// in both the Discord client and the Architus logs web dashboard.
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
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
///
/// ### Note
/// This function supports an extension to the syntax:
/// where custom emoji can be missing their name
/// (and only contain the id, as in `<a::814220915033899059>`
/// or `<::792017989583110154>`)
#[must_use]
pub fn find_custom_emoji_uses<'a>(content: &'a str) -> CustomEmojiUsages<'a> {
    lazy_static! {
        static ref CUSTOM_EMOJI_MENTION_REGEX: Regex =
            Regex::new(r#"<a?:([A-Za-z0-9_-]*):([0-9]+)>"#).unwrap();
    }

    // Use sets to de-duplicate ids/names
    let mut mentioned_ids = BTreeSet::<u64>::new();
    let mut mentioned_names = BTreeSet::<&'a str>::new();
    for capture in CUSTOM_EMOJI_MENTION_REGEX.captures_iter(content).flatten() {
        if let Some(name_capture) = capture.get(1) {
            if !name_capture.as_str().is_empty() {
                mentioned_names.insert(name_capture.as_str());
            }
        }

        if let Some(id_capture) = capture.get(2) {
            if let Ok(id) = id_capture.as_str().parse::<u64>() {
                mentioned_ids.insert(id);
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
/// - `www.google.com`
/// - `https://docs.archit.us/`
/// - `http://archit.us/app?code=XXX`
#[must_use]
pub fn find_urls(content: &str) -> Vec<&str> {
    // Thank you Stack Overflow :)
    // https://stackoverflow.com/a/17773849/13192375
    const URL_REGEX_RAW: &str = r#"(https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|www\.[a-zA-Z0-9][a-zA-Z0-9-]+[a-zA-Z0-9]\.[^\s]{2,}|https?:\/\/(?:www\.|(?!www))[a-zA-Z0-9]+\.[^\s]{2,}|www\.[a-zA-Z0-9]+\.[^\s]{2,})"#;

    lazy_static! {
        static ref URL_REGEX: Regex = Regex::new(URL_REGEX_RAW).unwrap();
    }

    // Use sets to de-duplicate urls
    let mut urls = BTreeSet::<&str>::new();
    for capture in URL_REGEX.captures_iter(content).flatten() {
        if let Some(whole_match) = capture.get(0) {
            urls.insert(whole_match.as_str());
        }
    }

    urls.into_iter().collect::<Vec<_>>()
}

/// Collects a list of URL-like strings into a de-duplicated list of "url stems".
/// These are substrings of the URL's domain
/// where each possible hierarchical subdomain is considered as a url stem.
/// For example, the following URLs:
/// - `www.google.com`
/// - `https://docs.archit.us/`
/// - `http://archit.us/app?code=XXX`
///
/// will return a vector that contains:
/// `["www.google.com", "google.com", "docs.archit.us", "archit.us"]`
#[must_use]
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
#[must_use]
pub fn get_url_stems(raw_url: impl AsRef<str>) -> Option<Vec<String>> {
    let parsed_url = Url::parse(raw_url.as_ref()).ok()?;
    let domain = parsed_url.host_str()?;

    let mut segments_reversed = domain.split('.').collect::<Vec<_>>();
    segments_reversed.reverse();
    if segments_reversed.len() < 2 {
        return None;
    }

    let (first, rest) = segments_reversed.as_slice().split_at(1);
    let mut accum = first.iter().collect::<Vec<_>>();
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_find_user_mentions() {
        use super::find_user_mentions;

        // Hack to bypass type inference limitations
        let empty_vec: Vec<u64> = vec![];

        // The function should find only the user mentions
        // among all other rich content syntax.
        assert_eq!(
            find_user_mentions(
                r#"
                    **bold**
                    *italic*
                    _italic_
                    __underline__
                    ~~strikethrough~~
                    ||spoilers||
                    ***bold_italic***
                    link: https://docs.archit.us/
                    user mention: <@448546825532866560>
                    role mention: <@&607639217840848910>
                    channel mention: <#641064458843586562>
                    custom (animated) emoji: <a:catKiss:814220915033899059>
                    custom emoji: <:architus:792017989583110154>
                    color mention: <##F97448>
                    unicode emoji: 🤔 :thinking: :thinking_face:
                "#
            ),
            vec!(448546825532866560),
        );
        assert_eq!(find_user_mentions(""), empty_vec);
        assert_eq!(
            find_user_mentions(
                "<@74646589213257728>
                <@214037134477230080>
                <@293486380183453696>"
            ),
            // The results happen to be in sorted order
            // due to the function using a BTreeSet internally
            vec!(74646589213257728, 214037134477230080, 293486380183453696),
        );

        // User mentions should get de-duplicated
        assert_eq!(
            find_user_mentions(
                "<@448546825532866560>
                <@448546825532866560>
                <@448546825532866560>"
            ),
            vec!(448546825532866560),
        );

        // These aren't valid user mentions
        assert_eq!(
            find_user_mentions(
                "<448546825532866560>
                @448546825532866560
                448546825532866560"
            ),
            empty_vec,
        );
    }

    #[test]
    fn test_find_role_mentions() {
        use super::find_role_mentions;

        // Hack to bypass type inference limitations
        let empty_vec: Vec<u64> = vec![];

        // The function should find only the role mentions
        // among all other rich content syntax.
        assert_eq!(
            find_role_mentions(
                r#"
                    **bold**
                    *italic*
                    _italic_
                    __underline__
                    ~~strikethrough~~
                    ||spoilers||
                    ***bold_italic***
                    link: https://docs.archit.us/
                    user mention: <@448546825532866560>
                    role mention: <@&607639217840848910>
                    channel mention: <#641064458843586562>
                    custom (animated) emoji: <a:catKiss:814220915033899059>
                    custom emoji: <:architus:792017989583110154>
                    color mention: <##F97448>
                    unicode emoji: 🤔 :thinking: :thinking_face:
                "#
            ),
            vec!(607639217840848910),
        );
        assert_eq!(find_role_mentions(""), empty_vec);
        assert_eq!(
            find_role_mentions(
                "<@&607639474956009492>
                <@&608083673866174679>
                <@&692116370199937075>"
            ),
            // The results happen to be in sorted order
            // due to the function using a BTreeSet internally
            vec!(607639474956009492, 608083673866174679, 692116370199937075),
        );

        // Role mentions should get de-duplicated
        assert_eq!(
            find_role_mentions(
                "<@&607639217840848910>
                <@&607639217840848910>
                <@&607639217840848910>"
            ),
            vec!(607639217840848910),
        );

        // These aren't valid role mentions
        assert_eq!(
            find_role_mentions(
                "<607639217840848910>
                <&607639217840848910>
                @&607639217840848910
                607639217840848910"
            ),
            empty_vec,
        );
    }

    #[test]
    fn test_find_channel_mentions() {
        use super::find_channel_mentions;

        // Hack to bypass type inference limitations
        let empty_vec: Vec<u64> = vec![];

        // The function should find only the channel mentions
        // among all other rich content syntax.
        assert_eq!(
            find_channel_mentions(
                r#"
                    **bold**
                    *italic*
                    _italic_
                    __underline__
                    ~~strikethrough~~
                    ||spoilers||
                    ***bold_italic***
                    link: https://docs.archit.us/
                    user mention: <@448546825532866560>
                    role mention: <@&607639217840848910>
                    channel mention: <#641064458843586562>
                    custom (animated) emoji: <a:catKiss:814220915033899059>
                    custom emoji: <:architus:792017989583110154>
                    color mention: <##F97448>
                    unicode emoji: 🤔 :thinking: :thinking_face:
                "#
            ),
            vec!(641064458843586562)
        );
        assert_eq!(find_channel_mentions(""), empty_vec);
        assert_eq!(
            find_channel_mentions(
                "<#641064458843586562>
                <#608083673866174679>
                <#607641549186007041>"
            ),
            // The results happen to be in sorted order
            // due to the function using a BTreeSet internally
            vec!(607641549186007041, 608083673866174679, 641064458843586562),
        );

        // Channel mentions should get de-duplicated
        assert_eq!(
            find_channel_mentions(
                "<#641064458843586562>
                <#641064458843586562>
                <#641064458843586562>"
            ),
            vec!(641064458843586562),
        );

        // These aren't valid channel mentions
        assert_eq!(
            find_channel_mentions(
                "<641064458843586562>
                #641064458843586562
                641064458843586562"
            ),
            empty_vec,
        );
    }

    #[test]
    fn test_find_custom_emoji_uses() {
        use super::{find_custom_emoji_uses, CustomEmojiUsages};

        // The function should find only the custom emoji uses
        // among all other rich content syntax.
        assert_eq!(
            find_custom_emoji_uses(
                r#"
                    **bold**
                    *italic*
                    _italic_
                    __underline__
                    ~~strikethrough~~
                    ||spoilers||
                    ***bold_italic***
                    link: https://docs.archit.us/
                    user mention: <@448546825532866560>
                    role mention: <@&607639217840848910>
                    channel mention: <#641064458843586562>
                    custom (animated) emoji: <a:catKiss:814220915033899059>
                    custom emoji: <:architus:792017989583110154>
                    color mention: <##F97448>
                    unicode emoji: 🤔 :thinking: :thinking_face:
                "#
            ),
            CustomEmojiUsages {
                // The results happen to be in sorted order
                // due to the function using a BTreeSet internally
                ids: vec!(792017989583110154, 814220915033899059),
                names: vec!("architus", "catKiss"),
            }
        );
        assert_eq!(
            find_custom_emoji_uses(""),
            CustomEmojiUsages {
                ids: vec!(),
                names: vec!(),
            }
        );
        assert_eq!(
            find_custom_emoji_uses(
                "<a:peepoGiggle:778726394897104926>
                <:YEP:732954591318245376>
                <a:catKiss:814220915033899059>
                In our extension syntax, a missing name is ok:
                <::900000000000000000>"
            ),
            CustomEmojiUsages {
                // The results happen to be in sorted order
                // due to the function using a BTreeSet internally
                ids: vec!(
                    732954591318245376,
                    778726394897104926,
                    814220915033899059,
                    900000000000000000
                ),
                names: vec!("YEP", "catKiss", "peepoGiggle"),
            }
        );

        // Custom emoji uses should get de-duplicated
        assert_eq!(
            find_custom_emoji_uses(
                "<a:catKiss:814220915033899059>
                <a:catKiss:814220915033899059>
                <a:catKiss:814220915033899059>"
            ),
            CustomEmojiUsages {
                ids: vec!(814220915033899059),
                names: vec!("catKiss"),
            }
        );

        // These aren't valid custom emoji uses
        assert_eq!(
            find_custom_emoji_uses(
                "<a:catKiss:>
                <g:catKiss:814220915033899059>
                :catKiss:
                🤔"
            ),
            CustomEmojiUsages {
                ids: vec!(),
                names: vec!(),
            }
        );
    }

    #[test]
    fn test_find_urls() {
        use super::find_urls;

        // Hack to bypass type inference limitations
        let empty_vec: Vec<&str> = vec![];

        // The function should find only the urls
        // among all other rich content syntax.
        assert_eq!(
            find_urls(
                r#"
                    **bold**
                    *italic*
                    _italic_
                    __underline__
                    ~~strikethrough~~
                    ||spoilers||
                    ***bold_italic***
                    link: https://docs.archit.us/
                    user mention: <@448546825532866560>
                    role mention: <@&607639217840848910>
                    channel mention: <#641064458843586562>
                    custom (animated) emoji: <a:catKiss:814220915033899059>
                    custom emoji: <:architus:792017989583110154>
                    color mention: <##F97448>
                    unicode emoji: 🤔 :thinking: :thinking_face:
                "#
            ),
            vec!("https://docs.archit.us/")
        );
        assert_eq!(find_urls(""), empty_vec);
        assert_eq!(
            find_urls(
                "https://docs.archit.us/internal/modules/logs/rich-content/#user-mention
                www.google.com
                https://archit.us
                http://discordapp.com
                "
            ),
            // The results happen to be in sorted order
            // due to the function using a BTreeSet internally
            vec!(
                "http://discordapp.com",
                "https://archit.us",
                "https://docs.archit.us/internal/modules/logs/rich-content/#user-mention",
                "www.google.com",
            ),
        );

        // URLs should get de-duplicated
        assert_eq!(
            find_urls(
                "https://docs.archit.us/
                https://docs.archit.us/
                https://docs.archit.us/"
            ),
            vec!("https://docs.archit.us/"),
        );

        // These aren't valid URLs
        assert_eq!(
            find_urls(
                "archit.us
                docs.archit.us
                mailto:someone@yoursite.com
                ftp://architus:password@archit.us:21/path
                http:/archit.us"
            ),
            empty_vec,
        );
    }

    #[test]
    fn test_collect_url_stems() {
        use super::collect_url_stems;

        // Hack to bypass type inference limitations
        let empty_str_vec: Vec<&str> = vec![];
        let empty_string_vec: Vec<&str> = vec![];

        assert_eq!(collect_url_stems(empty_str_vec), empty_string_vec);
        assert_eq!(
            collect_url_stems(vec!(
                "https://api.develop.archit.us",
                "http://api.archit.us",
                "gateway.archit.us",
            )),
            // The results happen to be in sorted order
            // due to the function using a BTreeSet internally
            vec!(
                String::from("api.archit.us"),
                String::from("api.develop.archit.us"),
                String::from("archit.us"),
                String::from("develop.archit.us"),
                String::from("gateway.archit.us"),
            ),
        );
    }

    #[test]
    fn test_get_url_stems() {
        use super::get_url_stems;

        assert_eq!(get_url_stems(""), None);
        assert_eq!(
            get_url_stems("https://api.develop.archit.us"),
            // The results are ordered starting at the highest-level domain
            // and proceeding down the hierarchy
            Some(vec!(
                String::from("archit.us"),
                String::from("develop.archit.us"),
                String::from("api.develop.archit.us"),
            ))
        );
        assert_eq!(
            get_url_stems("https://archit.us"),
            Some(vec!(String::from("archit.us")))
        );
    }
}
