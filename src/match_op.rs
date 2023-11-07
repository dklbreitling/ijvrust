use std::{
    io::{stdout, Read, Write},
    num::Wrapping,
};

use debug_print::{debug_eprint as deprint, debug_eprintln as deprintln};

use ascii::ToAsciiChar;

use crate::{Byte, Machine, OpError, Word};

const BIPUSH: Byte = 0x10;
const DUP: Byte = 0x59;
const IADD: Byte = 0x60;
const IAND: Byte = 0x7E;
const IOR: Byte = 0xB0;
const ISUB: Byte = 0x64;
const NOP: Byte = 0x00;
const POP: Byte = 0x57;
const SWAP: Byte = 0x5F;
const ERR: Byte = 0xFE;
const HALT: Byte = 0xFF;
const IN: Byte = 0xFC;
const OUT: Byte = 0xFD;
const GOTO: Byte = 0xA7;
const IFEQ: Byte = 0x99;
const IFLT: Byte = 0x9B;
const IF_ICMPEQ: Byte = 0x9F;
const LDC_W: Byte = 0x13;
const ILOAD: Byte = 0x15;
const ISTORE: Byte = 0x36;
const IINC: Byte = 0x84;
const WIDE: Byte = 0xC4;
const INVOKEVIRTUAL: Byte = 0xB6;
const IRETURN: Byte = 0xAC;

pub fn match_op_code(op_code: Byte) -> String {
    return match op_code {
        0x10 => String::from("BIPUSH"),
        0x59 => String::from("DUP"),
        0x60 => String::from("IADD"),
        0x7E => String::from("IAND"),
        0xB0 => String::from("IOR"),
        0x64 => String::from("ISUB"),
        0x00 => String::from("NOP"),
        0x57 => String::from("POP"),
        0x5F => String::from("SWAP"),
        0xFE => String::from("ERR"),
        0xFF => String::from("HALT"),
        0xFC => String::from("IN"),
        0xFD => String::from("OUT"),
        0xA7 => String::from("GOTO"),
        0x99 => String::from("IFEQ"),
        0x9B => String::from("IFLT"),
        0x9F => String::from("IF_ICMPEQ"),
        0x13 => String::from("LDC_W"),
        0x15 => String::from("ILOAD"),
        0x36 => String::from("ISTORE"),
        0x84 => String::from("IINC"),
        0xC4 => String::from("WIDE"),
        0xB6 => String::from("INVOKEVIRTUAL"),
        0xAC => String::from("IRETURN"),
        _ => String::from("invalid, likely arg"),
    };
}

fn _two_operand_instruction_common(
    machine: &mut Machine,
    operation: fn(a: Word, b: Word) -> Word,
    instruction: String,
) -> Result<(), OpError> {
    let a = pop_safe(machine, instruction.clone())?; //as i8;
    let b = pop_safe(machine, instruction.clone())?; //as i8;
    let res = operation(a, b);
    machine.stack.push(res as Word);
    return Ok(());
}

fn two_operand_instruction_common(machine: &mut Machine, op_code: Byte) -> Result<(), OpError> {
    let a = Wrapping(pop_safe(machine, match_op_code(op_code))?); //as i8;
    let b = Wrapping(pop_safe(machine, match_op_code(op_code))?); //as i8;
    machine.stack.push(match op_code {
        IADD => (a + b).0,
        ISUB => (b - a).0,
        IAND => (a & b).0,
        IOR => (a | b).0,
        _ => return Err(OpError::GenericError(())),
    });
    return Ok(());
}

