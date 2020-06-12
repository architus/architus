use db::*;

use std::thread::sleep;
use std::time::Duration;

fn main() {
    sleep(Duration::from_secs(20));
    let conn = establish_connection();
    let f = get_feature_by_id(&conn, 2).expect("Failed query 1");
    match f {
        Some(feat) => println!("id 2 is: {}", feat.name),
        None => println!("id 2 has no associated feature"),
    };

    let f = get_feature_by_id(&conn, 256).expect("Failed query 2");
    match f {
        Some(feat) => println!("id 256 is: {}", feat.name),
        None => println!("id 256 has no associated feature"),
    };
}
