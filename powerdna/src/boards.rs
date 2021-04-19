pub(crate) mod ai201;
pub(crate) mod dio405;

use core::option::Option::None;
use powerdna_sys::{pDQBCB, DQACBCFG};

pub(crate) const EVENT_TIMEOUT: i32 = 1000;

pub trait Bcb {
    fn bcb(&self) -> pDQBCB;
}

pub(crate) trait Empty {
    fn empty() -> Self;
}

impl Empty for DQACBCFG {
    fn empty() -> Self {
        Self {
            dirflags: 0,
            eucoeff: 0.0,
            euconvert: None,
            euoffset: 0.0,
            frames: 0,
            framesize: 0,
            hostringsz: 0,
            hwbufsize: 0,
            maxpktsize: 0,
            mode: 0,
            ppevent: 0,
            samplesz: 0,
            scansz: 0,
            valuesz: 0,
            wtrmark: 0,
        }
    }
}
