use std::fmt::{self, Write};

/// Writes a user mention that will be displayed using rich formatting.
/// User mentions look like `<@448546825532866560>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
///
/// # Errors
/// - `std::fmt::Error` if the write to the given writer fails
pub fn write_user_mention(writer: &mut impl Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<@{}>", id)
}

/// Writes a role mention that will be displayed using rich formatting.
/// Role mentions look like `<@&607639217840848910>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
///
/// # Errors
/// - `std::fmt::Error` if the write to the given writer fails
pub fn write_role_mention(writer: &mut impl Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<@&{}>", id)
}

/// Writes a channel mention that will be displayed using rich formatting.
/// Channel mentions look like `<#641064458843586562>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
///
/// # Errors
/// - `std::fmt::Error` if the write to the given writer fails
pub fn write_channel_mention(writer: &mut impl Write, id: u64) -> Result<(), fmt::Error> {
    write!(writer, "<#{}>", id)
}

/// Writes a channel mention that will be displayed using rich formatting.
/// Channel mentions look like `<#641064458843586562>` in Discord rich content,
/// which get converted to the interact-able mention element
/// in both the Discord client and the Architus logs web dashboard.
///
/// See <https://docs.archit.us/internal/modules/logs/rich-content/>
///
/// # Errors
/// - `std::fmt::Error` if the write to the given writer fails
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

#[cfg(test)]
mod tests {
    fn write_to_string(f: impl Fn(&mut String) -> ()) -> String {
        let mut s = String::new();
        f(&mut s);
        s
    }

    #[test]
    fn test_write_user_mention() {
        use super::write_user_mention;

        assert_eq!(
            write_to_string(|s| write_user_mention(s, 74646589213257728).unwrap()),
            String::from("<@74646589213257728>"),
        );
        assert_eq!(
            write_to_string(|s| write_user_mention(s, 214037134477230080).unwrap()),
            String::from("<@214037134477230080>"),
        );
        assert_eq!(
            write_to_string(|s| write_user_mention(s, 293486380183453696).unwrap()),
            String::from("<@293486380183453696>"),
        );
    }

    #[test]
    fn test_write_role_mention() {
        use super::write_role_mention;

        assert_eq!(
            write_to_string(|s| write_role_mention(s, 607639474956009492).unwrap()),
            String::from("<@&607639474956009492>"),
        );
        assert_eq!(
            write_to_string(|s| write_role_mention(s, 608083673866174679).unwrap()),
            String::from("<@&608083673866174679>"),
        );
        assert_eq!(
            write_to_string(|s| write_role_mention(s, 692116370199937075).unwrap()),
            String::from("<@&692116370199937075>"),
        );
    }

    #[test]
    fn test_write_channel_mention() {
        use super::write_channel_mention;

        assert_eq!(
            write_to_string(|s| write_channel_mention(s, 641064458843586562).unwrap()),
            String::from("<#641064458843586562>"),
        );
        assert_eq!(
            write_to_string(|s| write_channel_mention(s, 608083673866174679).unwrap()),
            String::from("<#608083673866174679>"),
        );
        assert_eq!(
            write_to_string(|s| write_channel_mention(s, 607641549186007041).unwrap()),
            String::from("<#607641549186007041>"),
        );
    }

    #[test]
    fn test_write_custom_emoji() {
        use super::write_custom_emoji;

        assert_eq!(
            write_to_string(
                |s| write_custom_emoji(s, 814220915033899059, Some("catKiss"), true).unwrap()
            ),
            String::from("<a:catKiss:814220915033899059>"),
        );
        assert_eq!(
            write_to_string(|s| write_custom_emoji(s, 814220915033899059, None, true).unwrap()),
            String::from("<a::814220915033899059>"),
        );
        assert_eq!(
            write_to_string(
                |s| write_custom_emoji(s, 792017989583110154, Some("architus"), false).unwrap()
            ),
            String::from("<:architus:792017989583110154>"),
        );
        assert_eq!(
            write_to_string(|s| write_custom_emoji(s, 792017989583110154, None, false).unwrap()),
            String::from("<::792017989583110154>"),
        );
    }
}
