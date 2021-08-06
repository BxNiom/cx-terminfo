//  Copyleft (ↄ) 2021 BxNiom <bxniom@protonmail.com> | https://github.com/bxniom
//
//  This work is free. You can redistribute it and/or modify it under the
//  terms of the Do What The Fuck You Want To Public License, Version 2,
//  as published by Sam Hocevar. See the COPYING file for more details.

use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::capabilities::{BoolCapability, NumberCapability, StringCapability};

/// magic number octal 0432 for legacy ncurses terminfo
const MAGIC_LEGACY: i16 = 0x11A;
/// magic number octal 01036 for new ncruses terminfo
const MAGIC_32BIT: i16 = 0x21E;
/// the offset into data where the names section begins.
const NAMES_OFFSET: usize = 12;

const EXT_HEADER_SIZE: usize = 10;
const TERMINFO_HEADER_SIZE: usize = 12;
const TERMINFO_MAX_SIZE: usize = 4096;

/// Terminfo database information
#[derive(Debug)]
pub struct TermInfo {
    data: Vec<u8>,
    read_i32: bool,
    int_size: usize,
    sec_name_size: usize,
    sec_bool_size: usize,
    sec_number_size: usize,
    sec_str_offsets_size: usize,
    sec_str_table_size: usize,
    ext_bool: HashMap<String, bool>,
    ext_numbers: HashMap<String, i32>,
    ext_strings: HashMap<String, String>,
}

#[derive(Debug)]
pub enum TermInfoError {
    InvalidDataSize,
    InvalidMagicNum,
    InvalidData,
    InvalidName,
}

impl Display for TermInfoError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}",
               match self {
                   TermInfoError::InvalidDataSize => "file/data length is above 4096 bytes or under 12 bytes",
                   TermInfoError::InvalidMagicNum => "magic number mismatch",
                   TermInfoError::InvalidData => "terminfo data is invalid or corrupt",
                   TermInfoError::InvalidName => "terminfo not found"
               })
    }
}

impl TermInfo {
    /// Returns the string value for the capability or Option::None
    ///
    /// # Arguments
    /// * `cap` - string capability
    ///
    /// # Example
    /// ```
    /// use cxterminfo::terminfo;
    /// use cxterminfo::capabilities::StringCapability;
    ///
    /// if let Ok(info) = terminfo::from_env() {
    ///     println!("{:?}", info.get_string(StringCapability::Bell));
    /// }
    /// ```
    pub fn get_string(&self, cap: StringCapability) -> Option<String> {
        let idx = cap as usize;
        if idx >= self.sec_str_offsets_size {
            None
        } else {
            let tbl_idx = read_i16(&self.data, self.offset_str_offsets() + (idx * 2)) as usize;
            if tbl_idx == 0 {
                None
            } else {
                Some(read_str(&self.data, self.offset_str_table() + tbl_idx).0.to_string())
            }
        }
    }

    /// Returns the number value for the capability or Option::None
    ///
    /// # Arguments
    /// * `cap` - number capability
    ///
    /// # Example
    /// ```
    /// use cxterminfo::terminfo;
    /// use cxterminfo::capabilities::NumberCapability;
    ///
    /// if let Ok(info) = terminfo::from_env() {
    ///     println!("{:?}", info.get_number(NumberCapability::MaxColors));
    /// }
    /// ```
    pub fn get_number(&self, cap: NumberCapability) -> Option<i32> {
        let idx = cap as usize;
        if idx >= self.sec_number_size {
            None
        } else {
            Some(read_int(&self.data, self.offset_number() + (idx * self.int_size), self.read_i32))
        }
    }

    /// Returns the bool value for the capability or Option::None
    ///
    /// # Arguments
    /// * `cap` - bool capability
    ///
    /// # Example
    /// ```
    /// use cxterminfo::terminfo;
    /// use cxterminfo::capabilities::BoolCapability;
    ///
    /// if let Ok(info) = terminfo::from_env() {
    ///     println!("{:?}", info.get_bool(BoolCapability::AutoLeftMargin));
    /// }
    /// ```
    pub fn get_bool(&self, cap: BoolCapability) -> Option<bool> {
        let idx = cap as usize;
        if idx >= self.sec_bool_size {
            None
        } else {
            Some(self.data[(self.offset_bool() + idx)] == 1)
        }
    }

