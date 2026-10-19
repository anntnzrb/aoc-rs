//! A six-register background program with instruction-pointer-directed control flow.

pub mod primary;

use crate::device::{DeviceError, Program};

/// Parses a bound instruction-pointer program.
///
/// # Errors
/// Returns an error for malformed instructions or invalid register operands.
pub fn parse(input: &str) -> Result<Program, DeviceError> {
    Program::parse(input)
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::device::{DeviceError, Machine};

    #[test]
    fn empty_input_is_rejected() {
        assert!(
            matches!(parse(""), Err(DeviceError::Syntax(_))),
            "no declaration"
        );
    }

    #[test]
    fn lines_keep_their_order() {
        let program = parse("#ip 5\nseti 7 0 0\nmuli 0 3 0").unwrap();
        let mut machine = Machine::default();
        while program.step(&mut machine).unwrap() {}
        assert_eq!(machine.registers[0], 21, "program order");
    }

    #[test]
    fn lines_borrow_the_input_and_strip_crlf() {
        let program = parse("#ip 5\r\nseti 3 999 0\r\n").unwrap();
        let mut machine = Machine::default();
        assert!(program.step(&mut machine).unwrap(), "CRLF parses");
        assert_eq!(machine.registers[0], 3, "owned parsed instruction");
        assert!(!program.step(&mut machine).unwrap(), "one instruction");
    }
}
