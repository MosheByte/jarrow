//! Fuzz target: arbitrary bytes through the class-file parser, writer and decoder.
//!
//! Beyond "no panic", it checks the crate's round-trip promise on every input that
//! parses: writing the parsed class must give back exactly the input bytes. It also runs
//! the instruction decoder over every `Code` attribute and the lossless MUTF-8 decoder over
//! every `Utf8` constant.

#![no_main]

use jarrow_classfile::{ClassFile, Constant, mutf8};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(class) = ClassFile::parse(data) else {
        return;
    };
    let written = class.to_bytes().expect("a parsed class always writes");
    assert_eq!(written, data, "parse -> write must reproduce the input");

    for (_, constant) in class.constant_pool.iter() {
        if let Constant::Utf8(raw) = constant {
            assert_eq!(&mutf8::encode_pieces(&mutf8::decode_lossless(raw)), raw);
            let _ = mutf8::decode(raw);
        }
    }
    for method in &class.methods {
        let Some(code) = method.code() else { continue };
        let mut covered = 0usize;
        for insn in code.instructions() {
            let Ok(insn) = insn else { break };
            assert_eq!(insn.offset as usize, covered);
            covered += insn.length as usize;
            let mut encoded = Vec::new();
            insn.encode(&mut encoded).expect("a decoded instruction always encodes");
            assert_eq!(encoded.len(), insn.length as usize);
        }
        assert!(covered <= code.code.len());
    }
    let _ = class.this_class_name();
    let _ = class.super_class_name();
});