    /// Returns the extended bool value for the given name or Option::None if name not exist
    ///
    /// # Arguments
    /// * `name` - key
    ///
    /// # Example
    /// ```
    /// use cxterminfo::terminfo;
    ///
    /// if let Ok(info) = terminfo::from_env() {
    ///     println!("{:?}", info.get_ext_bool("AT"));
    /// }
    /// ```
    pub fn get_ext_bool(&self, name: &str) -> Option<&bool> {
        self.ext_bool.get(name)
    }

    /// Returns the extended number value for the given name or Option::None if name not exist
    ///
    /// # Arguments
    /// * `name` - key
    ///
    /// # Example
    /// ```
    /// use cxterminfo::terminfo;
    ///
    /// if let Ok(info) = terminfo::from_env() {
    ///     println!("{:?}", info.get_ext_number("?"));
    /// }
    /// ```
    pub fn get_ext_number(&self, name: &str) -> Option<&i32> {
        self.ext_numbers.get(name)
    }

    /// Returns the extended string value for the given name or Option::None if name not exist
    ///
    /// # Arguments
    /// * `name` - key
    ///
    /// # Example
    /// ```
    /// use cxterminfo::terminfo;
    ///
    /// if let Ok(info) = terminfo::from_env() {
    ///     println!("{:?}", info.get_ext_number("xm"));
    /// }
    /// ```
    pub fn get_ext_string(&self, name: &str) -> Option<&String> {
        self.ext_strings.get(name)
    }

    /// Create terminfo database, using TERM environment var.
    pub fn from_env() -> Result<TermInfo, TermInfoError> {
        if let Ok(term) = std::env::var("TERM") {
            TermInfo::from_name(term.as_str())
        } else {
            Err(TermInfoError::InvalidName)
        }
    }

    /// Create terminfo database for the given name
    pub fn from_name(name: &str) -> Result<TermInfo, TermInfoError> {
        if name.len() == 0 {
            return Err(TermInfoError::InvalidName);
        }

        let first_letter = name.chars().nth(0).unwrap_or('X');

        let mut paths: Vec<PathBuf> = Vec::new();
        // env TERMINFO
        if let Ok(env_terminfo) = std::env::var("TERMINFO") {
            paths.push(PathBuf::from(format!("{}/{}/{}", env_terminfo, first_letter, name)));
        }

        // HOME .terminfo
        if let Ok(env_home) = std::env::var("HOME") {
            paths.push(PathBuf::from(format!("{}/{}/{}", env_home, first_letter, name)));
        }

        // Linux
        paths.push(PathBuf::from(format!("/etc/terminfo/{}/{}", first_letter, name)));
        paths.push(PathBuf::from(format!("/lib/terminfo/{}/{}", first_letter, name)));
        paths.push(PathBuf::from(format!("/usr/share/terminfo/{}/{}", first_letter, name)));
        paths.push(PathBuf::from(format!("/usr/share/misc/terminfo/{}/{}", first_letter, name)));

        // Mac
        paths.push(PathBuf::from(format!("/etc/terminfo/{:X}/{}", first_letter as u8, name)));
        paths.push(PathBuf::from(format!("/lib/terminfo/{:X}/{}", first_letter as u8, name)));
        paths.push(PathBuf::from(format!("/usr/share/terminfo/{:X}/{}", first_letter as u8, name)));
        paths.push(PathBuf::from(format!("/usr/share/misc/terminfo/{:X}/{}", first_letter as u8, name)));

        for path in paths {
            if path.exists() {
                return TermInfo::from_file(path.to_str().unwrap())
            }
        }

        Err(TermInfoError::InvalidName)
    }

    /// Create terminfo database using given filename
    pub fn from_file(filename: &str) -> Result<TermInfo, TermInfoError> {
        TermInfo::from_data(read_all_bytes_from_file(filename))
    }

