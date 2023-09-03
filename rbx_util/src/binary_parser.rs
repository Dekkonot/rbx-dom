//! Quick and dirty binary parser that doesn't do any extra processing on the
//! chunks it reads.
use serde::Serialize;

use std::io;

const MAGIC_NUMBER: &[u8] = b"<roblox!\x89\xff\x0d\x0a\x1a\x0a";
const MAGIC_LEN: usize = MAGIC_NUMBER.len();
const END_CHUNK: &[u8] = b"END\0";

const SUPPORTED_VERSION: u16 = 0;

const ZSTD_MAGIC_NUMBER: &[u8] = b"28\xb5\x2f\xfd";

#[derive(Debug, Serialize)]
pub struct BinaryFile {
    class_count: u32,
    instance_count: u32,
    reserved: String,
    chunks: Vec<Chunk>,
}

#[derive(Debug, Serialize)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
}

#[derive(Debug, Serialize)]
pub struct Chunk {
    name: String,
    reserved: String,
    compression_type: CompressionType,
    data: String,
}

pub fn parse_file<R: io::Read>(reader: &mut R) -> anyhow::Result<BinaryFile> {
    let magic_number = reader.read_n::<MAGIC_LEN>()?;
    assert_eq!(MAGIC_NUMBER, magic_number, "magic number did not match");

    let version = u16::from_le_bytes(reader.read_n()?);
    assert_eq!(
        SUPPORTED_VERSION, version,
        "version {version} is unsupported"
    );

    let class_count = u32::from_le_bytes(reader.read_n()?);
    let instance_count = u32::from_le_bytes(reader.read_n()?);
    let header_reserved = reader.read_n::<8>()?;

    let mut chunks = Vec::new();

    loop {
        let id = reader.read_n::<4>()?;
        let compressed_len = u32::from_le_bytes(reader.read_n()?);
        let decompressed_len = u32::from_le_bytes(reader.read_n()?);
        let reserved = reader.read_n::<4>()?;

        let (compression_type, data) = if compressed_len == 0 {
            let mut data = vec![0; decompressed_len as usize];
            reader.read_exact(&mut data)?;
            (CompressionType::None, data)
        } else {
            let mut data = vec![0; compressed_len as usize];
            reader.read_exact(&mut data)?;
            if &data[0..4] == ZSTD_MAGIC_NUMBER {
                (
                    CompressionType::Zstd,
                    zstd::bulk::decompress(&data, decompressed_len as usize)?,
                )
            } else {
                (
                    CompressionType::Lz4,
                    lz4_flex::decompress(&data, decompressed_len as usize)?,
                )
            }
        };

        chunks.push(Chunk {
            name: slice_to_string(&id),
            data: slice_to_hex_string(&data),
            reserved: slice_to_hex_string(&reserved),
            compression_type,
        });

        if &id == END_CHUNK {
            break;
        }
    }

    Ok(BinaryFile {
        class_count,
        instance_count,
        reserved: slice_to_hex_string(&header_reserved),
        chunks,
    })
}

fn slice_to_string(slice: &[u8]) -> String {
    match std::str::from_utf8(slice) {
        Ok(str) => str.to_string(),
        Err(_) => slice_to_hex_string(slice),
    }
}

fn slice_to_hex_string(slice: &[u8]) -> String {
    let mut buff = String::with_capacity(slice.len() * 3);
    for n in slice {
        buff.push_str(&format!("{n:02X} "));
    }
    buff.pop();
    buff
}

trait Reader: io::Read {
    fn read_n<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut data = [0; N];
        self.read_exact(&mut data)?;

        Ok(data)
    }
}

impl<R: io::Read> Reader for R {}
