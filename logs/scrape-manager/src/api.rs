use twilight_http::Client;
use twilight_model::id::{GuildId, ChannelId};
use twilight_model::channel::GuildChannel;

/// Limit of how many guilds discord will send in response to asking which guilds we're in.
const GUILD_REQUEST_LIMIT: u64 = 200;

/// Asks discord for a list of all of the guilds that architus is in.
pub async fn get_guilds(client: &Client) -> Result<Vec<GuildId>, ()> {
    // Architus is currently only in ~450 guilds. 1000 is a good number that
    // allows room to grow. Also, a GuildId is just a newtype for a u64 so
    // this shouldn't actually take up that much space (like 2 pages).
    let mut architus_guilds: Vec<GuildId> = Vec::with_capacity(1000);

    let request = client.current_user_guilds()
        .limit(GUILD_REQUEST_LIMIT).expect("Will succeed as long as `GUILD_REQUEST_LIMIT` is kept up to date")
        .exec().await;

    let response = match request {
        Ok(r) => r,
        Err(_) => return Err(()),
    };

    let data = response.models().await;
    let guilds = match data {
        Ok(d) => d,
        Err(_) => return Err(()),
    };

    for g in &guilds {
        architus_guilds.push(g.id);
    }

    while guilds.len() >= (GUILD_REQUEST_LIMIT as usize) {
        let request = client.current_user_guilds()
            .limit(GUILD_REQUEST_LIMIT).expect("Will succeed as long as `GUILD_REQUEST_LIMIT` is kept up to date")
            .exec().await;

        let response = match request {
            Ok(r) => r,
            Err(_) => return Err(()),
        };

        let data = response.models().await;
        let guilds = match data {
            Ok(d) => d,
            Err(_) => return Err(()),
        };

        for g in &guilds {
            architus_guilds.push(g.id);
        }
    }

    Ok(architus_guilds)
}

/// Ask discord for a list of all the channels in a guild.
pub async fn get_channels(client: &Client, guild: GuildId) -> Result<Vec<ChannelId>, ()> {
    let mut guild_channels: Vec<ChannelId> = Vec::with_capacity(300);

    let request = client.guild_channels(guild).exec().await;
    let response = match request {
        Ok(r) => r,
        Err(_) => return Err(()),
    };

    let data = response.models().await;
    let channels = match data {
        Ok(d) => d,
        Err(_) => return Err(()),
    };

    for c in &channels {
        // NOTE: This is where you add the additional type of channels you want to use.
        // See the twilight model docs for the types of channels that discord has.
        match c {
            GuildChannel::Text(tc) => guild_channels.push(tc.id),
            _ => {},
        };
    }

    Ok(guild_channels)
}
