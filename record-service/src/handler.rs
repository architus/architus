use log::{debug, info, warn};
use std::convert::TryInto;
use std::io::prelude::*;
use std::io::ErrorKind;
use std::net::TcpStream;

use bimap::hash::BiHashMap;
use rtp_rs::RtpReader;

use opus::{Channels, Decoder};

const SAMPLE_RATE: u32 = 48_000;
const CHANNELS: Channels = Channels::Stereo;

/// Types of packets that we might receive from the gateway.
#[derive(Debug, PartialEq, Eq)]
pub enum PacketType {
    Data,
    UserJoin,
    UserLeave,
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
            2 => PacketType::UserLeave,
            3 => PacketType::Metadata,
            4 => PacketType::End,
            5 => PacketType::Checkup,
            _ => PacketType::Invalid(b),
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

    // This is the number of channels that will exist in the final
    // WAV file. It is equivalent to the number of non-blocked users
    // that have sent voice data in the channel while the bot is
    // listening.
    num_channels: u32,

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

    // While the RTP does it's best to ensure that SSRCs are unique, it is not
    // guaranteed. One user could disconnect with one SSRC and then another
    // could reconnect using that same SSRC if luck is really against us.
    // This bi-directional hashmap allows us to figure out which SSRC belongs
    // to which user and disassociate them when the user leaves.
    ssrc_to_user: BiHashMap<u32, u64>,

    // This will end up being a user_id of every user that gets recorded.
    // The index of the user in this list will equate to which channel
    // that user is in the WAV file.
    user_list: Vec<u64>,

    // Decodes opus data into PCM
    decoder: Decoder,
}

impl WAVReceiver {
    pub fn new() -> Self {
        Self {
            disallowed_ids: Vec::new(),
            bot_id: 0,
            num_channels: 0,
            audio_channels: Vec::new(),
            packet_count: 0,
            ssrc_to_user: BiHashMap::new(),
            user_list: Vec::new(),
            decoder: Decoder::new(SAMPLE_RATE, CHANNELS).expect("Opus failed"),
        }
    }

