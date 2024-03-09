use std::{fmt, io::{Read, Seek}};

use byteorder::{BigEndian, ReadBytesExt};
use num_enum::{IntoPrimitive, TryFromPrimitive};

use super::Errors;

#[derive(Debug)]
pub struct Operation {
    pub code: OPCode,
    pub operands: Vec<Operand>
}

impl Operation {
    pub fn fetch<R: Read + Seek>(reader: &mut R, pos: u32) -> Result<Operation, Errors> {
        reader.seek(std::io::SeekFrom::Start(pos as u64)).map_err(Errors::IOError)?;
        
        let mut value = reader.read_u8().map_err(Errors::IOError)? as u32;
        if (value & 0x80) != 0 {
            value = (value << 8) + reader.read_u8().map_err(Errors::IOError)? as u32;
            if (value & 0xC000) == 0xC000 {
                value = (value << 16) + (reader.read_u16::<BigEndian>().map_err(Errors::IOError)? as u32);
                value -= 0xC100_0000;
            } else {
                value -= 0x8000;
            }
        }

        let operation = OPCode::try_from(value).map_err(|_| Errors::FetchOperation(format!("Couldn't convert '{value:X?}' into OPCode")))?;
        Ok(Operation {
            code: operation,
            operands: Operand::fetch_for_opcode(reader, operation)?
        })
    }
}


#[derive(Debug)]
pub struct Operand {
    pub operand_mode: OperandMode,
    pub addressing_mode: OperandAddressingMode,
}

