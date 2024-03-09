pub mod memory;
mod operations;
use std::io::Read;
use self::{memory::{Memory, MemoryError}, operations::Operation};

pub struct GlulxTerp {
    memory: Memory,
    pc: u32
}

#[derive(Debug)]
pub enum Errors {
    IOError(std::io::Error),
    MemoryError(memory::MemoryError),
    BinRead(binread::Error),
    FetchOperation(String)
}

impl GlulxTerp {
    pub fn from_reader<T: Read>(source: &mut T) -> Result<Self, Errors> {
        let mut raw: Vec<u8> = Vec::new();

        source.read_to_end(&mut raw).map_err(Errors::IOError)?;
        
        let memory = Memory::new(raw).map_err(Errors::MemoryError)?;
        let header = memory.get_header().map_err(Errors::BinRead)?;

        { // Check if the header's checksum is valid.
            const CHECKSUM_POS: u32 = 8*4;
            let mut index = CHECKSUM_POS;
            let valid_checksum: u32 = memory.get_u32(index);
            let mut checksum = 0u32;
            let length = memory.len() as u32;

            index = 0;
            while index < CHECKSUM_POS {
                checksum = checksum.wrapping_add(memory.get_u32(index));
                index += 4;
            }
            index = CHECKSUM_POS+4;
            while index < length {
                checksum = checksum.wrapping_add(memory.get_u32(index));
                index += 4;
            }

            if checksum != valid_checksum {
                return Err(Errors::MemoryError(MemoryError::BadChecksum))
            }
        }
        
        Ok(Self {
            memory,
            pc: header.start_func
        })
    }

    pub fn step(&mut self) -> Result<(), Errors> {
        print!("{:X}: ", self.pc);
        let operation = Operation::fetch(&mut self.memory.as_cursor(), self.pc)?;
        dbg!(operation);
        todo!("Execute the operation");
        Ok(())
    }

    pub fn run(&mut self) {
        loop {
            let result = self.step();
            if let Err(err) = result {
                eprintln!("{:?}", err);
                break;
            }
        }
    }
}