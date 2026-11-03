//! The instruction decoder on hand-assembled bytecode.

// Helpers outside #[test] functions unwrap as well: a failed unwrap is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use jarrow_classfile::{Error, Instruction, Instructions, Opcode, Operands, decode_instruction};

fn decode_all(code: &[u8]) -> Vec<Instruction> {
    Instructions::new(code).collect::<Result<_, _>>().unwrap()
}

fn one(code: &[u8]) -> Instruction {
    let insns = decode_all(code);
    assert_eq!(insns.len(), 1, "{code:02X?} decoded to {insns:?}");
    let insn = insns.into_iter().next().unwrap();
    assert_eq!(insn.length as usize, code.len());
    let mut out = Vec::new();
    insn.encode(&mut out).unwrap();
    assert_eq!(out, code, "re-encoding {insn:?}");
    insn
}

#[test]
fn every_opcode_has_a_unique_mnemonic() {
    let mut seen = HashSet::new();
    for byte in 0..=0xC9u8 {
        let op = Opcode::from_u8(byte).unwrap();
        assert_eq!(op.as_u8(), byte);
        assert!(
            seen.insert(op.mnemonic()),
            "duplicate mnemonic {}",
            op.mnemonic()
        );
    }
    assert_eq!(seen.len(), 202);
    assert_eq!(Opcode::INVOKEDYNAMIC.mnemonic(), "invokedynamic");
    assert_eq!(Opcode::JSR_W.as_u8(), 0xC9);
    for byte in 0xCA..=0xFFu8 {
        assert!(Opcode::from_u8(byte).is_none());
        assert_eq!(
            decode_instruction(&[byte], 0),
            Err(Error::BadOpcode {
                offset: 0,
                opcode: byte
            })
        );
    }
}

#[test]
fn opcodes_without_operands() {
    let ranges = [
        0x00..=0x0F,
        0x1A..=0x35,
        0x3B..=0x83,
        0x85..=0x98,
        0xAC..=0xB1,
        0xBE..=0xBF,
        0xC2..=0xC3,
    ];
    let mut count = 0;
    for byte in ranges.into_iter().flatten() {
        let insn = one(&[byte]);
        assert_eq!(insn.operands, Operands::None, "{}", insn.opcode);
        assert!(!insn.wide);
        count += 1;
    }
    // 202 opcodes minus the 54 with operands and the wide prefix.
    assert_eq!(count, 202 - 54 - 1);
}

#[test]
fn immediate_operands() {
    assert_eq!(one(&[0x10, 0xFB]).operands, Operands::Byte(-5));
    assert_eq!(one(&[0x11, 0xFE, 0xD4]).operands, Operands::Short(-300));
    assert_eq!(one(&[0xBC, 10]).operands, Operands::NewArray(10));
}

#[test]
fn constant_pool_operands() {
    let ldc = one(&[0x12, 7]);
    assert_eq!((ldc.opcode, ldc.constant_index()), (Opcode::LDC, Some(7)));
    assert!(ldc.opcode.is_ldc());
    assert_eq!(one(&[0x13, 0x01, 0x2C]).constant_index(), Some(300));
    assert_eq!(one(&[0x14, 0x01, 0x2D]).constant_index(), Some(301));
    for byte in [0xB2, 0xB3, 0xB4, 0xB5] {
        let insn = one(&[byte, 0x00, 0x09]);
        assert!(insn.opcode.is_field_access());
        assert_eq!(insn.operands, Operands::Constant(9));
    }
    for byte in [0xB6, 0xB7, 0xB8] {
        let insn = one(&[byte, 0x12, 0x34]);
        assert!(insn.opcode.is_invoke());
        assert_eq!(insn.constant_index(), Some(0x1234));
    }
    for byte in [0xBB, 0xBD, 0xC0, 0xC1] {
        let insn = one(&[byte, 0x00, 0x05]);
        assert!(insn.opcode.is_type_instruction());
        assert_eq!(insn.constant_index(), Some(5));
    }
    let itf = one(&[0xB9, 0x00, 0x0A, 3, 0]);
    assert_eq!(
        itf.operands,
        Operands::InvokeInterface {
            index: 10,
            count: 3
        }
    );
    assert!(itf.opcode.is_invoke());
    let indy = one(&[0xBA, 0x00, 0x0B, 0, 0]);
    assert_eq!(indy.operands, Operands::InvokeDynamic { index: 11 });
    assert_eq!(indy.constant_index(), Some(11));
    let multi = one(&[0xC5, 0x00, 0x0C, 3]);
    assert_eq!(
        multi.operands,
        Operands::MultiANewArray {
            index: 12,
            dimensions: 3
        }
    );
    assert!(multi.opcode.is_type_instruction());
    assert_eq!(multi.constant_index(), Some(12));
}

