use log::trace;
use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Write};
use tempfile::{tempdir, TempDir};

use rand::prelude::*;
use std::process::Command;

use futures::stream;
use manager::manager_client::ManagerClient;
use tokio::runtime::Runtime;
use tonic::Request;

pub mod manager {
    include!("/app/src/manager.rs");
}

const PW_ALPHABET: [char; 62] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l',
    'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '0', '1', '2', '3', '4',
    '5', '6', '7', '8', '9',
];

pub enum PublishError {
    Writing,
    Zipping,
    TooLarge,
    TooManyChannels,
    BigByteRate,
    Rpc,
    Tokio,
    DirectoryFailure,
}

impl PublishError {
    pub fn byte(&self) -> u8 {
        match self {
            PublishError::Writing => 0x2,
            PublishError::Zipping => 0x3,
            PublishError::TooLarge => 0x4,
            PublishError::TooManyChannels => 0x5,
            PublishError::BigByteRate => 0x6,
            PublishError::Rpc => 0x7,
            PublishError::Tokio => 0x8,
            PublishError::DirectoryFailure => 0x9,
        }
    }
}

/// This takes care of writing pcm to file, zipping, and sending to cdn.
pub fn zip(pcm: Vec<Vec<i16>>) -> Result<(String, String), PublishError> {
    // Get temporary directory to write wav and zip file to.
    let dir = match tempdir() {
        Ok(d) => d,
        Err(_) => return Err(PublishError::DirectoryFailure),
    };

    // Open up the Wav file that will be written to and then write
    // the PCM data to it.
    let path = dir.path().join("recording.wav");
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(_) => return Err(PublishError::Writing),
    };
    write_wav(&pcm, &mut file)?;
    trace!("Wrote zip");
    drop(pcm);

    // Zip the Wav file and get the password used to zip the file.
    let pw = zip_file(&dir)?;

    // Read the zip file back into memory so it can be sent to the
    // manager.
    let mut zip: Vec<u8> = Vec::new();
    let mut f = match File::open(dir.path().join("rec.zip")) {
        Ok(f) => f,
        Err(_) => return Err(PublishError::Zipping),
    };
    match f.read_to_end(&mut zip) {
        Ok(_) => {}
        Err(_) => return Err(PublishError::Zipping),
    }
    drop(f);

    // Make a tokio runtime to do async stuff and open up a connection
    // to the manager gRPC.
    let mut rt = match Runtime::new() {
        Ok(r) => r,
        Err(_) => return Err(PublishError::Tokio),
    };
    let mut manager =
        match rt.block_on(async { ManagerClient::connect("http://manager:50051").await }) {
            Ok(c) => c,
            Err(_) => return Err(PublishError::Rpc),
        };

    // Send the data to the manager in chunks.
    let mut packets = Vec::new();
    for bytes in zip.chunks(32768) {
        packets.push(manager::File {
            location: "recordings".to_string(),
            name: "".to_string(),
            filetype: "zip".to_string(),
            file: bytes.to_vec(),
        });
    }
    let request = Request::new(stream::iter(packets));
    match rt.block_on(async { manager.publish_file(request).await }) {
        Ok(resp) => return Ok((resp.into_inner().url, pw)),
        Err(_) => return Err(PublishError::Rpc),
    }
}

// This method assumes that the pcm data has been processed so that all of the channels
// have the same number of bytes.
fn write_wav(pcm: &Vec<Vec<i16>>, file: &mut File) -> Result<(), PublishError> {
    let data_chunk_size: u32 = match (pcm.len() * pcm[0].len() * 2).try_into() {
        Ok(s) => s,
        Err(_) => return Err(PublishError::TooLarge),
    };
    let channels: u16 = match pcm.len().try_into() {
        Ok(nc) => nc,
        Err(_) => return Err(PublishError::TooManyChannels),
    };
    let align: u16 = channels * 2;
    let byte_rate: u32 = match (48000 * channels * 2).try_into() {
        Ok(br) => br,
        Err(_) => return Err(PublishError::BigByteRate),
    };

    // All of the header info for a WAV file.
    file.write_all(b"RIFF").map_err(|_| PublishError::Writing)?;
    file.write_all(&(data_chunk_size + 32).to_le_bytes())
        .map_err(|_| PublishError::Writing)?;
    file.write_all(b"WAVE").map_err(|_| PublishError::Writing)?;
    file.write_all(b"fmt ").map_err(|_| PublishError::Writing)?;
    file.write_all(b"\x10\x00\x00\x00")
        .map_err(|_| PublishError::Writing)?;
    file.write_all(b"\x01\x00")
        .map_err(|_| PublishError::Writing)?;
    file.write_all(&channels.to_le_bytes())
        .map_err(|_| PublishError::Writing)?;
    file.write_all(b"\x80\xBB\x00\x00")
        .map_err(|_| PublishError::Writing)?;
    file.write_all(&byte_rate.to_le_bytes())
        .map_err(|_| PublishError::Writing)?;
    file.write_all(&align.to_le_bytes())
        .map_err(|_| PublishError::Writing)?;
    file.write_all(b"\x10\x00")
        .map_err(|_| PublishError::Writing)?;
    file.write(b"data").map_err(|_| PublishError::Writing)?;
    file.write(&data_chunk_size.to_le_bytes())
        .map_err(|_| PublishError::Writing)?;

    for i in 0..pcm[0].len() {
        for c in pcm.iter() {
            file.write(&c[i].to_le_bytes())
                .map_err(|_| PublishError::Writing)?;
        }
    }

    Ok(())
}

fn zip_file(dir: &TempDir) -> Result<String, PublishError> {
    let mut rng = thread_rng();
    let mut pw = String::with_capacity(15);
    for _ in 0..15 {
        let i: usize = rng.gen::<usize>() % 62;
        pw.push(PW_ALPHABET[i]);
    }
    trace!("Passowrd: {}", pw);

    let status = Command::new("zip")
        .arg(format!("-P {}", pw))
        .arg(dir.path().join("rec.zip"))
        .arg(dir.path().join("recording.wav"))
        .status()
        .map_err(|_| PublishError::Zipping);

    match status {
        Ok(es) => {
            if !es.success() {
                return Err(PublishError::Zipping);
            }
        }
        Err(e) => return Err(e),
    }

    Ok(pw)
}
