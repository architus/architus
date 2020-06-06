use log::{debug, info, warn};
use std::cmp::max;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::net::TcpStream;

use crate::zipper::*;
use audiopus::{coder::Decoder, packet, Channels, SampleRate};
use tempfile::tempdir;

const SAMPLE_RATE: SampleRate = SampleRate::Hz48000;

/// Types of packets that we might receive from the gateway.
#[derive(Debug, PartialEq, Eq)]
enum PacketType {
    Data,
    UserJoin,
    Metadata,
    End,
    Checkup,
    Invalid(u8),
}

impl From<u8> for PacketType {
    fn from(b: u8) -> Self {
        match b {
            0 => PacketType::Data,
            1 => PacketType::UserJoin,
            3 => PacketType::Metadata,
            4 => PacketType::End,
            5 => PacketType::Checkup,
            _ => PacketType::Invalid(b),
        }
    }
}

enum ResponseCode {
    Success,
    DirectoryFailure,
}

impl ResponseCode {
    fn byte(&self) -> u8 {
        match self {
            ResponseCode::Success => 0x0,
            ResponseCode::DirectoryFailure => 0x1,
        }
    }
}

/// Stores all relevant state about recording of voice channel.
pub struct WAVReceiver {
    /// User ids that will not be included in the recording.
    /// This is necessary for complying with the discord privacy
    /// policy.
    disallowed_ids: Vec<u64>,

    /// Holds the bots id so that it can be excluded from any
    /// recording.
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

    // Give everyone their own decoder. This might help out with making FEC
    // actually work. It will also make the decoders work if some people are
    // using stereo and others are using mono even though their all supposed
    // to be encoded as stereo. I'm not sure if that's happening or not.
    decoders: HashMap<(u64, Channels), Decoder>,

    // Holds the last sequence number sent by each channel.
    last_sequence: Vec<u16>,
}

impl WAVReceiver {
    pub fn new() -> Self {
        Self {
            disallowed_ids: Vec::new(),
            bot_id: 0,
            audio_channels: Vec::new(),
            packet_count: 0,
            user_list: Vec::new(),
            decoders: HashMap::new(),
            last_sequence: Vec::new(),
        }
    }

    /// Main loop for recording voice data. Will do it's darndest to handle errors properly.
    /// The only things that can cause it to error out without properly recording are a
    /// dropped TCP connection and failure to send required fields: bot id, disallowed ids.
    pub fn handle(mut self, mut connection: TcpStream) {
        let mut buffer: [u8; 4096] = [0; 4096];

        loop {
            let okay = match connection.read(&mut buffer) {
                Ok(_) => PacketType::from(buffer[0]) == PacketType::Metadata,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted || e.kind() == ErrorKind::TimedOut {
                        false
                    } else {
                        warn!("Connection errored out.");
                        return;
                    }
                }
            };
            if okay {
                break;
            }
        }

        self.bot_id = u64::from_be_bytes(buffer[1..9].try_into().expect("why wouldn't this work"));
        let num_disallowed =
            u16::from_be_bytes(buffer[9..11].try_into().expect("Why wouldn't this work"));
        let mut num_parsed_ids = 0;
        debug!("Parsing {} disallowed ids", num_disallowed);

        while num_parsed_ids < num_disallowed {
            match connection.read(&mut buffer) {
                Ok(_) => {}
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted || e.kind() == ErrorKind::TimedOut {
                        continue;
                    } else {
                        return;
                    }
                }
            }

