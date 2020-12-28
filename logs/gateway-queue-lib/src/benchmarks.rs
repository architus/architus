extern crate test;

use crate::GatewayEventOwned;
use architus_id::HoarFrost;
use rand::distributions::Alphanumeric;
use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use rand::{Rng, RngCore, SeedableRng};
use serde_json::json;
use test::{black_box, Bencher};

pub fn gen_events(count: usize) -> Vec<GatewayEventOwned> {
    let seed = 0x67_dd_cb_0c_a6_41_ce_89_u64;
    let mut random = StdRng::seed_from_u64(seed);
    let guild_id_pool = (0..5).map(|_| random.next_u64()).collect::<Vec<_>>();
    let event_names = vec![
        "MESSAGE_SEND",
        "MESSAGE_DELETE",
        "GUILD_UPDATE",
        "MESSAGE_EDIT",
        "CHANNEL_CREATE",
        "CHANNEL_DELETE",
    ];
    (0..count)
        .map(|_| {
            let content_length = random.gen_range(1, 2000);
            let content_seed = random.next_u64();
            let content: String = StdRng::seed_from_u64(content_seed)
                .sample_iter(&Alphanumeric)
                .take(content_length)
                .collect();
            let author_id = random.next_u64().to_string();
            let channel_id = random.next_u64().to_string();
            let guild_id = random.next_u64().to_string();
            let message_id = random.next_u64().to_string();
            let role_id = random.next_u64().to_string();
            let nonce = random.next_u64().to_string();
            GatewayEventOwned {
                id: HoarFrost(random.next_u64()),
                ingress_timestamp: random.next_u64(),
                inner: json!({
                  "attachments": [],
                  "author": {
                    "avatar": "a_f24c7db8c107a4f0fdf9242c27bf8682",
                    "discriminator": "1881",
                    "id": author_id,
                    "public_flags": 132,
                    "username": "joazlazer"
                  },
                  "channel_id": channel_id,
                  "content": content,
                  "edited_timestamp": null,
                  "embeds": [],
                  "flags": 0,
                  "guild_id": guild_id,
                  "id": message_id,
                  "member": {
                    "deaf": false,
                    "hoisted_role": null,
                    "joined_at": "2019-07-05T01:53:53.493+05:30",
                    "mute": false,
                    "roles": [
                      role_id
                    ]
                  },
                  "mention_everyone": false,
                  "mention_roles": [],
                  "mentions": [],
                  "nonce": nonce,
                  "pinned": false,
                  "referenced_message": null,
                  "timestamp": "2020-12-15T05:08:34.315+05:30",
                  "tts": false,
                  "type": 0
                }),
                event_type: event_names.iter().choose(&mut random).unwrap().to_string(),
                guild_id: guild_id_pool.iter().choose(&mut random).cloned(),
            }
        })
        .collect::<Vec<_>>()
}

#[bench]
fn bench_serialize_json(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(serde_json::to_vec(&event).unwrap());
    });
}

#[bench]
fn bench_serialize_json_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(serde_json::to_vec(event).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_json(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = serde_json::to_vec(&event).unwrap();
    b.iter(|| {
        black_box(serde_json::from_slice::<GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_json_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| serde_json::to_vec(event).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(serde_json::from_slice::<GatewayEventOwned>(slice).unwrap());
        });
    });
}

#[bench]
fn bench_serialize_messagepack(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(rmp_serde::to_vec(&event).unwrap());
    });
}

#[bench]
fn bench_serialize_messagepack_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(rmp_serde::to_vec(event).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_messagepack(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = rmp_serde::to_vec(&event).unwrap();
    b.iter(|| {
        black_box(rmp_serde::from_slice::<GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_messagepack_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| rmp_serde::to_vec(event).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(rmp_serde::from_slice::<GatewayEventOwned>(slice).unwrap());
        });
    });
}

#[bench]
fn bench_serialize_bincode(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(bincode::serialize(&event).unwrap());
    });
}

#[bench]
fn bench_serialize_bincode_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(bincode::serialize(event).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_bincode(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = bincode::serialize(&event).unwrap();
    b.iter(|| {
        black_box(bincode::deserialize::<GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_bincode_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| bincode::serialize(event).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(bincode::deserialize::<GatewayEventOwned>(slice).unwrap());
        });
    });
}

#[bench]
fn bench_serialize_pickle(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(serde_pickle::to_vec(&event, false).unwrap());
    });
}

#[bench]
fn bench_serialize_pickle_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(serde_pickle::to_vec(event, false).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_pickle(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = serde_pickle::to_vec(&event, false).unwrap();
    b.iter(|| {
        black_box(serde_pickle::from_slice::<GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_pickle_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| serde_pickle::to_vec(event, false).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(serde_pickle::from_slice::<GatewayEventOwned>(slice).unwrap());
        });
    });
}

#[bench]
fn bench_serialize_pickle3(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(serde_pickle::to_vec(&event, true).unwrap());
    });
}

#[bench]
fn bench_serialize_pickle3_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(serde_pickle::to_vec(event, true).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_pickle3(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = serde_pickle::to_vec(&event, true).unwrap();
    b.iter(|| {
        black_box(serde_pickle::from_slice::<GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_pickle3_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| serde_pickle::to_vec(event, true).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(serde_pickle::from_slice::<GatewayEventOwned>(slice).unwrap());
        });
    });
}

#[bench]
fn bench_serialize_xdr(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(serde_xdr::to_bytes(&event).unwrap());
    });
}

#[bench]
fn bench_serialize_xdr_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(serde_xdr::to_bytes(event).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_xdr(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = serde_xdr::to_bytes(&event).unwrap();
    b.iter(|| {
        black_box(serde_xdr::from_bytes::<_, GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_xdr_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| serde_xdr::to_bytes(event).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(serde_xdr::from_bytes::<_, GatewayEventOwned>(slice).unwrap());
        });
    });
}

#[bench]
fn bench_serialize_cbor(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    b.iter(|| {
        black_box(serde_cbor::to_vec(&event).unwrap());
    });
}

#[bench]
fn bench_serialize_cbor_100(b: &mut Bencher) {
    let events = gen_events(100);
    b.iter(|| {
        events.iter().for_each(|event| {
            black_box(serde_cbor::to_vec(event).unwrap());
        });
    });
}

#[bench]
fn bench_deserialize_cbor(b: &mut Bencher) {
    let event = gen_events(1).into_iter().next().unwrap();
    let bytes = serde_cbor::to_vec(&event).unwrap();
    b.iter(|| {
        black_box(serde_cbor::from_slice::<GatewayEventOwned>(&bytes).unwrap());
    });
}

#[bench]
fn bench_deserialize_cbor_100(b: &mut Bencher) {
    let events = gen_events(100);
    let bytes = events
        .iter()
        .map(|event| serde_cbor::to_vec(event).unwrap())
        .collect::<Vec<_>>();
    b.iter(|| {
        bytes.iter().for_each(|slice| {
            black_box(serde_cbor::from_slice::<GatewayEventOwned>(slice).unwrap());
        });
    });
}