#[test]
fn reserved_zero_bytes_must_be_zero() {
    assert_eq!(
        decode_instruction(&[0xB9, 0, 1, 1, 1], 0),
        Err(Error::NonZeroOperand { offset: 0 })
    );
    assert_eq!(
        decode_instruction(&[0xBA, 0, 1, 0, 1], 0),
        Err(Error::NonZeroOperand { offset: 0 })
    );
}

#[test]
fn locals_and_wide() {
    for byte in (0x15..=0x19).chain(0x36..=0x3A).chain([0xA9]) {
        let narrow = one(&[byte, 200]);
        assert_eq!(narrow.operands, Operands::Local(200));
        assert!(!narrow.wide);
        let wide = one(&[0xC4, byte, 0x01, 0x2C]);
        assert_eq!(
            (wide.opcode.as_u8(), wide.wide, wide.length),
            (byte, true, 4)
        );
        assert_eq!(wide.operands, Operands::Local(300));
    }
    assert_eq!(
        one(&[0x84, 3, 0xFF]).operands,
        Operands::Iinc {
            index: 3,
            delta: -1
        }
    );
    let wide_iinc = one(&[0xC4, 0x84, 0x01, 0x2C, 0xFC, 0x18]);
    assert_eq!(
        wide_iinc.operands,
        Operands::Iinc {
            index: 300,
            delta: -1000
        }
    );
    assert_eq!(wide_iinc.length, 6);
    // wide only modifies loads, stores, ret and iinc.
    assert_eq!(
        decode_instruction(&[0xC4, 0x00], 0),
        Err(Error::BadWide {
            offset: 0,
            opcode: 0x00
        })
    );
    assert_eq!(
        decode_instruction(&[0xC4, 0xC4, 0x15, 0, 0], 0),
        Err(Error::BadWide {
            offset: 0,
            opcode: 0xC4
        })
    );
    assert_eq!(
        decode_instruction(&[0xC4, 0xFF], 0),
        Err(Error::BadWide {
            offset: 0,
            opcode: 0xFF
        })
    );
}

#[test]
fn branches() {
    for byte in (0x99..=0xA8).chain([0xC6, 0xC7]) {
        assert_eq!(one(&[byte, 0xFF, 0xFD]).operands, Operands::Branch(-3));
    }
    assert_eq!(
        one(&[0xC8, 0x00, 0x01, 0x86, 0xA0]).operands,
        Operands::Branch(100_000)
    );
    assert_eq!(
        one(&[0xC9, 0xFF, 0xFF, 0xFF, 0xFE]).operands,
        Operands::Branch(-2)
    );
}

/// Builds `nops` × `nop`, then a switch, and returns the code and the switch's offset.
fn after_nops(nops: usize, switch: impl FnOnce(&mut Vec<u8>)) -> (Vec<u8>, usize) {
    let mut code = vec![0x00; nops];
    switch(&mut code);
    (code, nops)
}

#[test]
fn tableswitch_padding_at_every_alignment() {
    for nops in 0..8 {
        let (code, at) = after_nops(nops, |code| {
            code.push(0xAA);
            while code.len() % 4 != 0 {
                code.push(0);
            }
            for v in [100i32, -1, 1, 10, 20, 30] {
                code.extend_from_slice(&v.to_be_bytes());
            }
        });
        let insns = decode_all(&code);
        assert_eq!(insns.len(), nops + 1);
        let switch = &insns[nops];
        assert_eq!(switch.offset as usize, at);
        assert_eq!(switch.length as usize, code.len() - at);
        assert_eq!(switch.length as usize, 1 + (3 - at % 4) + 24);
        assert_eq!(
            switch.operands,
            Operands::TableSwitch {
                default: 100,
                low: -1,
                high: 1,
                offsets: vec![10, 20, 30]
            }
        );
        let mut out = vec![0x00; nops];
        switch.encode(&mut out).unwrap();
        assert_eq!(out, code);
    }
}

#[test]
fn lookupswitch_padding_at_every_alignment() {
    for nops in 0..8 {
        let (code, at) = after_nops(nops, |code| {
            code.push(0xAB);
            while code.len() % 4 != 0 {
                code.push(0);
            }
            for v in [-50i32, 2, 1, 7, 900, 8] {
                code.extend_from_slice(&v.to_be_bytes());
            }
        });
        let insns = decode_all(&code);
        let switch = &insns[nops];
        assert_eq!(switch.offset as usize, at);
        assert_eq!(switch.length as usize, code.len() - at);
        assert_eq!(
            switch.operands,
            Operands::LookupSwitch {
                default: -50,
                pairs: vec![(1, 7), (900, 8)]
            }
        );
        let mut out = vec![0x00; nops];
        switch.encode(&mut out).unwrap();
        assert_eq!(out, code);
    }
}

