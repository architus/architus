use std::fmt::{self, Write};

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
pub fn write_custom_emoji(
    writer: &mut impl Write,
    id: u64,
    name: Option<&str>,
    animated: bool,
) -> Result<(), fmt::Error> {
    let animated_prefix = if animated { "a" } else { "" };
    let maybe_name = name.unwrap_or("");
    write!(writer, "<{}:{}:{}>", animated_prefix, maybe_name, id)
}

// TODO test write_user_mention
// TODO test write_role_mention
// TODO test write_channel_mention
// TODO test write_custom_emoji