            let mut i = 0;
            while i < 4096 && num_parsed_ids < num_disallowed {
                self.disallowed_ids.push(u64::from_be_bytes(
                    buffer[i..(i + 8)]
                        .try_into()
                        .expect("Why wouldn't this work"),
                ));
                i += 8;
                num_parsed_ids += 1;
            }
        }

        debug!("Received id: {}", self.disallowed_ids[0]);

        if num_disallowed as usize != self.disallowed_ids.len() {
            warn!("Did not get all disallowed ids");
            return;
        }

        info!(
            "Entering main loop for {}",
            connection.peer_addr().expect("Peer not found")
        );
        'record_loop: loop {
            match connection.read(&mut buffer) {
                Ok(nb) => nb,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted || e.kind() == ErrorKind::TimedOut {
                        continue;
                    } else {
                        warn!("Errored out of record loop with bad connection.");
                        break 'record_loop;
                    }
                }
            };

            match PacketType::from(buffer[0]) {
                PacketType::Data => {
                    let payload_size =
                        u16::from_be_bytes(buffer[1..3].try_into().expect("Should always work"));
                    self.handle_data(&buffer[(3)..(13usize + payload_size as usize)]);
                }
                PacketType::End => {
                    self.close_connection(&mut connection);
                    break 'record_loop;
                }
                PacketType::UserJoin => {
                    self.user_join(&buffer[1..9]);
                }
                PacketType::Checkup => {
                    self.check_up(&mut connection);
                }
                PacketType::Metadata => {
                    // This really shouldn't happen.
                    info!("Got metadata packet during main loop");
                }
                PacketType::Invalid(b) => {
                    // Can't do anything with this packet so just dump it.
                    warn!("Got invalid packet type during mainloop: {}", b);
                }
            }
        }
    }

    // Takes an rtp packet and adds it to the appropriate audio channel
    // Packet should be formatted as:
    //
    // [  0x00, size0, size1,
    //    uid0,  uid1,  uid2,  uid3,
    //    uid4,  uid5,  uid6,  uid7,
    //    seq0,  seq1,
    //              --- payload bytes ---
    // ]
    //
    // where the method recieves everything after the size1.
    fn handle_data(&mut self, packet_bytes: &[u8]) {
        let uid = u64::from_be_bytes(packet_bytes[0..8].try_into().expect("Should always work."));
        let seq = u16::from_be_bytes(packet_bytes[8..10].try_into().expect("Should always work."));

        // Find which channel the uid belongs to. This should only ever match
        // to None when the uid is in the disallowed list.
        let channel_index = match self.get_channel(uid) {
            Some(i) => i,
            None => {
                return;
            }
        };

        // Drop packet if future packets have already been processed.
        if self.last_sequence[channel_index] < seq
            || (seq < 10 && self.last_sequence[channel_index] > 65530)
        {
            self.last_sequence[channel_index] = seq;
        } else {
            return;
        }

        // Find out how many channels there are and get relevant decoder.
        let channels = match packet::nb_channels(&packet_bytes[10..]) {
            Ok(nc) => nc,
            Err(_) => {
                warn!("Bad number of channels in packet");
                return;
            }
        };

        let decoder = self
            .decoders
            .entry((uid, channels))
            .or_insert_with(|| Decoder::new(SAMPLE_RATE, channels).unwrap());

        // The opus payload should never be larger than 3840 bytes so
        // this array should be fine for loading the data.
        let mut pcm: [i16; 3840] = [0; 3840];
        let num_bytes = decoder.decode(Some(&packet_bytes[10..]), &mut pcm[..], false);
        let num_bytes = match num_bytes {
            Ok(n) => n,
            Err(e) => {
                warn!("Bad packet: {:?}", e);
                return;
            }
        };

        // Add pcm data to vector.
        let offset: usize;
        if channels == Channels::Mono {
            offset = 1;
        } else {
            offset = 2;
        }
        let mut i = 0;
        while i < num_bytes {
            self.audio_channels[channel_index].push(pcm[i]);
            i += offset;
        }

        self.packet_count += 1;

        if self.packet_count > 25 {
            self.balance(false);
            self.packet_count = 0;
        }
    }

    // Returns which channel a uid gets mapped to.
    // If user is disallowed, return None.
    // If user not indexed already, give them an index.
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
        self.last_sequence.push(0);
        return Some(index);
    }

    // Adds a user to the index of uids if they havne't already been added
    // and they are not in the disallowed id list.
    //
    // [  0x01,
    //    uid0,  uid1,  uid2,  uid3,
    //    uid4,  uid5,  uid6,  uid7 ]
    //
    // Where the method is passed everything after the 0x01.
    fn user_join(&mut self, data: &[u8]) {
        let uid = u64::from_be_bytes(data[0..8].try_into().expect("Should always work"));

        // `get_channel` will automatically allocate if not disallowed so we can just
        // dispatch to it after parsing the uid.
        self.get_channel(uid);
    }

    // Lets the bot know how many bytes have been recorded so far.
    //
    // Called by a packet of [0x05]
    fn check_up(&mut self, connection: &mut TcpStream) {
        debug!("In check up function");
        let bytes = self.recording_size();
        loop {
            match connection.write(&bytes.to_be_bytes()) {
                Ok(_) => break,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted || e.kind() == ErrorKind::TimedOut {
                        continue;
                    } else {
                        warn!("Connection failed trying to send stats.");
                        break;
                    }
                }
            }
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

    // Close connection by sending how many bytes were read
    fn close_connection(mut self, connection: &mut TcpStream) {
        let bytes = self.recording_size();
        debug!("Recorded: {}", bytes);
        self.balance(true);
        let dir = match tempdir() {
            Ok(d) => d,
            Err(_) => match connection.write(&[ResponseCode::DirectoryFailure.byte()]) {
                Ok(_) => return,
                Err(_) => {
                    warn!("Tcp failed sending error");
                    return;
                }
            },
        };

        let (url, pw) = match zip(self.audio_channels, dir) {
            Ok(s) => s,
            Err(e) => match connection.write(&[e.byte()]) {
                Ok(_) => return,
                Err(_) => {
                    warn!("Tcp failed sending error");
                    return;
                }
            },
        };
    }

    // Returns total number of bytes recorded.
    fn recording_size(&self) -> u64 {
        let words: u64 = self
            .audio_channels
            .iter()
            .map(|c| c.len() as u64)
            .sum::<u64>();
        words * 2
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
