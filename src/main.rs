pub mod match_op;
use crate::match_op::*;

use std::env;
use std::fmt::Display;
use std::fs::{self};
use std::ops::{Index, IndexMut};

use debug_print::{debug_eprint as deprint, debug_eprintln as deprintln};

type Word = i32;
type Byte = u8;

pub struct Stack {
    data: Vec<Word>,
    sp: usize,
    lv: usize,
}

impl Stack {
    fn pop(&mut self) -> Result<Word, OpError> {
        if self.is_empty() {
            deprint!("\t\tWARN: Popping from empty stack!")
        }
        let ret = self.top();
        self.sp -= 1;
        return ret;
    }

    /*
     * LV points to link ptr,
     * link ptr points to caller's PC,
     * above caller's PC is caller's LV,
     * above caller's LV is callee's stack.
     */
    fn is_empty(&self) -> bool {
        let link_ptr = self.data[self.lv];
        let cond = self.sp <= link_ptr as usize + 1;
        if self.sp < link_ptr as usize + 1 {
            deprint!("\t\tWARN: SP below LINK PTR + 1!")
        }

        return cond;
    }

    fn top(&self) -> Result<Word, OpError> {
        return Ok(self.data[self.sp]);
        // return if self.sp >= (self.data[self.lv] as usize) {
        //     //TODO: maybe self.data[self.data[self.lv]]?
        //     Ok(self.data[self.sp])
        // } else {
        //     Err(OpError::EmptyStackError(()))
        // };
    }

    fn push(&mut self, val: Word) {
        self.sp += 1;
        self.data[self.sp] = val;
    }

    fn _eprint(&mut self) {
        deprint!(
            "\tStack: SP={} LV={} LINK_PTR={} [",
            self.sp,
            self.lv,
            self.data[self.lv]
        );
        if self.sp <= self.data[self.lv] as usize + 1 {
            deprint!("]");
            if self.sp < self.data[self.lv] as usize + 1 {
                deprint!("\t\tWARN: SP below LINK PTR + 1!")
            }
            deprint!("\n");
            return;
        }
        let old_sp = self.sp;
        loop {
            let _val = match self.pop() {
                Ok(v) => v,
                Err(_) => break,
            };
            if self.is_empty() {
                self.sp = old_sp;
                deprintln!("{}].", _val);
                break;
            }
            deprint!("{}, ", _val);
        }
    }

    fn _eprint_upto(&mut self, i: usize) {
        deprint!(
            "\tStack up to {i}: SP={} LV={} LINK_PTR={} [",
            self.sp,
            self.lv,
            self.data[self.lv]
        );
        let old_sp = self.sp;
        let mut halt = false;
        loop {
            let _val = self.data[self.sp];
            if self.sp > 0 {
                self.sp -= 1
            } else {
                halt = true;
            };
            if self.sp < i || halt {
                self.sp = old_sp;
                deprintln!("{_val}({:#02x})].", _val);
                break;
            }
            deprint!("{_val}({:#02x}), ", _val);
        }
    }

    fn _eprint_hex(&mut self) {
        deprint!(
            "\tHex stack: SP={:#02x} LV={:#02x} LINK_PTR={:#02x} [",
            self.sp,
            self.lv,
            self.data[self.lv]
        );
        if self.sp <= self.data[self.lv] as usize + 1 {
            deprint!("]");
            if self.sp < self.data[self.lv] as usize + 1 {
                deprint!("\t\tWARN: SP below LINK PTR + 1!")
            }
            deprint!("\n");
            return;
        }
        let old_sp = self.sp;
        loop {
            let _val = match self.pop() {
                Ok(v) => v,
                Err(_) => break,
            };
            if self.is_empty() {
                self.sp = old_sp;
                deprintln!("{:#02x}].", _val);
                break;
            }
            deprint!("{:#02x}, ", _val);
        }
    }
}

impl Index<Word> for Stack {
    type Output = Word;

    fn index(&self, index: Word) -> &Self::Output {
        return &self.data[index as usize];
    }
}

impl IndexMut<Word> for Stack {
    fn index_mut(&mut self, index: Word) -> &mut Self::Output {
        return &mut self.data[index as usize];
    }
}

pub struct Machine {
    stack: Stack,
    pc: i32,
    text: Vec<Byte>,
    text_size: Word,
    constant_pool: Vec<Byte>,
    halt: bool,
    halt_msg: String,
}

const MB: usize = 262144; // number of words in a MB is 2^20 / 4
const MAIN_LINK_PTR: Word = 257;
const STACK_SIZE: usize = 1000 * MB;

