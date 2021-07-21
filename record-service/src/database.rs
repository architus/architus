use amiquip::{Connection, Exchange, Publish};

pub fn get_connection(url: String) -> Result<T, Error> {
    let connection = Connection::insecure_open(url)
}

pub fn get_muted_ids(client: &mut Client, guild: u64) -> Vec<u64> {
    let mut ids = Vec::<u64>::new();
    for row in client.query("SELECT id From (guild) ($1)", &[&guild]) {
        ids.push_back(row.get(0));
    }
    ids
}
