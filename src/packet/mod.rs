use std::convert::TryInto;

use serde::Deserialize;

pub use types::*;

use crate::errors::Result;
use crate::raw::RawPacket;
use crate::ParseError;

mod parser;
pub mod raw;
mod types;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Packet {
    op: Operation,
    data: Vec<u8>,
}

impl Packet {
    pub fn op(&self) -> Operation {
        self.op
    }
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
    pub fn json<'a, T: Deserialize<'a>>(&'a self) -> Result<T> {
        serde_json::from_slice(&self.data).map_err(|e| ParseError::JSON(e).into())
    }
    pub fn int32_be(&self) -> Result<i32> {
        Ok(i32::from_be_bytes(
            self.data
                .as_slice()
                .try_into()
                .map_err(|_| ParseError::Int32BE)?,
        ))
    }
}

impl From<RawPacket> for Packet {
    fn from(pack: RawPacket) -> Self {
        Packet {
            op: pack.op,
            data: pack.data,
        }
    }
}