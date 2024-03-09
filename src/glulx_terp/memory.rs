use std::{io::Cursor, ops::{Deref, DerefMut}};

use binread::{BinRead, BinReaderExt};


#[derive(BinRead, Debug)]
pub struct Version {
    pub major: u16,
    pub minor: u8,
    pub patch: u8,
}

#[derive(BinRead, Debug)]
#[br(magic = b"Glul")]
#[br(big)]
pub struct Header {
    pub version: Version,
    pub ram_start: u32,
    pub ext_start: u32,
    pub end_mem: u32,
    pub stack_size: u32,
    pub start_func: u32,
    pub decoding_tree: u32,
    pub checksum: u32,
}

#[derive(Debug)]
pub enum MemoryError {
    NotEnoughData(usize),
    BadChecksum
}

pub struct Memory {
    raw: Vec<u8>,
    start_ram_address: u32
}

impl Deref for Memory {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.raw
    }
}

impl DerefMut for Memory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.raw
    }
}

impl Memory {
    pub fn new(raw: Vec<u8>) -> Result<Self, MemoryError> {
        if raw.len() < 36 { return Err(MemoryError::NotEnoughData(raw.len())) }

        let mut memory = Self {
            raw,
            start_ram_address: 0
        };
        
        memory.start_ram_address = memory.get_header().expect("Bad file header").ram_start;

        Ok(memory)
    } 

    // Specials
    pub fn get_header(&self) -> Result<Header, binread::Error> {
        Cursor::new(&self.raw).read_be()
    }

    fn add_ram_offset(&self, value: u32) -> u32 {
        self.start_ram_address.wrapping_add(value)
    }

    pub fn as_cursor(&self) -> Cursor<&Vec<u8>> {
        Cursor::new(&self.raw)
    }

    // Getters
    pub fn get_u8(&self, pos: u32) -> u8 {
        // TODO: Error handling
        self[pos as usize]
    }

    pub fn get_u16(&self, pos: u32) -> u16 {
        // TODO: Error handling
        let pos = pos as usize;
        u16::from_be_bytes(self[pos..pos+2].try_into().unwrap())
    }

    pub fn get_u32(&self, pos: u32) -> u32 {
        // TODO: Error handling
        let pos = pos as usize;
        u32::from_be_bytes(self[pos..pos+4].try_into().unwrap())
    }

    pub fn get_ram_u8(&self, pos: u32) -> u8 {
        self.get_u8(self.add_ram_offset(pos))
    }

    pub fn get_ram_u16(&self, pos: u32) -> u16 {
        self.get_u16(self.add_ram_offset(pos))
    }

    pub fn get_ram_u32(&self, pos: u32) -> u32 {
        self.get_u32(self.add_ram_offset(pos))
    }

    // Setters
    pub fn set_u8(&mut self, pos: u32, value: u8) {
        // TODO: Error handling
        let pos = pos as usize;
        self[pos] = value ;
    }

    pub fn set_u16(&mut self, pos: u32, value: u16) {
        // TODO: Error handling
        let pos = pos as usize;
        self[pos..pos+2].copy_from_slice(&value.to_be_bytes());
    }

    pub fn set_u32(&mut self, pos: u32, value: u32) {
        // TODO: Error handling
        let pos = pos as usize;
        self[pos..pos+4].copy_from_slice(&value.to_be_bytes());
    }

    pub fn set_ram_u8(&mut self, pos: u32, value: u32) {
        self.set_u8(self.add_ram_offset(pos), value as u8)
    }

    pub fn set_ram_u16(&mut self, pos: u32, value: u32) {
        self.set_u16(self.add_ram_offset(pos), value as u16)
    }

    pub fn set_ram_u32(&mut self, pos: u32, value: u32) {
        self.set_u32(self.add_ram_offset(pos), value)
    }
}