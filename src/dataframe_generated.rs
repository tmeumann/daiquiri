// automatically generated by the FlatBuffers compiler, do not modify



use std::mem;
use std::cmp::Ordering;

extern crate flatbuffers;
use self::flatbuffers::EndianScalar;

#[allow(unused_imports, dead_code)]
pub mod daiquiri {

  use std::mem;
  use std::cmp::Ordering;

  extern crate flatbuffers;
  use self::flatbuffers::EndianScalar;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Event {
  NONE = 0,
  SensorFrame = 1,
  BuzzerEvent = 2,

}

pub const ENUM_MIN_EVENT: u8 = 0;
pub const ENUM_MAX_EVENT: u8 = 2;

impl<'a> flatbuffers::Follow<'a> for Event {
  type Inner = Self;
  #[inline]
  fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    flatbuffers::read_scalar_at::<Self>(buf, loc)
  }
}

impl flatbuffers::EndianScalar for Event {
  #[inline]
  fn to_little_endian(self) -> Self {
    let n = u8::to_le(self as u8);
    let p = &n as *const u8 as *const Event;
    unsafe { *p }
  }
  #[inline]
  fn from_little_endian(self) -> Self {
    let n = u8::from_le(self as u8);
    let p = &n as *const u8 as *const Event;
    unsafe { *p }
  }
}

impl flatbuffers::Push for Event {
    type Output = Event;
    #[inline]
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {
        flatbuffers::emplace_scalar::<Event>(dst, *self);
    }
}

#[allow(non_camel_case_types)]
pub const ENUM_VALUES_EVENT:[Event; 3] = [
  Event::NONE,
  Event::SensorFrame,
  Event::BuzzerEvent
];

#[allow(non_camel_case_types)]
pub const ENUM_NAMES_EVENT:[&'static str; 3] = [
    "NONE",
    "SensorFrame",
    "BuzzerEvent"
];

pub fn enum_name_event(e: Event) -> &'static str {
  let index = e as u8;
  ENUM_NAMES_EVENT[index as usize]
}

pub struct EventUnionTableOffset {}
pub enum SensorFrameOffset {}
#[derive(Copy, Clone, Debug, PartialEq)]

pub struct SensorFrame<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for SensorFrame<'a> {
    type Inner = SensorFrame<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self {
            _tab: flatbuffers::Table { buf: buf, loc: loc },
        }
    }
}

impl<'a> SensorFrame<'a> {
    #[inline]
    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
        SensorFrame {
            _tab: table,
        }
    }
    #[allow(unused_mut)]
    pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
        args: &'args SensorFrameArgs<'args>) -> flatbuffers::WIPOffset<SensorFrame<'bldr>> {
      let mut builder = SensorFrameBuilder::new(_fbb);
      if let Some(x) = args.frame { builder.add_frame(x); }
      if let Some(x) = args.timestamps { builder.add_timestamps(x); }
      builder.add_samples(args.samples);
      builder.add_channels(args.channels);
      builder.finish()
    }

    pub const VT_CHANNELS: flatbuffers::VOffsetT = 4;
    pub const VT_SAMPLES: flatbuffers::VOffsetT = 6;
    pub const VT_TIMESTAMPS: flatbuffers::VOffsetT = 8;
    pub const VT_FRAME: flatbuffers::VOffsetT = 10;

  #[inline]
  pub fn channels(&self) -> u8 {
    self._tab.get::<u8>(SensorFrame::VT_CHANNELS, Some(0)).unwrap()
  }
  #[inline]
  pub fn samples(&self) -> u16 {
    self._tab.get::<u16>(SensorFrame::VT_SAMPLES, Some(0)).unwrap()
  }
  #[inline]
  pub fn timestamps(&self) -> Option<flatbuffers::Vector<'a, u32>> {
    self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, u32>>>(SensorFrame::VT_TIMESTAMPS, None)
  }
  #[inline]
  pub fn frame(&self) -> Option<flatbuffers::Vector<'a, f64>> {
    self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, f64>>>(SensorFrame::VT_FRAME, None)
  }
}

pub struct SensorFrameArgs<'a> {
    pub channels: u8,
    pub samples: u16,
    pub timestamps: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a ,  u32>>>,
    pub frame: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a ,  f64>>>,
}
impl<'a> Default for SensorFrameArgs<'a> {
    #[inline]
    fn default() -> Self {
        SensorFrameArgs {
            channels: 0,
            samples: 0,
            timestamps: None,
            frame: None,
        }
    }
}
pub struct SensorFrameBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> SensorFrameBuilder<'a, 'b> {
  #[inline]
  pub fn add_channels(&mut self, channels: u8) {
    self.fbb_.push_slot::<u8>(SensorFrame::VT_CHANNELS, channels, 0);
  }
  #[inline]
  pub fn add_samples(&mut self, samples: u16) {
    self.fbb_.push_slot::<u16>(SensorFrame::VT_SAMPLES, samples, 0);
  }
  #[inline]
  pub fn add_timestamps(&mut self, timestamps: flatbuffers::WIPOffset<flatbuffers::Vector<'b , u32>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(SensorFrame::VT_TIMESTAMPS, timestamps);
  }
  #[inline]
  pub fn add_frame(&mut self, frame: flatbuffers::WIPOffset<flatbuffers::Vector<'b , f64>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(SensorFrame::VT_FRAME, frame);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> SensorFrameBuilder<'a, 'b> {
    let start = _fbb.start_table();
    SensorFrameBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<SensorFrame<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

pub enum BuzzerEventOffset {}
#[derive(Copy, Clone, Debug, PartialEq)]

pub struct BuzzerEvent<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for BuzzerEvent<'a> {
    type Inner = BuzzerEvent<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self {
            _tab: flatbuffers::Table { buf: buf, loc: loc },
        }
    }
}

