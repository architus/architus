#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::too_many_lines)]

//! Implements an HTTP api for getting and setting scrape work that needs to be
//! done by the consumer.
//! Endpoints of note are:
//! 1. GET /work
//!     This endpoint is where work can be retrieved by a consumer
//! 2. POST /redo
//!     This endpoint is where someone can request a unit of work to be redone
//! 3. POST /update
//!     This endpoint is where someone can request a unit of work to be moved
//!     to the front of the work queue

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::convert::TryInto;
use log::{info, warn};

use tokio::time::{sleep, Duration};
use tokio::runtime::Builder; 
use tokio_postgres::NoTls;
use tokio_postgres::error::SqlState;
use twilight_http::Client;
use twilight_model::id::ChannelId;

use rocket::get;
use rocket::http::Status;
use rocket::post;
use rocket::State;
use rocket::serde::json::Json;

mod api;
mod work;

// This is the initial queue size for the work queue. I think 1,000 is reasonable for number of
// guilds + channels.
const INIT_QUEUE_SIZE: usize = 1_000;
const MS_IN_DAY: u64 = 86_400_000;
const AUDIT_CHANNEL: ChannelId = ChannelId(0);

fn main() {
    // Set up configuration values
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("bot_settings")).expect("Couldn't find settings file");
    let token = settings.get_str("architus_token").expect("Need to set `architus_token` param in settings file");
    let num_threads: usize = settings
        .get_int("tokio_threads")
        .expect("Set how many threads tokio should use")
        .try_into()
        .expect("Using a non usize number of threads for tokio");

    let work_queue = work::WorkQueue::new(INIT_QUEUE_SIZE);
    let state = Arc::new(Mutex::new(work_queue));

    let tokio_runtime = Builder::new_multi_thread()
        .worker_threads(num_threads)
        .thread_name("scrape-work-manager")
        .enable_all()
        .build()
        .unwrap();

    let server = rocket::build().mount("/", rocket::routes![redo_timespan, get_work, send_to_front]).manage(state.clone());

    // create thread for the work queue updater
    tokio_runtime.spawn(update_queue(state.clone(), token));

    // Start the runtime by blocking on the server
    tokio_runtime.block_on(server.launch()).expect("Server went down");
}

/// This is the get endpoint for a scrape consumer to retrieve pending work.
/// It will return a `work::Work` struct encoded to JSON. If there is no work
/// to be done, it will return the `work::NULL_WORK` struct.
#[get("/work")]
fn get_work(state: &State<Arc<Mutex<work::WorkQueue>>>) -> Json<work::Work> {
    let mut queue = state.lock().unwrap();
    let work = queue.get_work();
    drop(queue);

    match work {
        Some(w) => Json(w),
        None => Json(work::NULL_WORK),
    }
}

/// Endpoint for redoing some amount of scrape work. This will just add to the end of the
/// work queue. A `work::Work` struct encoded to json will need to be passed in as part of
/// the body of the request. This endpoint will not try to merge the requested work with
/// any that is currently in the queue.
#[post("/redo", data = "<w>")]
fn redo_timespan(state: &State<Arc<Mutex<work::WorkQueue>>>, w: Json<work::Work>) -> Status {
    let mut queue = state.lock().unwrap();
    queue.add_work(*w);
    drop(queue);

    Status::Created
}

/// Endpoint for moving some amount of work to the front of the work queue. Expects a `work::Work`
/// struct encdoded to json as part of the body of the struct. Will try to merge this work with
/// any overlapping work currently in the queue.
#[post("/update", data = "<w>")]
fn send_to_front(state: &State<Arc<Mutex<work::WorkQueue>>>, w: Json<work::Work>) -> Status {
    let mut queue = state.lock().unwrap();
    queue.move_to_front(*w);
    drop(queue);

    Status::Created
}