#[test]
fn malformed_switches_are_errors() {
    // low > high.
    let mut code = vec![0xAA, 0, 0, 0];
    for v in [0i32, 5, 4] {
        code.extend_from_slice(&v.to_be_bytes());
    }
    assert_eq!(
        decode_instruction(&code, 0),
        Err(Error::BadSwitch { offset: 0 })
    );
    // A table claiming 2^32 entries with none present: an error, not a huge allocation.
    let mut code = vec![0xAA, 0, 0, 0];
    for v in [0i32, i32::MIN, i32::MAX] {
        code.extend_from_slice(&v.to_be_bytes());
    }
    assert_eq!(
        decode_instruction(&code, 0),
        Err(Error::TruncatedInstruction { offset: 0 })
    );
    // A negative pair count.
    let mut code = vec![0xAB, 0, 0, 0];
    for v in [0i32, -1] {
        code.extend_from_slice(&v.to_be_bytes());
    }
    assert_eq!(
        decode_instruction(&code, 0),
        Err(Error::BadSwitch { offset: 0 })
    );
    // A pair count larger than the bytes left.
    let mut code = vec![0xAB, 0, 0, 0];
    for v in [0i32, 1000] {
        code.extend_from_slice(&v.to_be_bytes());
    }
    assert_eq!(
        decode_instruction(&code, 0),
        Err(Error::TruncatedInstruction { offset: 0 })
    );
}

#[test]
fn truncated_instructions_are_errors() {
    let complete: [&[u8]; 12] = [
        &[0x10, 1],
        &[0x11, 0, 1],
        &[0x12, 1],
        &[0x13, 0, 1],
        &[0x15, 1],
        &[0x84, 1, 1],
        &[0x99, 0, 3],
        &[0xC8, 0, 0, 0, 5],
        &[0xB9, 0, 1, 1, 0],
        &[0xBA, 0, 1, 0, 0],
        &[0xC5, 0, 1, 1],
        &[0xC4, 0x84, 0, 1, 0, 1],
    ];
    for code in complete {
        one(code);
        for len in 1..code.len() {
            let err = decode_instruction(&code[..len], 0).unwrap_err();
            assert_eq!(
                err,
                Error::TruncatedInstruction { offset: 0 },
                "{:02X?}",
                &code[..len]
            );
        }
    }
    assert_eq!(
        decode_instruction(&[], 0),
        Err(Error::TruncatedInstruction { offset: 0 })
    );
}

#[test]
fn iterator_reports_offsets_and_stops_after_an_error() {
    // iconst_1, bipush 7, an invalid opcode, then a return that is never reached.
    let code = [0x04, 0x10, 7, 0xFE, 0xB1];
    let results: Vec<_> = Instructions::new(&code).collect();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].as_ref().unwrap().offset, 0);
    assert_eq!(results[1].as_ref().unwrap().offset, 1);
    assert_eq!(
        results[2],
        Err(Error::BadOpcode {
            offset: 3,
            opcode: 0xFE
        })
    );
    assert!(Instructions::new(&[]).next().is_none());
}

#[test]
fn encoder_rejects_operands_that_do_not_fit() {
    let insn = |opcode: Opcode, wide: bool, operands: Operands| Instruction {
        offset: 0,
        opcode,
        wide,
        operands,
        length: 0,
    };
    let cases = [
        insn(Opcode::LDC, false, Operands::Constant(256)),
        insn(Opcode::GOTO, false, Operands::Branch(40_000)),
        insn(Opcode::ILOAD, false, Operands::Local(256)),
        insn(
            Opcode::IINC,
            false,
            Operands::Iinc {
                index: 1,
                delta: 200,
            },
        ),
        insn(Opcode::NOP, true, Operands::None),
        insn(Opcode::NOP, false, Operands::Byte(1)),
        insn(
            Opcode::INVOKEVIRTUAL,
            false,
            Operands::InvokeDynamic { index: 1 },
        ),
        insn(
            Opcode::TABLESWITCH,
            false,
            Operands::TableSwitch {
                default: 0,
                low: 0,
                high: 2,
                offsets: vec![1],
            },
        ),
    ];
    for case in cases {
        let mut out = Vec::new();
        assert!(
            matches!(case.encode(&mut out), Err(Error::OperandMismatch { .. })),
            "{case:?}"
        );
    }
}
