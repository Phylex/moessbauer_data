use std::mem::{transmute, size_of};
use std::io::Cursor;
extern crate byteorder;
use byteorder::{BigEndian, ReadBytesExt};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_serialize() {
        let serialized_data: Vec<u8> = vec![0, 0, 0, 0, 0, 15, 66, 64,
                                            0, 0, 0, 0, 0, 0, 0, 100,
                                            0, 0, 0, 0, 0, 0, 0, 20,
                                            0, 0, 0, 0, 0, 0, 0, 50,
                                            0, 0, 0, 0, 0, 30, 132, 128];
        let config = FilterConfig { pthresh: 1_000_000,
                                    tdead: 100,
                                    k: 20,
                                    l: 50,
                                    m: 2_000_000 };
        for (ser_byte, ref_byte) in config.serialize().iter().zip(serialized_data.iter()) {
            assert_eq!(ser_byte, ref_byte);
        }
    }

    #[test]
    fn peak_serialize() {
        let serialized_data: Vec<u8> = vec![0, 0, 0, 0x0d, 0xc7, 0x86, 0x4a, 0x8c,
                                            0x03, 0x37, 0x36, 0x47,
                                            0x03, 0xb3,
                                            0, 0, 0x9d, 0x56,];
        let peak = MeasuredPeak { timestamp: 59182041740,
                                    peak_height: 53950023,
                                    speed: 947,
                                    cycle: 40278 };
        for (ser_byte, ref_byte) in peak.serialize().iter().zip(serialized_data.iter()) {
            assert_eq!(ser_byte, ref_byte);
        }
    }

    #[test]
    fn peak_serialize_deserialize() {
        let peak = MeasuredPeak { timestamp: 59182041740,
                                    peak_height: 53950023,
                                    speed: 947,
                                    cycle: 40278 };
        let buffer = peak.serialize();
        assert_eq!(Ok((peak, 18)), MeasuredPeak::deserialize(&buffer));
    }

    #[test]
    fn filter_serialize_deserialize() {
        let config = FilterConfig { pthresh: 1_000_000,
                                    tdead: 100,
                                    k: 20,
                                    l: 50,
                                    m: 2_000_000 };
        let buffer = config.serialize();
        assert_eq!(Ok((config, 40)), FilterConfig::deserialize(&buffer));
    }

    #[test]
    fn serialize_status() {
        let stat = Status::Start;
        let ser_data = stat.serialize();
        assert_eq!(ser_data[0], 0);
        let stat = Status::Stop;
        let ser_data = stat.serialize();
        assert_eq!(ser_data[0], 1);
    }
}

#[derive(Debug, PartialEq)]
pub struct FilterConfig {
    pub pthresh: u64,
    pub tdead: u64,
    pub k: u64,
    pub l: u64,
    pub m: u64,
}

#[derive(Debug, PartialEq)]
pub struct MeasuredPeak {
    pub timestamp: u64,
    pub peak_height: u32,
    pub speed: u16,
    pub cycle: u32,
}

pub enum Status {
    Start,
    Stop,
}

pub enum Message {
    Data(Vec<MeasuredPeak>),
    Status(Status),
    Config(FilterConfig),
}

pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

pub trait Deserialize {
    type Item;
    fn deserialize(buffer: &[u8]) -> Result<(Self::Item, usize), ()>;
}

impl MeasuredPeak {
    pub fn new(raw_data: &[u8]) -> MeasuredPeak {
        if raw_data.len() != 12 {
            panic!("Wrong buffer length passed in")
        }
        let mut timestamp: u64 = 0;
        for (i, &byte) in raw_data[..5].iter().enumerate() {
            timestamp |= (byte as u64) << i*8;
        }
        // the cycle count has two normal bytes and one shared one
        let mut cycle: u32 = 0;
        cycle |= (raw_data[5] as u32) << 8;
        cycle |= (raw_data[6] as u32) << 16;
        cycle |= (raw_data[7] as u32) & 0x03 << 24;
        // decode the speed counter
        let mut speed: u16 = 0;
        speed |= (raw_data[7] as u16) & 0xFC >> 2;
        speed |= (raw_data[8] as u16) & 0x0F << 6;
        // last but not least the peak height
        let mut peak_height: u32 = 0;
        peak_height |= (raw_data[8] as u32) & 0xF0 >> 4;
        for (i, &byte) in raw_data[9..12].iter().enumerate() {
            peak_height |= (byte as u32) << (i*8+4)
        }
        MeasuredPeak { timestamp, peak_height, speed, cycle }
    }
}

impl Serialize for MeasuredPeak {
    fn serialize(&self) -> Vec<u8> {
        let mut ser_data: Vec<u8> = Vec::with_capacity(18);
        let timestamp_bytes: [u8; 8] = unsafe { transmute(self.timestamp.to_be()) };
        for &byte in timestamp_bytes.iter() {
            ser_data.push(byte);
        }
        let peak_height_bytes: [u8; 4] = unsafe { transmute(self.peak_height.to_be()) };
        for &byte in peak_height_bytes.iter() {
            ser_data.push(byte);
        }
        let speed_bytes: [u8; 2] = unsafe { transmute(self.speed.to_be()) };
        for &byte in speed_bytes.iter() {
            ser_data.push(byte);
        }
        let cycle_bytes: [u8; 4] = unsafe { transmute(self.cycle.to_be()) };
        for &byte in cycle_bytes.iter() {
            ser_data.push(byte);
        }
        ser_data
    }
}

