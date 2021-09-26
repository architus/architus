mod scrape;

use tokio::runtime::Builder;

use twilight_http::Client;

const TOKEN: &str = "PUT TOKEN HERE";

#[tokio::main(flavor="multi_thread", worker_threads=4)]
async fn main() {
    let client = Client::new(TOKEN.to_owned());

    let guilds = scrape::get_guilds(&client).await;

    let guild = match guilds {
        Ok(gs) => gs[0],
        Err(e) => {
            println!("{:?}", e);
            return;
        },
    };

    let audit_events = scrape::scrape_before(&client, guild, None).await;

    match audit_events {
        Ok(events) => {
            println!("Got {} event(s)", events.entries.len());
            events.entries.iter().for_each(|e| println!("{:?}", e));
        },
        Err(e) => println!("{:?}", e),
    };
}

async fn worker() {
    loop {}
}
