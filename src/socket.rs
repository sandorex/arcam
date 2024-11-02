//! File containing all things related to the unix socket communication

use crate::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[repr(u8)]
pub enum Command {
    /// Signal repeated by each exec / shell command launch to keep dispose
    /// of dead processes automatically
    Heartbeat {
        /// Host PID of process invoking it
        host_pid: u32,

        /// PID of the process invoked inside container
        container_pid: u32,
    } = 1,
}

// impl Command {
//     pub fn to_bson(&self) -> Result<Vec<u8>> {
//         bson::to_vec(&self)
//             .context("Failed to serialize data to bson")
//     }

//     pub fn from_bson(&self, data: &[u8]) -> Result<Self> {
//         bson::from_slice(data)
//             .context("Failed to deserialize data from bson")
//     }
// }

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Response {
    /// Returns index of received command, for error checking
    Received(u8),
}
