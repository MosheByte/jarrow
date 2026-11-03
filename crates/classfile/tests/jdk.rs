//! Round trip on the JDK class library: every class in `$JAVA_HOME/jmods/*.jmod` must
//! parse, write back byte for byte, and have bytecode that decodes and re-encodes exactly.
//!
//! The JDK is found through `JAVA_HOME`, then through `java` on `PATH`. Without one the
//! test prints a skip note and passes, unless `JARROW_REQUIRE_JDK=1`, in which case it
//! fails. CI sets `JARROW_REQUIRE_JDK=1` after installing Temurin 21.
//!
//! A jmod file is the bytes `JM 01 00` followed by a zip archive; the zip reader below
//! handles the stored and deflate methods and zip64, which is all a jmod uses.

// Helpers outside #[test] functions unwrap as well: a failed unwrap is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs, thread};

use jarrow_classfile::{
    Attribute, AttributeBody, ClassFile, Constant, ConstantPool, Opcode, mutf8,
};

#[test]
fn jdk_classes_round_trip() {
    let required = env::var("JARROW_REQUIRE_JDK").is_ok_and(|v| v.trim() == "1");
    let Some(jmods) = find_jmods() else {
        assert!(
            !required,
            "JARROW_REQUIRE_JDK=1 but no JDK with a jmods directory was found (JAVA_HOME or java on PATH)"
        );
        eprintln!(
            "skipped: no JDK with a jmods directory found (set JAVA_HOME, or JARROW_REQUIRE_JDK=1 to fail)"
        );
        return;
    };
    let mut files: Vec<(u64, PathBuf)> = fs::read_dir(&jmods)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "jmod"))
        .map(|p| (fs::metadata(&p).map(|m| m.len()).unwrap_or(0), p))
        .collect();
    assert!(
        !files.is_empty(),
        "{} holds no .jmod files",
        jmods.display()
    );
    // Largest first, so java.base does not end up last on one thread.
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let totals = Mutex::new(Totals::default());
    let next = AtomicUsize::new(0);
    let threads = thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(files.len());
    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((_, path)) = files.get(i) else { break };
                    let mut local = Totals::default();
                    check_jmod(path, &mut local);
                    totals.lock().unwrap().merge(local);
                }
            });
        }
    });
    let totals = totals.into_inner().unwrap();

    eprintln!(
        "{} jmods, {} classes, {} methods with code, {} instructions",
        files.len(),
        totals.classes,
        totals.code,
        totals.instructions
    );
    for (name, count) in &totals.attributes {
        eprintln!("  {count:>8}  {name}");
    }
    if !totals.failures.is_empty() {
        let shown: Vec<_> = totals.failures.iter().take(25).collect();
        panic!(
            "{} of {} classes failed:\n{}",
            totals.failures.len(),
            totals.classes,
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    assert!(
        totals.classes >= 1000,
        "only {} classes found in {}",
        totals.classes,
        jmods.display()
    );
    // Every typed attribute a JDK 21 build is known to contain was parsed as typed; a typo
    // in a name would silently turn it into Unknown, so check they all occur.
    for name in [
        "Code",
        "StackMapTable",
        "Exceptions",
        "InnerClasses",
        "EnclosingMethod",
        "Signature",
        "SourceFile",
        "LineNumberTable",
        "ConstantValue",
        "Deprecated",
        "RuntimeVisibleAnnotations",
        "AnnotationDefault",
        "BootstrapMethods",
        "NestHost",
        "NestMembers",
        "Record",
        "PermittedSubclasses",
    ] {
        assert!(
            totals.attributes.contains_key(name),
            "no typed {name} attribute in the JDK"
        );
    }
}

#[derive(Default)]
struct Totals {
    classes: usize,
    code: usize,
    instructions: usize,
    attributes: BTreeMap<String, usize>,
    failures: Vec<String>,
}

impl Totals {
    fn merge(&mut self, other: Totals) {
        self.classes += other.classes;
        self.code += other.code;
        self.instructions += other.instructions;
        for (k, v) in other.attributes {
            *self.attributes.entry(k).or_default() += v;
        }
        self.failures.extend(other.failures);
    }
}

fn find_jmods() -> Option<PathBuf> {
    let mut homes = Vec::new();
    if let Some(home) = env::var_os("JAVA_HOME").filter(|h| !h.is_empty()) {
        homes.push(PathBuf::from(home));
    }
    if let Some(path) = env::var_os("PATH") {
        for dir in env::split_paths(&path) {
            for exe in ["java", "java.exe"] {
                let candidate = dir.join(exe);
                if candidate.is_file() {
                    // bin/java -> the JDK home, through any symlinks (/usr/bin/java on Linux).
                    if let Some(home) = fs::canonicalize(&candidate)
                        .ok()
                        .and_then(|p| Some(p.parent()?.parent()?.to_path_buf()))
                    {
                        homes.push(home);
                    }
                }
            }
        }
    }
    homes.into_iter().map(|h| h.join("jmods")).find(|j| {
        fs::read_dir(j).is_ok_and(|mut it| {
            it.any(|e| e.is_ok_and(|e| e.path().extension().is_some_and(|x| x == "jmod")))
        })
    })
}

fn check_jmod(path: &Path, totals: &mut Totals) {
    let label = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            totals.failures.push(format!("{label}: cannot read: {e}"));
            return;
        }
    };
    let result = for_each_entry(&data, |name, bytes| {
        if name.starts_with("classes/") && name.ends_with(".class") {
            totals.classes += 1;
            if let Err(why) = check_class(&bytes, totals) {
                totals.failures.push(format!("{label}!{name}: {why}"));
            }
        }
    });
    if let Err(why) = result {
        totals.failures.push(format!("{label}: {why}"));
    }
}