#[derive(Debug)]
pub enum OpError {
    IoError(std::io::Error),
    EmptyStackError(()),
    GenericError(()),
}

impl Display for OpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpError::IoError(e) => write!(f, "{}", e),
            OpError::GenericError(_) => write!(f, "OpError"),
            OpError::EmptyStackError(_) => write!(f, "EmptyStackError"),
        }
    }
}

impl From<std::io::Error> for OpError {
    fn from(e: std::io::Error) -> Self {
        OpError::IoError(e)
    }
}

impl From<()> for OpError {
    fn from(_: ()) -> Self {
        OpError::GenericError(())
    }
}

impl std::error::Error for OpError {}

fn get_big_endian_word(buf: &Vec<Byte>, start_ptr: &mut usize) -> Word {
    let w = ((buf[*start_ptr + 3] as Word) << 0)
        | ((buf[*start_ptr + 2] as Word) << 8)
        | ((buf[*start_ptr + 1] as Word) << 16)
        | ((buf[*start_ptr + 0] as Word) << 24);

    *start_ptr = *start_ptr + 4;
    return w;
}

fn main() {
    if cfg!(debug_assertions) {
        eprintln!("Debugging enabled.\n");
    } else {
        eprintln!("Debugging disabled.\n");
    }

    env::set_var("RUST_BACKTRACE", "1");

    deprintln!("Hello, file reading!");

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("No argument provided, exiting. Please provide an input file.");
        return;
    }

    let file_path = &args[1];

    deprintln!("In file {}", file_path);

    let contents: Vec<Byte> = fs::read(file_path).expect("Couldn't read contents");

    deprintln!(
        "Contents are {:02x?}, length is {}",
        contents,
        contents.len()
    );

    const MAGIC: Word = 0x1deadfad;
    let mut text_ptr: usize = 0;
    let magic_num: Word = get_big_endian_word(&contents, &mut text_ptr);

    deprintln!("MAGIC valid? {}.", MAGIC == magic_num);

    if magic_num != MAGIC {
        return;
    }

    let _cp_origin: Word = get_big_endian_word(&contents, &mut text_ptr);
    let cp_size: Word = get_big_endian_word(&contents, &mut text_ptr);
    deprint!("cp_origin {:08x?}; cp_size {:08x?}; ", _cp_origin, cp_size);
    let cp_data: Vec<Byte> = Vec::from(&contents[text_ptr..text_ptr + (cp_size as usize)]);
    text_ptr += cp_size as usize;
    deprintln!(" cp_data {:02x?}", cp_data);

    let _text_origin: Word = get_big_endian_word(&contents, &mut text_ptr);
    let text_size: Word = get_big_endian_word(&contents, &mut text_ptr);
    let text_data: Vec<Byte> = Vec::from(&contents[text_ptr..text_ptr + (text_size as usize)]);
    deprintln!(
        "text_origin {:#08x?}; text_size {:#08x?}; text_data {:02x?}",
        _text_origin,
        text_size,
        text_data
    );

    let mut machine = Machine {
        text: text_data,
        text_size: text_size,
        pc: 0,
        stack: Stack {
            data: vec![0; STACK_SIZE], // TODO: keep track of which LV's have been stored?!
            lv: 0,
            sp: MAIN_LINK_PTR as usize + 1,
        },
        constant_pool: cp_data,
        halt: false,
        halt_msg: String::from("Generic Error."),
    };

    let lv = machine.stack.lv as i32;
    machine.stack[lv] = MAIN_LINK_PTR;

    loop {
        if machine.halt {
            deprintln!("Halting machine. Reason: {}", machine.halt_msg);
            break;
        }

        step(&mut machine);
    }
}

fn step(machine: &mut Machine) {
    let cur_op: Byte = machine.text[machine.pc as usize];
    machine.pc += 1;
    deprint!("At PC {}: {}", machine.pc - 1, match_op_code(cur_op));
    match do_op(cur_op, machine) {
        Ok(_) => (),
        Err(_e) => {
            deprint!("ERROR: {_e}");
            machine.halt = true
        }
    };

    let _val = match machine.stack.top() {
        Ok(val) => val,
        Err(_) => {
            deprint!("\ttos: Unexpected Error.");
            machine.halt = true;
            return;
        }
    };

    if machine.stack.is_empty() {
        deprint!("\tstack.is_empty() true");
    } else {
        deprint!(
            "\ttos: SP={}, top of stack is {_val} (hex {:#02x}).",
            machine.stack.sp,
            _val
        );
    }

    // machine.stack._eprint();

    deprintln!();

    if machine.pc >= machine.text_size {
        machine.halt = true;
        machine.halt_msg = String::from("End of text reached.");
    }
}