    /// Main loop for recording voice data. Will do it's darndest to handle errors properly.
    /// The only things that can cause it to error out without properly recording are a
    /// dropped TCP connection and failure to send required fields: bot id, disallowed ids.
    pub fn handle(&mut self, mut connection: TcpStream) {
        let mut buffer: [u8; 4096] = [0; 4096];

        loop {
            let okay = match connection.read(&mut buffer) {
                Ok(num_bytes) => PacketType::from(buffer[0]) == PacketType::Metadata,
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
            while i < 64 && num_parsed_ids < num_disallowed {
                self.disallowed_ids.push(u64::from_be_bytes(
                    buffer[i..(i + 8)]
                        .try_into()
                        .expect("Why wouldn't this work"),
                ));
                i += 8;
                num_parsed_ids += 1;
            }
        }

        if num_disallowed as usize != self.disallowed_ids.len() {
            warn!("Did not get all disallowed ids");
            return;
        }

        info!(
            "Entering main loop for {}",
            connection.peer_addr().expect("Peer not found")
        );
        'record_loop: loop {
            let num_bytes = match connection.read(&mut buffer) {
                Ok(nb) => nb,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted || e.kind() == ErrorKind::TimedOut {
                        continue;
                    } else {
                        break 'record_loop;
                    }
                }
            };

            let mut index: usize = 0;
            while index < num_bytes {
                let pt = PacketType::from(buffer[index]);
                match pt {
                    PacketType::Data => {
                        let payload_size = u16::from_be_bytes(
                            buffer[(index + 1usize)..(index + 3usize)]
                                .try_into()
                                .expect("Should always work"),
                        );
                        self.handle_data(
                            &buffer[(index + 2usize)..(index + 14usize + payload_size as usize)],
                        );
                        index += 14 + payload_size as usize;
                    }
                    PacketType::End => {
                        self.close_connection(&mut connection);
                        break 'record_loop;
                    }
                    PacketType::UserJoin => {
                        self.user_join(&buffer[1..num_bytes]);
                        index += 13;
                    }
                    PacketType::UserLeave => {
                        self.user_leave(&buffer[1..num_bytes]);
                        index += 5;
                    }
                    PacketType::Checkup => {
                        self.check_up(&mut connection);
                        index += 1;
                    }
                    PacketType::Metadata => {
                        info!("Got metadata packet during main loop");
                        index += 11;
                    }
                    PacketType::Invalid(b) => {
                        warn!("Got invalid packet type during mainloop: {}", b);
                        index = usize::MAX;
                    }
                }
            }
        }
    }

    // Takes an rtp packet and adds it to the appropriate audio channel
    // Packet should be formatted as:
    //
    // [ 0x00,
    //          --- RTP Header (12 bytes) ---
    //          ---  Opus Data (n bytes)  ---
    // ]
    //
    // where the method recieves everything after the 0x00.
    fn handle_data(&mut self, packet_bytes: &[u8]) {
        // Parse the rtp packet from the bytes and return if it's invalid
        let packet = match RtpReader::new(packet_bytes) {
            Ok(p) => p,
            Err(_) => {
                warn!("Got invalid rtp packet of correct payload type.");
                return;
            }
        };

        // Can't do anything without a valid ssrc <-> uid mapping so
        // return if one is not found.
        let ssrc = packet.ssrc();
        let uid = match self.ssrc_to_user.get_by_left(&ssrc) {
            Some(u) => u,
            None => {
                info!("Got packet from ssrc not in map");
                return;
            }
        };

        // All of the decoded PCM data should be 3840 bytes as that is 20 ms of
        // 16 bit audio sampled at 48,000 Hz. So we need 1920 i16s as a buffer
        // to store the relevant pcm data. The FEC option is hardcoded to be
        // false as we are not able to make use of it. We would need to use a
        // separate decoder per channel and maintain sequence order for that
        // to work which adds lots of complexity and memory overhead that is
        // unnecessary for a microservice that can't really guarantee that
        // that additional overhead will actually improve service much.
        // The decode function can only ever return an Ok so using unwrap
        // should be fine.
        let mut pcm: [i16; 1920] = [0; 1920];
        let num_bytes = self
            .decoder
            .decode(packet.payload(), &mut pcm, false)
            .unwrap();

        // Find which channel the uid belongs to. This should never go down the
        // error branch.
        let channel_index = match self.get_channel(*uid) {
            Some(i) => i,
            None => {
                warn!("User in map but not user_list.");
                return;
            }
        };

        // Add pcm data to vector.
        let mut i = 0;
        while i < num_bytes {
            self.audio_channels[channel_index].push(pcm[i]);
            i += 2;
        }

        self.packet_count += 1;
    }

    // Returns which channel a uid gets mapped to.
    // Returns an error if the uid is not mapped to a channel
    fn get_channel(&self, uid: u64) -> Option<usize> {
        for (i, e) in self.user_list.iter().enumerate() {
            if *e == uid {
                return Some(i);
            }
        }

        None
    }

    // Stops recording bytes and does some close up stuff.
    // Packet format:
    //
    // 0x04
    //
    // This method does not require any data from the packet.
    fn close_connection(&mut self, connection: &mut TcpStream) {
        let words: usize = self.audio_channels.iter().map(|c| c.len()).sum::<usize>() * 2;
        info!("Ending connection with {} bytes", words);
        'stats_loop: loop {
            match connection.write(&words.to_be_bytes()) {
                Ok(_) => break,
                Err(e) => {
                    if e.kind() == ErrorKind::Interrupted || e.kind() == ErrorKind::TimedOut {
                        continue;
                    } else {
                        warn!("Connection failed trying to send stats.");
                        break 'stats_loop;
                    }
                }
            }
        }
    }

    // Adds a user to the ssrc <-> uid mapping. Packet should be formed as:
    //
    // [  0x01,
    //   ssrc1, ssrc2, ssrc3, ssrc4,
    //    uid1,  uid2,  uid3,  uid4,
    //    uid5,  uid6,  uid7,  uid8 ]
    //
    //   Where the method is passed everything after the 0x01.
    fn user_join(&mut self, data: &[u8]) {
        let ssrc = u32::from_be_bytes(data[0..4].try_into().expect("Should always work"));
        let uid = u64::from_be_bytes(data[4..12].try_into().expect("Should always work"));

        debug!("Adding {} <-> {}", ssrc, uid);

        // Add to ssrc map.
        self.ssrc_to_user.insert(ssrc, uid);

        // See if uid needs to be added to the user list
        match self.get_channel(uid) {
            Some(_) => {}
            None => {
                self.user_list.push(uid);
                self.audio_channels.push(Vec::new());
                self.num_channels += 1;
            }
        }
    }

    // Removes an ssrc from the ssrc <-> uid mapping. Packet should be formed as:
    //
    // [  0x02,
    //   ssrc1, ssrc2, ssrc3, ssrc4 ]
    //
    // And this method gets everything after the 0x02.
    fn user_leave(&mut self, data: &[u8]) {
        let ssrc = u32::from_be_bytes(data[0..4].try_into().expect("Should always work"));

        match self.ssrc_to_user.remove_by_left(&ssrc) {
            None => warn!("Tried to remove non-existant mapping"),
            _ => {}
        }
    }

    // Lets the bot know how many bytes have been recorded so far.
    //
    // Called by a packet of [5]
    fn check_up(&mut self, connection: &mut TcpStream) {
        let words: usize = self.audio_channels.iter().map(|c| c.len()).sum::<usize>() * 2;
        loop {
            match connection.write(&words.to_be_bytes()) {
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
    fn _disallowed(&self, id: u64) -> bool {
        for u in self.disallowed_ids.iter() {
            if *u == id {
                return true;
            }
        }

        false
    }
}

fn _split(word: i16) -> (u8, u8) {
    let high: u8 = (word.swap_bytes() & 0x00FF) as u8;
    let low: u8 = (word & 0x00FF) as u8;
    (high, low)
}
