use std::convert::TryInto;
use std::fs::File;
use std::io::{Read, Write};
use tempfile::TempDir;

use base64::encode;
use rand::prelude::*;
use std::process::Command;

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
        }
    }
}

/// This takes care of writing pcm to file, zipping, and sending to cdn.
pub fn zip(pcm: Vec<Vec<i16>>, dir: TempDir) -> Result<(String, String), PublishError> {
    let path = dir.path().join("recording.wav");
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(_) => return Err(PublishError::Writing),
    };
    write_wav(&pcm, &mut file)?;
    let pw = zip_file(&dir)?;
    drop(pcm);

    let mut zip: Vec<u8> = Vec::new();
    let mut f = match File::open(dir.path().join("rec.zip")) {
        Ok(f) => f,
        Err(_) => return Err(PublishError::Zipping),
    };

    match f.read_to_end(&mut zip) {
        Ok(_) => {}
        Err(_) => return Err(PublishError::Zipping),
    }

    let encoded_file = encode(zip);

    Ok((encoded_file, pw))
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

    let status = Command::new("zip")
        .current_dir(dir.path())
        .arg(format!("-p {}", pw))
        .arg("rec.zip")
        .arg("recording.wav")
        .status()
        .map_err(|_| PublishError::Zipping);

    match status {
        Ok(es) => {
            if !es.success() {
                return Err(PublishError::Zipping);
            }
        }
        Err(_) => return Err(PublishError::Zipping),
    }

    Ok(pw)
}
