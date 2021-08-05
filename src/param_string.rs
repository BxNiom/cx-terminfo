//  Copyleft (ↄ) 2021 BxNiom <bxniom@protonmail.com> | https://github.com/bxniom
//
//  This work is free. You can redistribute it and/or modify it under the
//  terms of the Do What The Fuck You Want To Public License, Version 2,
//  as published by Sam Hocevar. See the COPYING file for more details.

use std::error::Error;
use std::ffi::CString;
use std::fmt::{Debug, Display, Formatter};

use super::sprintf;

#[derive(Clone)]
pub enum Param {
    Bool(bool),
    Number(i32),
    Word(String),
}

impl Param {
    fn as_str(&self) -> &str {
        match self {
            Param::Word(s) => s.as_str(),
            _ => "",
        }
    }

    fn as_int(&self) -> i32 {
        match self {
            Param::Number(n) => *n,
            Param::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    fn as_char(&self) -> char {
        match self {
            Param::Number(0) => 128u8 as char,
            Param::Number(n) => *n as u8 as char,
            _ => '\0',
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            Param::Number(n) => *n != 0,
            Param::Bool(b) => *b,
            _ => false,
        }
    }
}

impl Default for Param {
    fn default() -> Self {
        Param::Number(0)
    }
}

#[derive(Default)]
struct Variables {
    static_vars: [Param; 26],
    dynamic_vars: [Param; 26],
}

impl Variables {
    pub fn new() -> Self {
        Default::default()
    }
}

#[derive(Debug)]
pub enum EvalError {
    StackEmpty(usize),
    Invalid(usize),
    InvalidPrintf(usize),
}

impl Display for EvalError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::StackEmpty(pos) => write!(f, "Stack is empty ({})", pos),
            EvalError::Invalid(pos) => write!(f, "Invalid terminfo ({})", pos),
            EvalError::InvalidPrintf(pos) => write!(f, "Invalid printf format pattern ({})", pos),
        }
    }
}

impl Error for EvalError {}

pub fn evaluate(term: &str, params: &[Param]) -> Result<String, EvalError> {
    let mut vars = Variables::new();
    let mut stack: Vec<Param> = Vec::new();
    let mut pos = 0;
    let chars = Vec::from(term)
        .iter()
        .map(|c| *c as char)
        .collect::<Vec<char>>();
    __eval(&chars, params, &mut pos, &mut stack, &mut vars)
}