pub fn do_op(op_code: Byte, machine: &mut Machine) -> Result<(), OpError> {
    let mut ret: Result<(), OpError> = Ok(());
    match op_code {
        BIPUSH => {
            machine
                .stack
                .push((machine.text[machine.pc as usize] as i8) as Word);
            machine.pc += 1;
        }
        DUP => {
            machine.stack.push(machine.stack.top()?);
        }
        IADD => two_operand_instruction_common(machine, op_code)?,
        IAND => two_operand_instruction_common(machine, op_code)?,
        IOR => two_operand_instruction_common(machine, op_code)?,
        ISUB => two_operand_instruction_common(machine, op_code)?,
        NOP => (),
        POP => {
            pop_safe(machine, String::from("POP"))?;
        }
        SWAP => {
            let a = pop_safe(machine, match_op_code(op_code))?;
            let b = pop_safe(machine, match_op_code(op_code))?;
            machine.stack.push(a as Word);
            machine.stack.push(b as Word);
        }
        ERR => {
            machine.halt_msg = String::from("ERR reached.");
            machine.halt = true;
        }
        HALT => {
            machine.halt_msg = String::from("HALT reached.");
            machine.halt = true;
        }
        IN => {
            let mut inb: Vec<Byte> = vec![0; 1];
            match std::io::stdin().read_exact(&mut inb) {
                Ok(_) => {
                    if inb[0] as char == '\n' {
                        deprintln!("IN: read newline (i.e. EOF), pushing 0");
                        machine.stack.push(0);
                    } else {
                        machine.stack.push(inb[0] as Word)
                    }
                }
                Err(e) => {
                    machine.halt_msg = String::from(format!("IN: Error {e} when reading."));
                    ret = Err(OpError::IoError(e));
                }
            }
            deprintln!(
                "\tIN: read {} with tos = {}",
                inb[0] as Word,
                machine.stack.top()?
            )
        }
        OUT => {
            let c = pop_safe(machine, match_op_code(op_code))? as u8;

            unsafe {
                print!("{}", c.to_ascii_char_unchecked());
                stdout().flush()?;
            }
        }
        GOTO => {
            let offset = get_short_offset(machine) as Word - 1;
            machine.pc += offset;
        } // account for step incrementing PC
        IFEQ => {
            if pop_safe(machine, match_op_code(op_code))? == 0 {
                do_op(GOTO, machine)?;
            } else {
                machine.pc += 2;
            }
        }
        IFLT => {
            if (pop_safe(machine, match_op_code(op_code))?) < 0 {
                do_op(GOTO, machine)?;
            } else {
                machine.pc += 2;
            }
        }
        IF_ICMPEQ => {
            let a = pop_safe(machine, match_op_code(op_code))?;
            let b = pop_safe(machine, match_op_code(op_code))?;
            deprintln!(
                "IF_ICMPEQ: a = {} (hex {:#02x}), b = {} (hex {:#02x})",
                a,
                a,
                b,
                b
            );
            if a == b {
                do_op(GOTO, machine)?;
            } else {
                machine.pc += 2;
            }
        }
        LDC_W => {
            let i = get_short_offset(machine);
            let c = get_constant(machine, i)?;
            machine.stack.push(c);
            machine.pc += 2;
        }
        ILOAD => {
            machine.stack._eprint_upto(0);
            let i = machine.text[machine.pc as usize] as u8;
            load_lv(machine, i)?;
            machine.pc += 1;
            machine.stack._eprint_upto(0);
        }
        ISTORE => {
            machine.stack._eprint_upto(0);
            let i = machine.text[machine.pc as usize] as u8;
            store_lv(machine, i)?;
            machine.pc += 1;
            machine.stack._eprint_upto(0);
        }
        IINC => {
            // TODO: LVs are now practically reduced to i8 size, fix?!
            let i = machine.text[machine.pc as usize] as u8;
            machine.pc += 1;
            let val = machine.text[machine.pc as usize] as i8;
            machine.pc += 1;
            deprint!(
                "IINC: LV index {i} (= {} (hex {:#02x})) + {} (hex {:#02x})",
                _get_lv(machine, i) as i8,
                _get_lv(machine, i) as i8,
                val,
                val
            );
            let lv_i = calc_lv_index(machine, i);
            machine.stack[lv_i] = machine.stack[lv_i] + val as Word;
            deprintln!(
                ", now {} (hex {:#02x}).",
                _get_lv(machine, i) as i8,
                _get_lv(machine, i) as i8
            );
        }
        WIDE => (),
        INVOKEVIRTUAL => {
            machine.stack._eprint_upto(255);

            let old_lv = machine.stack.lv;
            let old_pc = machine.pc + 2;

            let i = get_short_offset(machine);
            let method_pc = get_constant(machine, i)? - 1;
            machine.pc = method_pc + 1;

            // OBJREF is counted in num_args, but replaced by link pointer.
            let num_args = get_short_offset(machine); // TODO: bug somewhere here
            machine.pc += 2;

            let num_lv = get_short_offset(machine);
            machine.pc += 2;

            machine.stack.lv = machine.stack.sp - num_args as usize + 1; // + 1;

            // First make space for LVs then push old lv + pc
            machine.stack.sp += num_lv as usize; // + 1 for objref

            machine.stack.push(old_pc);

            // Link Pointer points to previous PC
            let lv = machine.stack.lv as Word;
            machine.stack.push(old_lv as Word);

            machine.stack[lv] = machine.stack.sp as Word - 1;

            machine.stack._eprint_upto(255);
        }
        IRETURN => {
            machine.stack._eprint_upto(255);

            let return_value = pop_safe(machine, String::from("IRETURN"))?;
            let lv = machine.stack.lv as Word;
            let link_ptr = machine.stack[lv];
            let ret_pc = machine.stack[link_ptr];
            let ret_lv = machine.stack[link_ptr + 1] as usize;

            // Restore program counter.
            machine.pc = ret_pc;

            // Restore stack.
            machine.stack.sp = machine.stack.lv;

            machine.stack.lv = ret_lv;

            // Link pointer of returning function needs to be popped.
            pop_safe(machine, String::from("IRETURN"))?;
            // Return value should be placed on top of calling context's stack.
            machine.stack.push(return_value);

            machine.stack._eprint_upto(255);
        }
        _ => {
            machine.halt_msg = String::from(format!(
                "Error: op_code {:#02x} unknown or not implemented.",
                op_code
            ));
            ret = Err(OpError::GenericError(()));
        }
    }
    return ret;
}