impl<'a> BuzzerEvent<'a> {
    #[inline]
    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
        BuzzerEvent {
            _tab: table,
        }
    }
    #[allow(unused_mut)]
    pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
        args: &'args BuzzerEventArgs) -> flatbuffers::WIPOffset<BuzzerEvent<'bldr>> {
      let mut builder = BuzzerEventBuilder::new(_fbb);
      builder.add_timestamp(args.timestamp);
      builder.finish()
    }

    pub const VT_TIMESTAMP: flatbuffers::VOffsetT = 4;

  #[inline]
  pub fn timestamp(&self) -> u32 {
    self._tab.get::<u32>(BuzzerEvent::VT_TIMESTAMP, Some(0)).unwrap()
  }
}

pub struct BuzzerEventArgs {
    pub timestamp: u32,
}
impl<'a> Default for BuzzerEventArgs {
    #[inline]
    fn default() -> Self {
        BuzzerEventArgs {
            timestamp: 0,
        }
    }
}
pub struct BuzzerEventBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> BuzzerEventBuilder<'a, 'b> {
  #[inline]
  pub fn add_timestamp(&mut self, timestamp: u32) {
    self.fbb_.push_slot::<u32>(BuzzerEvent::VT_TIMESTAMP, timestamp, 0);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> BuzzerEventBuilder<'a, 'b> {
    let start = _fbb.start_table();
    BuzzerEventBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<BuzzerEvent<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

pub enum DaiquiriDataOffset {}
#[derive(Copy, Clone, Debug, PartialEq)]

pub struct DaiquiriData<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for DaiquiriData<'a> {
    type Inner = DaiquiriData<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self {
            _tab: flatbuffers::Table { buf: buf, loc: loc },
        }
    }
}

impl<'a> DaiquiriData<'a> {
    #[inline]
    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
        DaiquiriData {
            _tab: table,
        }
    }
    #[allow(unused_mut)]
    pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
        args: &'args DaiquiriDataArgs) -> flatbuffers::WIPOffset<DaiquiriData<'bldr>> {
      let mut builder = DaiquiriDataBuilder::new(_fbb);
      if let Some(x) = args.event { builder.add_event(x); }
      builder.add_event_type(args.event_type);
      builder.finish()
    }

    pub const VT_EVENT_TYPE: flatbuffers::VOffsetT = 4;
    pub const VT_EVENT: flatbuffers::VOffsetT = 6;

  #[inline]
  pub fn event_type(&self) -> Event {
    self._tab.get::<Event>(DaiquiriData::VT_EVENT_TYPE, Some(Event::NONE)).unwrap()
  }
  #[inline]
  pub fn event(&self) -> Option<flatbuffers::Table<'a>> {
    self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Table<'a>>>(DaiquiriData::VT_EVENT, None)
  }
  #[inline]
  #[allow(non_snake_case)]
  pub fn event_as_sensor_frame(&self) -> Option<SensorFrame<'a>> {
    if self.event_type() == Event::SensorFrame {
      self.event().map(|u| SensorFrame::init_from_table(u))
    } else {
      None
    }
  }

  #[inline]
  #[allow(non_snake_case)]
  pub fn event_as_buzzer_event(&self) -> Option<BuzzerEvent<'a>> {
    if self.event_type() == Event::BuzzerEvent {
      self.event().map(|u| BuzzerEvent::init_from_table(u))
    } else {
      None
    }
  }

}

pub struct DaiquiriDataArgs {
    pub event_type: Event,
    pub event: Option<flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>>,
}
impl<'a> Default for DaiquiriDataArgs {
    #[inline]
    fn default() -> Self {
        DaiquiriDataArgs {
            event_type: Event::NONE,
            event: None,
        }
    }
}
pub struct DaiquiriDataBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> DaiquiriDataBuilder<'a, 'b> {
  #[inline]
  pub fn add_event_type(&mut self, event_type: Event) {
    self.fbb_.push_slot::<Event>(DaiquiriData::VT_EVENT_TYPE, event_type, Event::NONE);
  }
  #[inline]
  pub fn add_event(&mut self, event: flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(DaiquiriData::VT_EVENT, event);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> DaiquiriDataBuilder<'a, 'b> {
    let start = _fbb.start_table();
    DaiquiriDataBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<DaiquiriData<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

#[inline]
pub fn get_root_as_daiquiri_data<'a>(buf: &'a [u8]) -> DaiquiriData<'a> {
  flatbuffers::get_root::<DaiquiriData<'a>>(buf)
}

#[inline]
pub fn get_size_prefixed_root_as_daiquiri_data<'a>(buf: &'a [u8]) -> DaiquiriData<'a> {
  flatbuffers::get_size_prefixed_root::<DaiquiriData<'a>>(buf)
}

#[inline]
pub fn finish_daiquiri_data_buffer<'a, 'b>(
    fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>,
    root: flatbuffers::WIPOffset<DaiquiriData<'a>>) {
  fbb.finish(root, None);
}

#[inline]
pub fn finish_size_prefixed_daiquiri_data_buffer<'a, 'b>(fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>, root: flatbuffers::WIPOffset<DaiquiriData<'a>>) {
  fbb.finish_size_prefixed(root, None);
}
}  // pub mod Daiquiri

