//! File containing all things related to the unix socket communication

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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Response {
    /// Basically nothing, placeholder for now
    Received,
}