    /// Create terminfo database by parse byte-array directly
    pub fn from_data(data: Vec<u8>) -> Result<TermInfo, TermInfoError> {
        if data.len() < TERMINFO_HEADER_SIZE || data.len() > TERMINFO_MAX_SIZE {
            return Err(TermInfoError::InvalidDataSize);
        }

        let mut info = TermInfo {
            data,
            read_i32: false,
            int_size: 2,
            sec_name_size: 0,
            sec_bool_size: 0,
            sec_number_size: 0,
            sec_str_offsets_size: 0,
            sec_str_table_size: 0,
            ext_bool: HashMap::new(),
            ext_numbers: HashMap::new(),
            ext_strings: HashMap::new(),
        };

        // read the magic number.
        let magic = read_i16(&info.data, 0);

        info.read_i32 = match magic {
            MAGIC_LEGACY => false,
            MAGIC_32BIT => true,
            _ => return Err(TermInfoError::InvalidMagicNum),
        };

        info.int_size = match info.read_i32 {
            true => 4,
            false => 2,
        };

        if read_i16(&info.data, 2) < 0
            || read_i16(&info.data, 4) < 0
            || read_i16(&info.data, 6) < 0
            || read_i16(&info.data, 8) < 0
            || read_i16(&info.data, 10) < 0
        {
            return Err(TermInfoError::InvalidData)
        }

        info.sec_name_size = read_i16(&info.data, 2) as usize;
        info.sec_bool_size = read_i16(&info.data, 4) as usize;
        info.sec_number_size = read_i16(&info.data, 6) as usize;
        info.sec_str_offsets_size = read_i16(&info.data, 8) as usize;
        info.sec_str_table_size = read_i16(&info.data, 10) as usize;


        // In addition to the main section of bools, numbers, and strings, there is also
        // an "extended" section.  This section contains additional entries that don't
        // have well-known indices, and are instead named mappings.  As such, we parse
        // all of this data now rather than on each request, as the mapping is fairly complicated.
        // This function relies on the data stored above, so it's the last thing we run.
        let mut ext_offset = round_up_even(info.offset_str_table() + info.sec_str_table_size);

        // Check if there is an extended section
        if ext_offset + EXT_HEADER_SIZE < info.data.len() {
            if read_i16(&info.data, ext_offset) < 0
                || read_i16(&info.data, ext_offset + 2) < 0
                || read_i16(&info.data, ext_offset + 4) < 0
            {
                // The extended contained invalid data
                return Ok(info);
            }

            let ext_bool_count = read_i16(&info.data, ext_offset) as usize;
            let ext_number_count = read_i16(&info.data, ext_offset + 2) as usize;
            let ext_str_count = read_i16(&info.data, ext_offset + 4) as usize;

            // Read extended bool values
            let mut bool_values = Vec::with_capacity(ext_bool_count);

            ext_offset += EXT_HEADER_SIZE;
            for i in 0..ext_bool_count {
                let pos = ext_offset + read_i16(&info.data, ext_offset + i * 2) as usize;

                if pos == 0 || ext_offset > info.data.len() {
                    return Ok(info);
                }

                bool_values.push(info.data[pos] == 1);
            }

            // Read extended number values
            let mut number_values = Vec::with_capacity(ext_number_count);

            ext_offset += if ext_bool_count == 0 { 0 } else { (ext_bool_count - 1) * 2 };
            for i in 0..ext_number_count {
                let pos = ext_offset + read_i16(&info.data, ext_offset + i * 2) as usize;

                if pos == 0 || ext_offset > info.data.len() {
                    return Ok(info);
                }

                &number_values.push(read_int(&info.data, pos, info.read_i32));
            }

            // Now we need to parse all of the extended string values.  These aren't necessarily
            // "in order", meaning the offsets aren't guaranteed to be increasing.  Instead, we parse
            // the offsets in order, pulling out each string it references and storing them into our
            // value vector in the order of the offsets.
            let mut str_values = Vec::with_capacity(ext_str_count);

            ext_offset += if ext_number_count == 0 { 0 } else { (ext_number_count - 1) * 2 };

            let tbl_offset = ext_offset
                + ext_str_count * 2
                + (ext_bool_count + ext_number_count + ext_str_count) * 2;
            let mut last_end: usize = 0;
            for i in 0..ext_str_count {
                let pos = tbl_offset + read_i16(&info.data, ext_offset + i * 2) as usize;

                if pos == 0 || ext_offset > info.data.len() {
                    return Ok(info);
                }

                let (str, null_term_pos) = read_str(&info.data, pos);
                &str_values.push(str);
                last_end = last_end.max(null_term_pos)
            }

            // Read extended names
            // The names are in order for the bools, then the numbers, and then the strings.
            let mut names = Vec::with_capacity(ext_bool_count + ext_number_count + ext_str_count);
            let mut pos = last_end + 1;

            while pos < info.data.len() {
                let (str, null_term_pos) = read_str(&info.data, pos);
                &names.push(str);
                pos = null_term_pos + 1;
            }

            // Associate names with the bool values
            for i in 0..ext_bool_count {
                &info.ext_bool.insert(names[i].to_string(), bool_values[i]);
            }

            // Associate names with the number values
            for i in 0..ext_number_count {
                &info.ext_numbers
                     .insert(names[i + ext_bool_count - 1].to_string(), number_values[i]);
            }

            // Associate names with the string values
            for i in 0..ext_str_count {
                &info.ext_strings.insert(
                    names[i + ext_bool_count + ext_number_count].to_string(),
                    str_values[i].to_string(),
                );
            }
        }

        Ok(info)
    }

