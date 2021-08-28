use songbird::model::payload::{ClientConnect, ClientDisconnect, Speaking};
use std::cmp::max;
use std::collections::HashMap;
use std::num::Wrapping;
use std::sync::Arc;

use crate::zipper::*;

use discortp::rtp::Rtp;
use serenity::async_trait;
use songbird::events::EventHandler as VoiceHandler;
use songbird::{Event, EventContext};
use tokio::sync::Mutex;

const MAX_SEQ_BREAK: Wrapping<u16> = Wrapping(10u16);
const MAX_SEQ_NUM: Wrapping<u16> = Wrapping(65530);

// Need a new)type that wraps the actual recording in an arc mutex because
// Serenity requires us to make multiple references to it.
pub struct Recording(pub Arc<Mutex<WAVReceiver>>);

/// Stores all relevant state about recording of voice channel.
pub struct WAVReceiver {
    // User ids that will not be included in the recording.
    // This is necessary for complying with the discord privacy
    // policy.
    disallowed_ids: Vec<u64>,

    // Holds the bots id so that it can be excluded from any
    // recording.
    bot_id: u64,

    // This will hold the actual voice data received over the network.
    // Each `Vec<u16>` will be a separate channel of audio data representing
    // a single user. This bot will compress all audio data from a user to
    // a single channel so only one vec encoding a single channel per user
    // is needed. This can be done completely losslessly as discord simply sends
    // mono-channel audio encoded into dual-channel audio which produces
    // two channels with exactly the same information.
    audio_channels: Vec<Vec<i16>>,

    // The total number of audio packets received and saved. Useful for doing
    // some basic sketchy synchronization of audio across channels.
    packet_count: u64,

    // This will end up being a user_id of every user that gets recorded.
    // The index of the user in this list will equate to which channel
    // that user is in the WAV file.
    user_list: Vec<u64>,

    // Holds the last sequence number sent by each channel.
    last_sequence: Vec<Wrapping<u16>>,

    // This will be for keeping track of ssrc <-> discord id.
    uid_map: HashMap<u32, u64>,
}

// This is basically just a dispatch to the actual handling functions.
// The songbird library just sends everything to this one method on
// this trait inside of a massive enum.
#[async_trait]
impl VoiceHandler for Recording {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let mut wav = self.0.lock().await;
        match ctx {
            EventContext::VoicePacket { audio, packet, .. } => {
                if let Some(a) = audio {
                    if a.len() != 0 {
                        wav.handle_data(a, packet);
                    }
                }
            }
            EventContext::ClientConnect(client) => {
                wav.user_join(client);
            }
            EventContext::ClientDisconnect(client) => {
                wav.user_leave(client);
            }
            EventContext::SpeakingStateUpdate(Speaking { ssrc, user_id, .. }) => {
                if let Some(id) = user_id {
                    wav.speaking_update(*ssrc, id.0);
                }
            }
            _ => return None,
        };
        return None;
    }
}

impl WAVReceiver {
    pub fn new(disallowed: Vec<u64>, bot: u64) -> Self {
        Self {
            disallowed_ids: disallowed,
            bot_id: bot,
            audio_channels: Vec::new(),
            packet_count: 0,
            user_list: Vec::new(),
            last_sequence: Vec::new(),
            uid_map: HashMap::new(),
        }
    }

    // Calls out to the zipping module to do all of the actual file stuff.
    // Returns back the url and password for the zip file on the manager.
    pub fn save(&mut self) -> Result<(String, String), PublishError> {
        self.balance(true);
        return zip(&self.audio_channels);
    }