fn pop_safe(machine: &mut Machine, instruction: String) -> Result<Word, OpError> {
    match machine.stack.pop() {
        Ok(val) => return Ok(val),
        Err(OpError::EmptyStackError(_)) => {
            machine.halt_msg =
                String::from(format!("Error: Calling {instruction} on empty stack."));
            return Err(OpError::EmptyStackError(()));
        }
        Err(e) => {
            machine.halt_msg = String::from(format!(
                "Error: Unknown error popping in {instruction} instruction."
            ));
            return Err(e);
        }
    }
}

fn get_short_offset(machine: &Machine) -> i16 {
    let ptr: usize = machine.pc as usize;
    return ((machine.text[(ptr + 1) as usize] as i16) << 0) | ((machine.text[ptr] as i16) << 8);
}

fn load_lv(machine: &mut Machine, index: u8) -> Result<(), OpError> {
    // TODO: make sure LV is actually stored before
    let index = calc_lv_index(machine, index);
    let val = machine.stack[index];
    machine.stack.push(val);
    return Ok(());
}

fn store_lv(machine: &mut Machine, index: u8) -> Result<(), OpError> {
    // TODO: make sure there is enough LV space
    let val = pop_safe(machine, String::from("ISTORE"))?;
    let index = calc_lv_index(machine, index);
    machine.stack[index] = val;
    return Ok(());
}

fn calc_lv_index(machine: &mut Machine, index: u8) -> Word {
    (machine.stack.lv + index as usize) as Word + if machine.stack.lv == 0 { 1 } else { 0 }
}

fn _get_lv(machine: &mut Machine, index: u8) -> Word {
    let index = calc_lv_index(machine, index);
    return machine.stack[index];
}

fn get_constant(machine: &mut Machine, index: i16) -> Result<Word, OpError> {
    let index = (index * 4) as usize;
    let res: Result<Word, OpError> = if (index) < machine.constant_pool.len() {
        Ok(((machine.constant_pool[index + 3] as Word) << 0)
            | ((machine.constant_pool[index + 2] as Word) << 8)
            | ((machine.constant_pool[index + 1] as Word) << 16)
            | ((machine.constant_pool[index + 0] as Word) << 24))
    } else {
        machine.halt_msg =
            String::from("Error: Attempting to get constant with index out of bounds.");
        Err(OpError::GenericError(()))
    };
    return res;
}