fn check_class(bytes: &[u8], totals: &mut Totals) -> Result<(), String> {
    let class = ClassFile::parse(bytes).map_err(|e| format!("parse: {e}"))?;
    let written = class.to_bytes().map_err(|e| format!("write: {e}"))?;
    if written != bytes {
        let at = written
            .iter()
            .zip(bytes)
            .position(|(a, b)| a != b)
            .unwrap_or(written.len().min(bytes.len()));
        return Err(format!(
            "written bytes differ at byte {at} (lengths {} and {})",
            written.len(),
            bytes.len()
        ));
    }
    let pool = &class.constant_pool;
    for (index, constant) in pool.iter() {
        if let Constant::Utf8(raw) = constant
            && mutf8::encode_pieces(&mutf8::decode_lossless(raw)) != *raw
        {
            return Err(format!("Utf8 #{index} does not survive decode_lossless"));
        }
    }
    for list in all_attribute_lists(&class) {
        for attribute in list {
            let name = match attribute.body.name() {
                Some(name) => name.to_owned(),
                None => format!(
                    "(raw) {}",
                    mutf8::decode_lossy(pool.utf8(attribute.name_index).unwrap_or(b"?"))
                ),
            };
            *totals.attributes.entry(name).or_default() += 1;
        }
    }
    for method in &class.methods {
        let Some(code) = method.code() else { continue };
        totals.code += 1;
        let mut reencoded = Vec::with_capacity(code.code.len());
        for insn in code.instructions() {
            let insn = insn.map_err(|e| format!("decode: {e}"))?;
            totals.instructions += 1;
            check_operand_kind(pool, insn.opcode, insn.constant_index())
                .map_err(|e| format!("{} at {}: {e}", insn.opcode, insn.offset))?;
            insn.encode(&mut reencoded)
                .map_err(|e| format!("encode: {e}"))?;
        }
        if reencoded != code.code {
            return Err("re-encoded bytecode differs from the original".to_owned());
        }
    }
    Ok(())
}

/// The constant an instruction names must be of the kind its opcode needs. A decoder that
/// read operands from the wrong bytes would fail this long before the JDK ran out.
fn check_operand_kind(
    pool: &ConstantPool,
    opcode: Opcode,
    index: Option<u16>,
) -> Result<(), String> {
    let Some(index) = index else { return Ok(()) };
    let constant = pool
        .get(index)
        .ok_or_else(|| format!("constant #{index} does not exist"))?;
    let ok = match opcode {
        Opcode::LDC | Opcode::LDC_W => matches!(
            constant,
            Constant::Integer(_)
                | Constant::Float(_)
                | Constant::String { .. }
                | Constant::Class { .. }
                | Constant::MethodType { .. }
                | Constant::MethodHandle { .. }
                | Constant::Dynamic { .. }
        ),
        Opcode::LDC2_W => matches!(
            constant,
            Constant::Long(_) | Constant::Double(_) | Constant::Dynamic { .. }
        ),
        Opcode::GETSTATIC | Opcode::PUTSTATIC | Opcode::GETFIELD | Opcode::PUTFIELD => {
            matches!(constant, Constant::Fieldref { .. })
        }
        Opcode::INVOKEVIRTUAL => matches!(constant, Constant::Methodref { .. }),
        Opcode::INVOKESPECIAL | Opcode::INVOKESTATIC => {
            matches!(
                constant,
                Constant::Methodref { .. } | Constant::InterfaceMethodref { .. }
            )
        }
        Opcode::INVOKEINTERFACE => matches!(constant, Constant::InterfaceMethodref { .. }),
        Opcode::INVOKEDYNAMIC => matches!(constant, Constant::InvokeDynamic { .. }),
        Opcode::NEW
        | Opcode::ANEWARRAY
        | Opcode::CHECKCAST
        | Opcode::INSTANCEOF
        | Opcode::MULTIANEWARRAY => {
            matches!(constant, Constant::Class { .. })
        }
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!("constant #{index} is a {}", constant.kind_name()))
    }
}

fn all_attribute_lists(class: &ClassFile) -> Vec<&[Attribute]> {
    let mut lists: Vec<&[Attribute]> = vec![&class.attributes];
    lists.extend(class.fields.iter().map(|f| f.attributes.as_slice()));
    lists.extend(class.methods.iter().map(|m| m.attributes.as_slice()));
    let mut nested = Vec::new();
    for list in &lists {
        for attribute in *list {
            match &attribute.body {
                AttributeBody::Code(code) => nested.push(code.attributes.as_slice()),
                AttributeBody::Record(components) => {
                    nested.extend(components.iter().map(|c| c.attributes.as_slice()))
                }
                _ => {}
            }
        }
    }
    lists.extend(nested);
    lists
}