    /// The offset into data where the bools section begins
    fn offset_bool(&self) -> usize {
        NAMES_OFFSET + self.sec_name_size
    }
    /// The offset into data where the numbers section begins
    fn offset_number(&self) -> usize {
        round_up_even(self.offset_bool() + self.sec_bool_size)
    }
    /// The offset into data where the string offsets section begins.  We index into this section
    /// to find the location within the strings table where a string value exists.
    fn offset_str_offsets(&self) -> usize {
        self.offset_number() + (self.sec_number_size * self.int_size)
    }
    /// The offset into data where the string table exists
    fn offset_str_table(&self) -> usize {
        self.offset_str_offsets() + (self.sec_str_offsets_size * 2)
    }
}

/// Read i16 or i32
///
/// # Arguments
/// * `data`        -
/// * `pos`         - start position in data
/// * `as_32bit`    - true => read_i32, false => read_i16
///
///
/// # Warning
/// NOT SAFE
fn read_int(data: &Vec<u8>, pos: usize, as_32bit: bool) -> i32 {
    match as_32bit {
        true => read_i32(data, pos),
        false => read_i16(data, pos) as i32,
    }
}

/// Read i32 from data
///
/// # Warning
/// NOT SAFE
fn read_i32(data: &Vec<u8>, pos: usize) -> i32 {
    ((data[pos] as i32) << 24)
        | ((data[pos + 1] as i32) << 16)
        | ((data[pos + 2] as i32) << 8)
        | (data[pos + 3] as i32)
}

/// Read i16 from data
///
/// # Warning
/// NOT SAFE
fn read_i16(data: &Vec<u8>, pos: usize) -> i16 {
    ((data[pos + 1] as i16) << 8) | (data[pos] as i16)
}

/// Read all data from binary file to a vec<u8>
///
/// # Warning
/// NOT SAFE
fn read_all_bytes_from_file(filename: &str) -> Vec<u8> {
    let mut f = File::open(&filename).expect("no file found");
    let metadata = fs::metadata(&filename).expect("unable to read metadata");
    let mut buffer = vec![0; metadata.len() as usize];
    f.read(&mut buffer).expect("buffer overflow");

    buffer
}

/// Read string from data
///
/// # Warning
/// NOT SAFE
fn read_str(data: &Vec<u8>, pos: usize) -> (String, usize) {
    let null_term = find_null_term(data, pos);
    (data[pos..null_term].iter()
                         .map(|c| *c as char)
                         .collect::<String>(),
     null_term)
}

/// Find the next '\0' char in data
fn find_null_term(data: &Vec<u8>, pos: usize) -> usize {
    let mut term_pos = pos as i32;
    while term_pos < data.len() as i32 && data[term_pos as usize] != '\0' as u8 {
        term_pos += 1;
    }
    term_pos as usize
}

/// Simple int rounding to get even numbers
fn round_up_even(n: usize) -> usize {
    match n % 2 {
        1 => n + 1,
        _ => n,
    }
}