/// Main loop for periodically updating the work queue. Roughly every hour, it will wake up and
/// go through every guild and channel Architus can see and add relevant work to the work queue.
async fn update_queue(state: Arc<Mutex<work::WorkQueue>>, token: String) -> ! {
    // Some times that are useful for sleeping
    let hour = Duration::new(3600, 0);
    let minute = Duration::new(3600, 0);

    // This is the mainloop for updating. It will check if it needs to do update the work queue
    // every hour.
    'update: loop {
        // Sleep at the beginning of the loop so no awaits happen while some non-send value is
        // in scope.
        sleep(minute).await;
        let mut need_to_update = false;
        {
            // Set the update flag in its own block to avoid send + sync requirements on the queue
            // lock. rustc still thinks it's in scope even if you drop it.
            let queue = state.lock().unwrap();
            if queue.requires_update() {
                need_to_update = true;
            }
        }
        // If there is no need to update the queue, just sleep for an hour and then check again.
        if !need_to_update {
            sleep(hour).await;
            continue 'update;
        }

        info!("Updating work queue");
        let discord_client = Client::new(token.clone());
        let guilds = api::get_guilds(&discord_client).await;

        let guilds = match guilds {
            Ok(g) => g,
            Err(_) => {
                warn!("Unable to get guild list from discord");
                continue 'update;
            },
        };

        let (db_client, connection) =
            match tokio_postgres::connect("host=postgres user=autbot pass=autism port=5432", NoTls).await {
                Ok(c) => c,
                Err(_) => {
                    warn!("Could not make connection to the postgres database");
                    continue 'update;
                }
            };
        // This is where the actual connection will be made.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                warn!("postgres err {}", e);
            }
        });

        for g in guilds {
            let start: u64;
            let end: u64;
            let backlog_done: bool;
            match db_client.query_one("SELECT * FROM scrape_tracker WHERE guild_id = $1", &[&(g.0 as i64)]).await {
                Ok(r) => {
                    start = r.get::<_, i64>("start_date") as u64;
                    end = r.get::<_, i64>("end_date") as u64;
                    backlog_done = r.get("backlog_done");
                },
                Err(e) => {
                    match e.code() {
                        Some(&SqlState::NO_DATA) => {
                            info!("Now tracking guild {} for audit log", g.0);
                            let curr_time = SystemTime::now();
                            // This should only fail past the year 500 million or so. At that
                            // point, not my problem. Or, it could fail if run before the year
                            // 1970. Also, not my problem.
                            start = (curr_time.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() - MS_IN_DAY as u128).try_into().expect("Architus is too old");
                            end = start;
                            backlog_done = false;
                        },
                        _ => {
                            warn!("Couldn't fetch guild {} from the audit state table", g.0);
                            continue 'update;
                        }
                    };
                },
            };

            // Need this to be it's own scope because the queue mutex lock cannot be taken across
            // an await. So it needs to be dropped before it is used later for adding work to the
            // channels since there are awaits in between.
            {
                let mut queue = state.lock().unwrap();
                let current = work::Timespan(end, end + MS_IN_DAY);
                let current_work = work::Work(g, AUDIT_CHANNEL, current);
                queue.add_work(current_work);
                if !backlog_done {
                    let backlog = work::Timespan(start - MS_IN_DAY, start);
                    let backlog_work = work::Work(g, AUDIT_CHANNEL, backlog);
                    queue.add_work(backlog_work);
                }
            }

            let channels = api::get_channels(&discord_client, g).await;
            let channels = match channels {
                Ok(c) => c,
                Err(()) => {
                    warn!("Unable to get channels for guild {}", g.0);
                    continue 'update;
                }
            };

            for chan in &channels {
                let start: u64;
                let end: u64;
                let backlog_done: bool;
                match db_client.query_one("SELECT * FROM channel_tracker WHERE channel_id = $1", &[&(chan.0 as i64)]).await {
                    Ok(r) => {
                        start = r.get::<_, i64>("start_date") as u64;
                        end = r.get::<_, i64>("end_date") as u64;
                        backlog_done = r.get("backlog_done");
                    },
                    Err(e) => {
                        match e.code() {
                            Some(&SqlState::NO_DATA) => {
                                info!("Now tracking channel {}", chan.0);
                                let curr_time = SystemTime::now();
                                // See above for why this should never fail.
                                start = (curr_time.duration_since(UNIX_EPOCH).expect("Time went backwards").as_millis() - MS_IN_DAY as u128).try_into().expect("Architus is too old");
                                end = start;
                                backlog_done = false;
                            },
                            _ => {
                                warn!("Could not find any info for channel {}", chan.0);
                                continue 'update;
                            },
                        };
                    },
                };

                {
                    let mut queue = state.try_lock().unwrap();

                    let new_timespan = work::Timespan(end, end + MS_IN_DAY);
                    let new_work = work::Work(g, *chan, new_timespan);
                    queue.add_work(new_work);
                    if !backlog_done {
                        let backlock_time = work::Timespan(start - MS_IN_DAY, start);
                        let backlog_work = work::Work(g, *chan, backlock_time);
                        queue.add_work(backlog_work);
                    }
                }
            }
        }

        let mut queue = state.lock().unwrap();
        queue.update_timestamp();
    }
}
