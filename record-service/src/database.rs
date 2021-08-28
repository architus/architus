//! Handles connections between the postgres database and the
//! record service.

use postgres::{Client, NoTls};

pub struct Database(Client);

impl Database {
    pub fn get_connection(url: String, user: String, pass: String) -> anyhow::Result<Self> {
        let db = format!("host={} user={} password={} port=5432", url, user, pass);
        match Client::connect(db, NoTls) {
            Ok(c) => Ok(Self(c)),
            Err(_) => Err(()),
        }
    }

    pub fn get_muted_ids(&mut self, guild: u64) -> Vec<u64> {
        let mut ids = Vec::<u64>::new();
        for row in self.0.query("SELECT id From (guild) ($1)", &[&guild]) {
            ids.push_back(row.get(0));
        }
        ids
    }
}