// ------------------------------------------------------------------ a minimal jmod reader

fn le16(data: &[u8], at: usize) -> Result<u16, String> {
    data.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| format!("truncated at {at}"))
}

fn le32(data: &[u8], at: usize) -> Result<u32, String> {
    data.get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("truncated at {at}"))
}

fn le64(data: &[u8], at: usize) -> Result<u64, String> {
    let b = data
        .get(at..at + 8)
        .ok_or_else(|| format!("truncated at {at}"))?;
    Ok(u64::from_le_bytes(
        b.try_into().map_err(|_| "bad slice".to_owned())?,
    ))
}

fn to_usize(v: u64) -> Result<usize, String> {
    usize::try_from(v).map_err(|_| format!("{v} does not fit usize"))
}

/// Calls `visit(name, bytes)` for every file entry of a jmod.
fn for_each_entry(file: &[u8], mut visit: impl FnMut(&str, Vec<u8>)) -> Result<(), String> {
    let zip = file
        .strip_prefix(b"JM\x01\x00")
        .ok_or("not a jmod: missing the JM 01 00 header")?;
    let eocd = (0..=zip.len().saturating_sub(22))
        .rev()
        .take(22 + 65_535)
        .find(|&i| zip[i..].starts_with(b"PK\x05\x06"))
        .ok_or("no end of central directory record")?;
    let mut entries = u64::from(le16(zip, eocd + 10)?);
    let mut cd_offset = u64::from(le32(zip, eocd + 16)?);
    if entries == 0xFFFF || cd_offset == 0xFFFF_FFFF {
        let locator = eocd.checked_sub(20).ok_or("no zip64 locator")?;
        if !zip[locator..].starts_with(b"PK\x06\x07") {
            return Err("missing zip64 end of central directory locator".to_owned());
        }
        let record = to_usize(le64(zip, locator + 8)?)?;
        if !zip
            .get(record..)
            .is_some_and(|r| r.starts_with(b"PK\x06\x06"))
        {
            return Err("missing zip64 end of central directory record".to_owned());
        }
        entries = le64(zip, record + 32)?;
        cd_offset = le64(zip, record + 48)?;
    }
    let mut at = to_usize(cd_offset)?;
    for _ in 0..entries {
        if !zip.get(at..).is_some_and(|r| r.starts_with(b"PK\x01\x02")) {
            return Err(format!("bad central directory entry at {at}"));
        }
        let method = le16(zip, at + 10)?;
        let mut compressed = u64::from(le32(zip, at + 20)?);
        let mut uncompressed = u64::from(le32(zip, at + 24)?);
        let name_len = usize::from(le16(zip, at + 28)?);
        let extra_len = usize::from(le16(zip, at + 30)?);
        let comment_len = usize::from(le16(zip, at + 32)?);
        let mut local = u64::from(le32(zip, at + 42)?);
        let name_bytes = zip
            .get(at + 46..at + 46 + name_len)
            .ok_or("truncated name")?;
        let name = String::from_utf8_lossy(name_bytes).into_owned();
        let extra = zip
            .get(at + 46 + name_len..at + 46 + name_len + extra_len)
            .ok_or("truncated extra field")?;
        // Zip64 extended information: the 64-bit values replace the 0xFFFFFFFF ones, in order.
        let mut e = 0;
        while e + 4 <= extra.len() {
            let id = le16(extra, e)?;
            let size = usize::from(le16(extra, e + 2)?);
            if id == 1 {
                let mut p = e + 4;
                for value in [&mut uncompressed, &mut compressed, &mut local] {
                    if *value == 0xFFFF_FFFF {
                        *value = le64(extra, p)?;
                        p += 8;
                    }
                }
            }
            e += 4 + size;
        }
        at += 46 + name_len + extra_len + comment_len;
        if name.ends_with('/') {
            continue;
        }
        let local = to_usize(local)?;
        if !zip
            .get(local..)
            .is_some_and(|r| r.starts_with(b"PK\x03\x04"))
        {
            return Err(format!("{name}: bad local header"));
        }
        let start =
            local + 30 + usize::from(le16(zip, local + 26)?) + usize::from(le16(zip, local + 28)?);
        let raw = zip
            .get(start..start + to_usize(compressed)?)
            .ok_or_else(|| format!("{name}: truncated data"))?;
        let bytes = match method {
            0 => raw.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(raw, to_usize(uncompressed)?)
                .map_err(|e| format!("{name}: inflate failed: {e:?}"))?,
            other => return Err(format!("{name}: unsupported compression method {other}")),
        };
        if bytes.len() as u64 != uncompressed {
            return Err(format!(
                "{name}: inflated to {} bytes, expected {uncompressed}",
                bytes.len()
            ));
        }
        visit(&name, bytes);
    }
    Ok(())
}