fn __eval(
    chars: &Vec<char>,
    params: &[Param],
    pos: &mut usize,
    stack: &mut Vec<Param>,
    vars: &mut Variables,
) -> Result<String, EvalError> {
    let mut output: String = String::new();
    let mut saw_if = false;

    while *pos < chars.len() {
        if chars[*pos] != '%' {
            output.push(chars[*pos]);
            *pos += 1;
            continue;
        }

        *pos += 1;
        match chars[*pos] {
            '%' => {
                output.push('%');
            }
            'c' => {
                if let Some(param) = stack.pop() {
                    output.push(param.as_char());
                }
            }
            's' => {
                if let Some(param) = stack.pop() {
                    output.push_str(param.as_str());
                }
            }
            'd' => {
                if let Some(param) = stack.pop() {
                    output.push_str(&param.as_int().to_string());
                }
            }
            'p' => {
                *pos += 1;
                debug_assert!(CHAR_BETWEEN(chars[*pos], '0', '9'));
                stack.push(params[CHAR_SUB(chars[*pos], '1') as usize].clone());
            }
            'l' => {
                if let Some(param) = stack.pop() {
                    stack.push(Param::Number(param.as_str().len() as i32))
                }
            }
            '{' => {
                *pos += 1;
                let mut lit = 0;
                while chars[*pos] != '}' {
                    debug_assert!(CHAR_BETWEEN(chars[*pos], '0', '9'));
                    lit = (lit * 10) + CHAR_SUB(chars[*pos], '0');
                    *pos += 1;
                }

                stack.push(Param::Number(lit as i32))
            }
            '\'' => {
                stack.push(Param::Number(chars[*pos + 1] as i32));
                debug_assert!(chars[*pos + 2] == '\'');
                *pos += 2;
            }
            'P' | 'g' => {
                *pos += 1;
                debug_assert!(
                    CHAR_BETWEEN(chars[*pos], 'A', 'Z') || CHAR_BETWEEN(chars[*pos], 'a', 'z')
                );
                let is_static = CHAR_BETWEEN(chars[*pos], 'A', 'Z');
                let idx = if is_static {
                    CHAR_SUB(chars[*pos], 'A')
                } else {
                    CHAR_SUB(chars[*pos], 'a')
                } as usize;

                match chars[*pos - 1] == 'P' {
                    true => {
                        // P = pop value
                        match is_static {
                            true => vars.static_vars[idx] = stack.pop().unwrap(),
                            false => vars.dynamic_vars[idx] = stack.pop().unwrap(),
                        }
                    }
                    false => {
                        // g = push value
                        match is_static {
                            true => stack.push(vars.static_vars[idx].clone()),
                            false => stack.push(vars.dynamic_vars[idx].clone()),
                        }
                    }
                }
            }
            // Unary operatioin
            '!' | '~' => {
                if let Some(val) = stack.pop() {
                    stack.push(if chars[*pos] == '!' {
                        Param::Number(match !val.as_bool() {
                            true => 1,
                            false => 0,
                        })
                    } else {
                        Param::Number(!val.as_int())
                    });
                }
            }
            // Binary operations
            '+' | '-' | '*' | '/' | 'm' | '^' | '&' | '|' | '=' | '>' | '<' | 'A' | 'O' => {
                if let (Some(second), Some(first)) = (stack.pop(), stack.pop()) {
                    let fi = first.as_int();
                    let si = second.as_int();
                    stack.push(Param::Number(match chars[*pos] {
                        '+' => (fi + si),
                        '-' => (fi - si),
                        '*' => (fi * si),
                        '/' => (fi / si),
                        'm' => (fi % si),
                        '^' => (fi ^ si),
                        '&' => (fi & si),
                        '|' => (fi | si),
                        '=' => {
                            if fi == si {
                                1
                            } else {
                                0
                            }
                        }
                        '>' => {
                            if fi > si {
                                1
                            } else {
                                0
                            }
                        }
                        '<' => {
                            if fi < si {
                                1
                            } else {
                                0
                            }
                        }
                        'A' => {
                            if first.as_bool() && second.as_bool() {
                                1
                            } else {
                                0
                            }
                        }
                        'O' => {
                            if first.as_bool() || second.as_bool() {
                                1
                            } else {
                                0
                            }
                        }
                        _ => 0,
                    }));
                }
            }
            '?' => {
                saw_if = true;
            }
            't' => {
                let result = if let Some(x) = stack.pop() {
                    x.as_bool()
                } else {
                    return Err(EvalError::StackEmpty(*pos));
                };
                *pos += 1;

                let then_res = __eval(chars, params, pos, stack, vars)?;
                if result {
                    output.push_str(then_res.as_str());
                }

                debug_assert!(chars[*pos] == 'e' || chars[*pos] == ';');
                if let Some(is_else) = stack.pop() {
                    if !is_else.as_bool() {
                        *pos += 1;
                        let else_res = __eval(chars, params, pos, stack, vars)?;
                        if !result {
                            output.push_str(else_res.as_str());
                        }

                        if let Some(done_check) = stack.pop() {
                            if !done_check.as_bool() {
                                return Err(EvalError::Invalid(*pos));
                            }
                        }
                    }
                } else {
                    return Err(EvalError::Invalid(*pos));
                }

                if saw_if {
                    stack.push(Param::Number(1));
                    return Ok(output);
                }

                saw_if = false;
            }
            ';' | 'e' => {
                stack.push(Param::Number(match chars[*pos] == ';' {
                    true => 1,
                    false => 0,
                }));
                return Ok(output);
            }
            _ => {
                if [
                    'o', 'X', 'x', ':', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
                ]
                    .contains(&chars[*pos])
                {
                    let mut printf_end = *pos;
                    while printf_end < chars.len() {
                        printf_end += 1;
                        if ['d', 'o', 'x', 'X', 's'].contains(&chars[printf_end]) {
                            break;
                        }
                    }

                    if printf_end >= chars.len() {
                        return Err(EvalError::Invalid(*pos));
                    }

                    let printf_fmt = chars[*pos - 1..printf_end].iter().collect::<String>();
                    if let Some(a) = stack.pop() {
                        let printf_res = match a {
                            Param::Bool(_) | Param::Number(_) => sprintf!(printf_fmt, a.as_int()),
                            Param::Word(_) => sprintf!(printf_fmt, CString::new(a.as_str())),
                        };

                        match printf_res {
                            Ok(res_str) => output.push_str(res_str.as_str()),
                            Err(_) => return Err(EvalError::InvalidPrintf(*pos)),
                        }
                    }
                }
            }
        }

        *pos += 1;
    }

    stack.push(Param::Number(1));
    Ok(output)
}

static CHAR_SUB: fn(char, char) -> u32 = |a: char, b: char| (a as u32) - (b as u32);
static CHAR_LE: fn(char, char) -> bool = |a: char, b: char| (a as u32) <= (b as u32);
static CHAR_GE: fn(char, char) -> bool = |a: char, b: char| (a as u32) >= (b as u32);
static CHAR_BETWEEN: fn(char, char, char) -> bool =
    |a: char, b: char, c: char| CHAR_GE(a, b) && CHAR_LE(a, c);