impl Deserialize for MeasuredPeak {
    type Item = MeasuredPeak;
    fn deserialize(buffer: &[u8]) -> Result<(Self::Item, usize), ()> {
        let needed_len = size_of::<u64>() +
                         size_of::<u32>() +
                         size_of::<u16>() +
                         size_of::<u32>();
        let mut peak = MeasuredPeak {
            timestamp: 0,
            peak_height: 0,
            speed: 0,
            cycle: 0 };
        if buffer.len() != needed_len {
            println!("{}", buffer.len());
            println!("{}", needed_len);
            Err(())
        } else {
            let mut reader = Cursor::new(buffer);
            peak.timestamp = reader.read_u64::<BigEndian>().unwrap();
            peak.peak_height = reader.read_u32::<BigEndian>().unwrap();
            peak.speed = reader.read_u16::<BigEndian>().unwrap();
            peak.cycle = reader.read_u32::<BigEndian>().unwrap();
            Ok((peak, needed_len))
        }
    }
}

impl Serialize for FilterConfig {
    fn serialize(&self) -> Vec<u8> {
        let mut ser_data: Vec<u8> = Vec::with_capacity(40);
        let pthresh_bytes: [u8; 8] = unsafe { transmute(self.pthresh.to_be()) };
        for &byte in pthresh_bytes.iter() {
            ser_data.push(byte);
        }
        let tdead_bytes: [u8; 8] = unsafe { transmute(self.tdead.to_be()) };
        for &byte in tdead_bytes.iter() {
            ser_data.push(byte);
        }
        let k_bytes: [u8; 8] = unsafe { transmute(self.k.to_be()) };
        for &byte in k_bytes.iter() {
            ser_data.push(byte);
        }
        let l_bytes: [u8; 8] = unsafe { transmute(self.l.to_be()) };
        for &byte in l_bytes.iter() {
            ser_data.push(byte);
        }
        let m_bytes: [u8; 8] = unsafe { transmute(self.m.to_be()) };
        for &byte in m_bytes.iter() {
            ser_data.push(byte);
        }
        ser_data
    }
}

impl Deserialize for FilterConfig {
    type Item = FilterConfig;
    fn deserialize(buffer: &[u8]) -> Result<(Self::Item, usize), ()> {
        let needed_len = size_of::<u64>() * 5;
        let mut config = FilterConfig {
            pthresh: 0,
            tdead: 0,
            k: 0,
            l: 0,
            m: 0 };
        if buffer.len() != needed_len {
            Err(())
        } else {
            let mut reader = Cursor::new(buffer);
            config.pthresh = reader.read_u64::<BigEndian>().unwrap();
            config.tdead = reader.read_u64::<BigEndian>().unwrap();
            config.k = reader.read_u64::<BigEndian>().unwrap();
            config.l = reader.read_u64::<BigEndian>().unwrap();
            config.m = reader.read_u64::<BigEndian>().unwrap();
            Ok((config, needed_len))
        }
    }
}

impl Serialize for Status {
    fn serialize(&self) -> Vec<u8> {
        let mut ser_data: Vec<u8> = Vec::with_capacity(1);
        match self {
            Status::Start => { ser_data.push(0) },
            Status::Stop => { ser_data.push(1) },
        }
        ser_data
    }
}

impl Deserialize for Status {
    type Item = Status;
    fn deserialize(buffer: &[u8]) -> Result<(Self::Item, usize), ()> {
        match &buffer[0] {
            0 => Ok((Status::Start, 1)),
            1 => Ok((Status::Stop, 1)),
            _ => Err(()),
        }
    }
}

impl Serialize for Message {
    fn serialize(&self) -> Vec<u8> {
        let mut ser_data: Vec<u8> = Vec::new();
        match self {
            Message::Data(peaks) => {
                ser_data.push(0);
                let peak_count: [u8; 8] = unsafe { transmute((peaks.len() as u64).to_be()) };
                for &byte in peak_count.iter() {
                    ser_data.push(byte);
                }
                for peak in peaks.iter() {
                    for &byte in peak.serialize().iter() {
                        ser_data.push(byte);
                    }
                }
                ser_data
            },
            Message::Status(status) => {
                ser_data.push(1);
                for &byte in status.serialize().iter() {
                    ser_data.push(byte);
                }
                ser_data
            },
            Message::Config(config) => {
                ser_data.push(2);
                for &byte in config.serialize().iter() {
                    ser_data.push(byte);
                }
                ser_data
            },
        }
    }
}

impl Deserialize for Message {
    type Item = Message;
    fn deserialize(buffer: &[u8]) -> Result<(Self::Item, usize), ()> {
        match &buffer[0] {
            0 => {
                let mut reader = Cursor::new(&buffer[1..9]);
                let peak_len = size_of::<MeasuredPeak>() - 6;
                let peak_cnt = reader.read_u64::<BigEndian>().unwrap() as usize;
                let message_len =  peak_cnt * peak_len;
                let peak_buf = &buffer[10..];
                let mut size = 10;
                if buffer[10..].len() < message_len {
                    Err(())
                } else {
                    let mut peak_vec: Vec<MeasuredPeak> = Vec::with_capacity(peak_cnt);
                    for i in 0..peak_cnt {
                        let cur_buf = &peak_buf[i*peak_len .. (i+1) * peak_len];
                        let (cur_peak, peak_size) = MeasuredPeak::deserialize(cur_buf).unwrap();
                        peak_vec.push(cur_peak);
                        size += peak_size;
                    }
                    Ok((Message::Data(peak_vec), size as usize))
                }
            },
            1 => {
                let (status, size) = Status::deserialize(&buffer[1..2]).unwrap();
                Ok((Message::Status(status), 1 as usize + size))
            },
            2 => {
                Err(())
            },
            _ => { Err(()) }
        }
    }
}
