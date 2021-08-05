//  Copyleft (ↄ) 2021 BxNiom <bxniom@protonmail.com> | https://github.com/bxniom
//
//  This work is free. You can redistribute it and/or modify it under the
//  terms of the Do What The Fuck You Want To Public License, Version 2,
//  as published by Sam Hocevar. See the COPYING file for more details.

#[macro_use]
pub mod terminfo;
pub mod capabilities;
pub mod param_string;

#[macro_export]
macro_rules! sprintf {
    ($f:expr, $($a:expr),*)
    =>
    {
        {
            extern "C" { fn sprintf(s: *mut std::os::raw::c_char, format: *const std::os::raw::c_char, ...) -> std::os::raw::c_int; }

            unsafe {
                if let (Ok(rp), Ok(fm)) = (std::ffi::CString::new(""), std::ffi::CString::new($f)) {
                    let rp_raw = rp.into_raw();
                    sprintf(rp_raw, fm.as_ptr(), $($a),*);
                    Ok(std::ffi::CString::from_raw(rp_raw).as_bytes().iter().map(|c| *c as char).collect::<String>())
                } else {
                    Err(())
                }
            }
        }
    }
}