    // Handle a packet of audio data.
    fn handle_data(&mut self, audio: &Vec<i16>, packet: &Rtp) {
        let ssrc = packet.ssrc;
        let uid = match self.uid_map.get(&ssrc) {
            Some(u) => u.clone(),
            None => return,
        };
        let seq = packet.sequence.0;
        let channel_index = self.get_channel(uid);
        if channel_index.is_none() {
            return;
        }
        let channel_index = channel_index.unwrap();

        // Drop packet if future packets have already been processed.
        if (self.last_sequence[channel_index] < seq)
            || (seq < MAX_SEQ_BREAK && self.last_sequence[channel_index] > MAX_SEQ_NUM)
        {
            self.last_sequence[channel_index] = seq;
        } else {
            return;
        }

        // Add pcm data to vector. Want to have a step of 2 because all
        // audio coming from discord should be stereo channel.
        for pcm in audio.iter().step_by(2) {
            self.audio_channels[channel_index].push(*pcm);
        }

        self.packet_count += 1;

        if self.packet_count > 25 {
            self.balance(false);
            self.packet_count = 0;
        }
    }

    // Adds a user to the index of uids if they haven't already been added
    // and they are not in the disallowed id list.
    fn user_join(&mut self, client: &ClientConnect) {
        let uid = client.user_id.0;
        let ssrc = client.audio_ssrc;

        if self.uid_map.contains_key(&ssrc) {
            return;
        }
        self.uid_map.insert(ssrc, uid);

        // `get_channel` will automatically allocate if not disallowed so we can just
        // dispatch to it after parsing the uid.
        self.get_channel(uid);
    }

    // Architus requires an ssrc <-> uid mapping before it can record anyone's audio.
    // As most people will have joined the VC before architus has, the main way
    // architus will gain that mapping information is from a speaking state update.
    fn speaking_update(&mut self, ssrc: u32, uid: u64) {
        if self.uid_map.contains_key(&ssrc) {
            return;
        }
        self.uid_map.insert(ssrc, uid);
        self.get_channel(uid);
    }

    // Handle a user leaving the voice channel.
    // Remove their mapping from the `uid_map`
    fn user_leave(&mut self, client: &ClientDisconnect) {
        let uid = client.user_id.0;
        let ssrcs = self
            .uid_map
            .iter()
            .filter(|(_, v)| **v == uid)
            .map(|(k, _)| k.clone())
            .collect::<Vec<_>>();
        for ssrc in ssrcs {
            self.uid_map.remove(&ssrc);
        }
    }

    // Checks whether an id is disallowed.
    fn disallowed(&self, id: u64) -> bool {
        for u in self.disallowed_ids.iter() {
            if *u == id {
                return true;
            }
        }

        if id == self.bot_id {
            true
        } else {
            false
        }
    }

    // Returns total number of bytes recorded.
    pub fn recording_size(&self) -> u64 {
        let words: u64 = self
            .audio_channels
            .iter()
            .map(|c| c.len() as u64)
            .sum::<u64>();
        words * 2
    }

    // Get which channel a user is mapped to.
    // If user is disallowed, return None.
    // Add user if not already indexed.
    fn get_channel(&mut self, uid: u64) -> Option<usize> {
        if self.disallowed(uid) {
            return None;
        }

        for (i, e) in self.user_list.iter().enumerate() {
            if *e == uid {
                return Some(i);
            }
        }

        let index = self.user_list.len();
        self.user_list.push(uid);
        self.audio_channels.push(Vec::new());
        self.last_sequence.push(Wrapping(0));
        Some(index)
    }

    // Balances out all of the audio channels to the same length as a basic
    // synchronization strategy.
    // Equalize tells the balance function wether each channel should be
    // comletely synchronized or just get them roughly equal if a channel
    // is falling behind.
    fn balance(&mut self, equalize: bool) {
        let largest: usize = self
            .audio_channels
            .iter()
            .fold(0, |acc, c| max(acc, c.len()));

        // 23,000 is roughly a quarter second of audio data.
        let max_diff: usize = if equalize { 0 } else { 23_000 };
        for c in self.audio_channels.iter_mut() {
            if largest - c.len() > max_diff {
                for _ in 0..(largest - c.len()) {
                    c.push(0x0000);
                }
            }
        }
    }
}