impl Operand {
    pub fn fetch_for_opcode<R: Read>(reader: &mut R, operation: OPCode) -> Result<Vec<Operand>, Errors> {
        let operand_types = operation.get_operand_types();
        let nb_operands = (operand_types.0 + operand_types.1) as usize;
    
        let mut types: Vec<OperandMode> = Vec::with_capacity(nb_operands);
    
        // Not a big fan of this way of handling it 
        match operation { 
            OPCode::CATCH => {
                types.extend((0..operand_types.1).map(|_| OperandMode::Store));
                types.extend((0..operand_types.0).map(|_| OperandMode::Load));
            }
            _ => {
                types.extend((0..operand_types.0).map(|_| OperandMode::Load));
                types.extend((0..operand_types.1).map(|_| OperandMode::Store));
            }
        };

        let mut operands_raw: Vec<u8> = Vec::with_capacity(nb_operands+1);
        let mut operands: Vec<Operand> = Vec::with_capacity(nb_operands);
        
        for _ in 0..((nb_operands as f32 / 2.0).ceil() as u32) {
            let modes = reader.read_u8().map_err(Errors::IOError)?;
            operands_raw.push(modes & 0x0F);
            operands_raw.push((modes & 0xF0) >> 4)
        }

        for (raw_mode, operand_type) in operands_raw.iter().take(nb_operands).zip(types) {
            operands.push(Operand { 
                operand_mode: operand_type, 
                addressing_mode: OperandAddressingMode::try_fetch(reader, *raw_mode).map_err(Errors::BinRead)? 
            });
        } 

        Ok(operands)
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OperandMode {
    Load,
    Store
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
#[repr(u8)]
pub enum OperandAddressingMode {
    #[default]
    ConstantZero,
    Constant1Byte(u32),
    Constant2Bytes(u32),
    Constant4Bytes(u32),

    __Unused1,

    ContentOfAddress1Byte(u32),
    ContentOfAddress2Bytes(u32),
    ContentOfAddress4Bytes(u32),

    Stack,

    CallFrameLocalAtAddress1Byte(u32),
    CallFrameLocalAtAddress2Bytes(u32),
    CallFrameLocalAtAddress4Bytes(u32),

    __Unused2,

    ContentOfRAMAddress1Byte(u32),
    ContentOfRAMAddress2Bytes(u32),
    ContentOfRAMAddress4Bytes(u32),
}

impl OperandAddressingMode {
    fn try_fetch<R: Read>(reader: &mut R, mode: u8) -> Result<OperandAddressingMode, binread::Error> {
        match mode {
            0 => Ok(Self::ConstantZero),
            1 => Ok(Self::Constant1Byte(reader.read_u8()? as u32)),
            2 => Ok(Self::Constant2Bytes(reader.read_u16::<BigEndian>()? as u32)),
            3 => Ok(Self::Constant4Bytes(reader.read_u32::<BigEndian>()?)),

            4 => Ok(Self::__Unused1),

            5 => Ok(Self::ContentOfAddress1Byte(reader.read_u8()? as u32)),
            6 => Ok(Self::ContentOfAddress2Bytes(reader.read_u16::<BigEndian>()? as u32)),
            7 => Ok(Self::ContentOfAddress4Bytes(reader.read_u32::<BigEndian>()?)),

            8 => Ok(Self::Stack),

            9 => Ok(Self::CallFrameLocalAtAddress1Byte(reader.read_u8()? as u32)),
            0xA => Ok(Self::CallFrameLocalAtAddress2Bytes(reader.read_u16::<BigEndian>()? as u32)),
            0xB => Ok(Self::CallFrameLocalAtAddress4Bytes(reader.read_u32::<BigEndian>()?)),

            0xC => Ok(Self::__Unused2),

            0xD => Ok(Self::ContentOfRAMAddress1Byte(reader.read_u8()? as u32)),
            0xE => Ok(Self::ContentOfRAMAddress2Bytes(reader.read_u16::<BigEndian>()? as u32)),
            0xF => Ok(Self::ContentOfRAMAddress4Bytes(reader.read_u32::<BigEndian>()?)),

            _ => Err(binread::error::Error::NoVariantMatch { pos: mode as u64 })
        }
    }
}

#[repr(u32)]
#[derive(Eq, PartialEq, IntoPrimitive, TryFromPrimitive, Copy, Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub enum OPCode {
    // 2.1. Integer Math
    ADD = 0x10,
    SUB,
    MUL,
    DIV,
    MOD,
    NEG,

    BITAND = 0x18,
    BITOR,
    BITXOR,
    BITNOT,
    SHIFTL,
    SSHIFTR,
    USHIFTR,

    // 2.2. Branches
    JUMP = 0x20,
    JZ   = 0x22,
    JNZ,
    JEQ,
    JNE,
    JLT,
    JGE,
    JGT,
    JLE,
    JLTU,
    JGEU,
    JGTU,
    JLEU,
    JUMPABS = 0x104,

    // 2.3. Moving Data
    COPY = 0x40,
    COPYS,
    COPYB,
    SEXS = 0x44,
    SEXB,

    // 2.4. Array Data
    ALOAD = 0x48,
    ALOADS,
    ALOADB,
    ALOADBIT,
    ASTORE,
    ASTORES,
    ASTOREB,
    ASTOREBIT,

    // 2.5. The Stack
    STKCOUNT = 0x50,
    STKPEEK,
    STKSWAP,
    STKROLL,
    STKCOPY,

    // 2.6. Functions
    CALL = 0x30,
    RETURN,
    TAILCALL = 0x34,
    CALLF = 0x160,
    CALLFI,
    CALLFII,
    CALLFIII,

    // 2.7. Continuations
    CATCH = 0x32,
    THROW,

    // 2.8. Memory Map
    GETMEMSIZE = 0x102,
    SETMEMSIZE,

    // 2.9. Memory Allocation Heap
    MALLOC = 0x178,
    MFREE,

    // 2.10. Game State
    QUIT = 0x120,
    VERIFY,
    RESTART,
    SAVE,
    RESTORE,
    SAVEUNDO,
    RESTOREUNDO,
    PROTECT,
    HASUNDO,
    DISCARDUNDO,

    // 2.11. Output
    GETIOSYS = 0x148,
    SETIOSYS,
    STREAMCHAR = 0x70,
    STREAMNUM,
    STREAMSTR,
    STREAMUNICHAR,
    GETSTRINGTBL = 0x140,
    SETSTRINGTBL,

    // 2.12. Floating-Point Math
    NUMTOF = 0x190,
    FTONUMZ,
    FTONUMN,
    CEIL = 0x198,
    FLOOR,
    FADD = 0x1A0,
    FSUB,
    FMUL,
    FDIV,
    FMOD,
    SQRT = 0x1A8,
    EXP,
    LOG,
    POW,
    SIN = 0x1B0,
    COS,
    TAN,
    ASIN,
    ACOS,
    ATAN,
    ATAN2,

    // 2.13. Double-Precision Math
    NUMTOD = 0x200,
    DTONUMZ,
    DTONUMN,
    FTOD,
    DTOF,
    DCEIL = 0x208,
    DFLOOR,
    DADD = 0x210,
    DSUB,
    DMUL,
    DDIV,
    DMODR,
    DMODQ,
    DSQRT = 0x218,
    DEXP,
    DLOG = 0x21A,
    DPOW,
    DSIN = 0x220,
    DCOS,
    DTAN,
    DASIN,
    DACOS,
    DATAN,
    DATAN2,

    // 2.14. Floating-Point Comparisons
    JFEQ = 0x1C0,
    JFNE,
    JFLT,
    JFLE,
    JFGT,
    JFGE,
    JISNAN = 0x1C8,
    JISINF,

    // 2.15. Double-Precision Comparisons
    JDEQ = 0x230,
    JDNE,
    JDLT,
    JDLE,
    JDGT,
    JDGE,
    JDISNAN = 0x238,
    JDISINF,

    // 2.16. Random Number Generator
    RANDOM = 0x110,
    SETRANDOM,

    // 2.17. Block Copy and Clear
    MZERO = 0x170,
    MCOPY,

    // 2.18. Searching
    LINEARSEARCH = 0x150,
    BINARYSEARCH,
    LINKEDSEARCH,

    // 2.19. Accelerated Functions
    ACCELFUNC = 0x180,
    ACCELPARAM,

    // 2.20. Miscellaneous
    NOP = 0x00,
    GESTALT = 0x100,
    DEBUGTRAP,
    GLK = 0x130,
}



impl OPCode {
    pub fn get_operand_types(self) -> (u8, u8) {
        match self {
            Self::STKSWAP |
            Self::QUIT |
            Self::RESTART |
            Self::DISCARDUNDO |
            Self::NOP => (0,0),

            Self::STKCOUNT |
            Self::GETMEMSIZE |
            Self::SAVEUNDO |
            Self::RESTOREUNDO |
            Self::HASUNDO |
            Self::VERIFY |
            Self::GETSTRINGTBL => (0, 1),

            Self::GETIOSYS => (0, 2),

            Self::JUMP |
            Self::JUMPABS |
            Self::STKCOPY |
            Self::RETURN |
            Self::MFREE |
            Self::STREAMCHAR |
            Self::STREAMUNICHAR |
            Self::STREAMNUM |
            Self::STREAMSTR |
            Self::SETSTRINGTBL |
            Self::SETRANDOM | 
            Self::DEBUGTRAP => (1, 0),

            Self::NEG |
            Self::BITNOT |
            Self::COPY |
            Self::COPYS |
            Self::COPYB |
            Self::SEXS |
            Self::SEXB |
            Self::STKPEEK |
            Self::CALLF |
            Self::CATCH | //IMPORTANT: SPECIAL CASE S1 before L1: https://eblong.com/zarf/glulx/Glulx-Spec.html#continuations
            Self::SETMEMSIZE |
            Self::MALLOC |
            Self::SAVE |
            Self::RESTORE |
            Self::NUMTOF |
            Self::FTONUMZ |
            Self::FTONUMN |
            Self::CEIL |
            Self::FLOOR |
            Self::SQRT |
            Self::EXP |
            Self::LOG |
            Self::SIN |
            Self::COS |
            Self::TAN |
            Self::ACOS |
            Self::ASIN |
            Self::ATAN |
            Self::RANDOM => (1, 1),

            Self::NUMTOD |
            Self::FTOD => (1, 2),

            Self::JZ  |
            Self::JNZ |
            Self::STKROLL |
            Self::TAILCALL |
            Self::THROW |
            Self::PROTECT |
            Self::SETIOSYS |
            Self::JISNAN |
            Self::JISINF |
            Self::MZERO | 
            Self::ACCELFUNC |
            Self::ACCELPARAM => (2, 0),

            Self::ADD | 
            Self::SUB | 
            Self::MUL | 
            Self::DIV |
            Self::MOD | 
            Self::BITAND |
            Self::BITOR  |
            Self::BITXOR |
            Self::SHIFTL |
            Self::USHIFTR |
            Self::SSHIFTR |
            Self::ALOAD |
            Self::ALOADS |
            Self::ALOADB |
            Self::ALOADBIT |
            Self::CALL |
            Self::CALLFI |
            Self::FADD |
            Self::FSUB |
            Self::FMUL |
            Self::FDIV |
            Self::POW |
            Self::ATAN2 |
            Self::DTONUMZ |
            Self::DTONUMN |
            Self::DTOF |
            Self::GESTALT |
            Self::GLK => (2, 1),

            Self::FMOD |
            Self::DCEIL |
            Self::DFLOOR |
            Self::DSQRT |
            Self::DEXP |
            Self::DLOG |
            Self::DSIN |
            Self::DCOS |
            Self::DTAN |
            Self::DACOS |
            Self::DASIN |
            Self::DATAN => (2, 2),

            Self::JEQ |
            Self::JNE |
            Self::JLT |
            Self::JLE |
            Self::JGT |
            Self::JGE |
            Self::JLTU |
            Self::JLEU |
            Self::JGTU |
            Self::JGEU |
            Self::ASTORE |
            Self::ASTORES |
            Self::ASTOREB |
            Self::ASTOREBIT |
            Self::JFLT |
            Self::JFLE |
            Self::JFGT |
            Self::JFGE |
            Self::JDISNAN |
            Self::JDISINF |
            Self::MCOPY => (3, 0),

            Self::CALLFII => (3, 1),

            Self::JFEQ |
            Self::JFNE => (4, 0),

            Self::CALLFIII => (4, 1),

            Self::DADD |
            Self::DSUB |
            Self::DMUL |
            Self::DDIV |
            Self::DMODR |
            Self::DMODQ |
            Self::DPOW |
            Self::DATAN2 => (4, 2),

            Self::JDLT |
            Self::JDLE |
            Self::JDGT |
            Self::JDGE => (5, 0),

            Self::LINKEDSEARCH => (6, 1),

            Self::JDEQ |
            Self::JDNE => (7, 0),

            Self::LINEARSEARCH |
            Self::BINARYSEARCH => (7, 1)
        }
    }
}
