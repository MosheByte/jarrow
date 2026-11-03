//! Every format against its hand-written fixture: what the parser reads, and that writing and
//! reading back gives the same set (and the same text the second time).

// Helpers outside #[test] functions unwrap too: a failure there is a failing test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use jarrow_mappings::{
    Error, FieldMapping, Format, LineRange, MappingSet, MethodMapping, ParamMapping, detect,
    intermediary, names, proguard, tiny1, tiny2, tsrg,
};

const TINY_V2: &str = include_str!("fixtures/sample-v2.tiny");
const TINY_V2_ESCAPED: &str = include_str!("fixtures/escaped-v2.tiny");
const TINY_V1: &str = include_str!("fixtures/sample-v1.tiny");
const PROGUARD: &str = include_str!("fixtures/sample-proguard.txt");
const TSRG: &str = include_str!("fixtures/sample.tsrg");
const TSRG2: &str = include_str!("fixtures/sample.tsrg2");

/// Parses, writes, parses again; the sets must match, and a second write must reproduce the
/// first one byte for byte.
fn round_trip(
    set: &MappingSet,
    write: impl Fn(&MappingSet) -> jarrow_mappings::Result<String>,
    parse: impl Fn(&str) -> jarrow_mappings::Result<MappingSet>,
) -> String {
    let text = write(set).unwrap();
    let again = parse(&text).unwrap();
    assert_eq!(
        &again, set,
        "round trip changed the set; written text:\n{text}"
    );
    assert_eq!(write(&again).unwrap(), text, "writer output is not stable");
    text
}

#[test]
fn detects_every_fixture() {
    assert_eq!(detect(TINY_V2), Some(Format::TinyV2));
    assert_eq!(detect(TINY_V1), Some(Format::TinyV1));
    assert_eq!(detect(PROGUARD), Some(Format::Proguard));
    assert_eq!(detect(TSRG), Some(Format::Tsrg));
    assert_eq!(detect(TSRG2), Some(Format::Tsrg2));
}

#[test]
fn tiny_v2_reads_everything() {
    let set = tiny2::parse(TINY_V2).unwrap();
    assert_eq!(set.namespaces(), ["official", "intermediary", "named"]);
    assert_eq!(
        set.properties(),
        [(
            "jarrow-fixture".to_owned(),
            Some("hand-written for jarrow-mappings tests".to_owned())
        )]
    );
    let widget = set.class("a").unwrap();
    assert_eq!(widget.name(2), Some("net/minecraft/demo/Widget"));
    assert_eq!(
        widget.comment(),
        Some("A widget.\nIts second line has a tab:\there and a backslash: \\.")
    );
    assert_eq!(
        set.field("a", "a", Some("I")).unwrap().comment(),
        Some("How big it is.")
    );
    let combine = set.method("a", "a", "(Lb;I)La;").unwrap();
    assert_eq!(combine.name(1), Some("method_1"));
    assert_eq!(combine.params().len(), 2);
    let gadget = combine.param(1).unwrap();
    assert_eq!(
        (gadget.name(0), gadget.name(1), gadget.name(2)),
        (None, None, Some("gadget"))
    );
    assert_eq!(gadget.comment(), Some("The gadget to add."));
    let vars = combine.vars();
    assert_eq!(vars.len(), 2);
    assert_eq!(
        (
            vars[0].lv_index(),
            vars[0].start_offset(),
            vars[0].lvt_index()
        ),
        (3, 12, None)
    );
    assert_eq!(vars[1].lvt_index(), Some(2));
    assert_eq!(vars[1].comment(), Some("Counts loops."));
    // An empty name is a missing name.
    assert_eq!(
        set.method("b", "b", "([Ljava/lang/String;J)[[Z")
            .unwrap()
            .name(2),
        None
    );
    assert_eq!(
        set.class("a$a").unwrap().name(2),
        Some("net/minecraft/demo/Widget$Part")
    );
    let stats = set.stats();
    assert_eq!(
        (
            stats.classes,
            stats.fields,
            stats.methods,
            stats.params,
            stats.vars
        ),
        (3, 3, 4, 2, 2)
    );
    round_trip(&set, tiny2::write, tiny2::parse);
}

#[test]
fn tiny_v2_escaped_names() {
    let set = tiny2::parse(TINY_V2_ESCAPED).unwrap();
    let class = set.class("odd\\name").unwrap();
    assert_eq!(class.name(1), Some("odd\tname"));
    assert_eq!(
        set.field("odd\\name", "f\\1", Some("I")).unwrap().name(1),
        Some("f\n1")
    );
    let text = round_trip(&set, tiny2::write, tiny2::parse);
    assert!(text.contains("\n\tescaped-names\n"));
    // Without escaped-names, backslashes in names are literal.
    let plain = tiny2::parse("tiny\t2\t0\ta\tb\nc\tx\\ty\tz\n").unwrap();
    assert_eq!(plain.class("x\\ty").unwrap().name(1), Some("z"));
    round_trip(&plain, tiny2::write, tiny2::parse);
}

#[test]
fn tiny_v2_accepts_crlf_and_a_bom() {
    let crlf = format!("\u{feff}{}", TINY_V2.replace('\n', "\r\n"));
    assert_eq!(tiny2::parse(&crlf).unwrap(), tiny2::parse(TINY_V2).unwrap());
    assert_eq!(
        tiny2::parse_bytes(crlf.as_bytes()).unwrap(),
        tiny2::parse(TINY_V2).unwrap()
    );
}

#[test]
fn tiny_v1_reads_and_round_trips() {
    let set = tiny1::parse(TINY_V1).unwrap();
    assert_eq!(set.namespaces(), ["official", "intermediary"]);
    assert_eq!(
        set.field("a", "b", Some("Lb;")).unwrap().name(1),
        Some("field_2")
    );
    // A member whose owner has no CLASS line still gets its class, unnamed elsewhere.
    let c = set.class("c").unwrap();
    assert_eq!(c.name(1), None);
    assert_eq!(
        set.method("c", "a", "()V").unwrap().name(1),
        Some("method_4")
    );
    round_trip(&set, tiny1::write, tiny1::parse);
    // tiny v1 and tiny v2 agree on the names they share.
    let v2 = tiny2::parse(TINY_V2)
        .unwrap()
        .project(&["official", "intermediary"])
        .unwrap();
    for class in v2.classes() {
        let other = set.class(class.primary()).unwrap();
        assert_eq!(other.name(1), class.name(1));
    }
}

#[test]
fn tiny_v1_cannot_write_what_it_cannot_hold() {
    let mut set = MappingSet::new(["a", "b"]).unwrap();
    let c = set.add_class(names(["x", "y"])).unwrap();
    set.add_field(c, FieldMapping::new(names(["f", "g"]), None))
        .unwrap();
    assert!(matches!(
        tiny1::write(&set),
        Err(Error::Unrepresentable { .. })
    ));
    assert!(matches!(
        tiny2::write(&set),
        Err(Error::Unrepresentable { .. })
    ));
    let mut tabs = MappingSet::new(["a", "b"]).unwrap();
    tabs.add_class(names(["x", "y\tz"])).unwrap();
    assert!(tiny1::write(&tabs).is_err());
    assert!(tiny2::write(&tabs).is_ok());
}

#[test]
fn proguard_reads_lines_inner_classes_and_inline_frames() {
    let set = proguard::parse_mojang(PROGUARD).unwrap();
    assert_eq!(set.namespaces(), ["mojang", "obf"]);
    let widget = set.class("net/minecraft/demo/Widget").unwrap();
    assert_eq!(widget.name(1), Some("a"));
    assert_eq!(
        set.field("net/minecraft/demo/Widget", "gadget", None)
            .unwrap()
            .desc(),
        Some("Lnet/minecraft/demo/Gadget;")
    );
    let combine = set
        .method(
            "net/minecraft/demo/Widget",
            "combine",
            "(Lnet/minecraft/demo/Gadget;I)Lnet/minecraft/demo/Widget;",
        )
        .unwrap();
    assert_eq!(combine.name(1), Some("a"));
    let ranges: Vec<_> = combine.lines().iter().map(|r| (r.start, r.end)).collect();
    assert_eq!(ranges, [(15, 15), (16, 18)]);
    let tick = set
        .method("net/minecraft/demo/Widget", "tick", "()V")
        .unwrap();
    assert_eq!(
        tick.lines(),
        [LineRange {
            start: 20,
            end: 20,
            original_start: Some(40),
            original_end: Some(42)
        }]
    );
    let frames = widget.inline_frames();
    assert_eq!(frames.len(), 1);
    assert_eq!(
        (
            frames[0].owner.as_str(),
            frames[0].name.as_str(),
            frames[0].obfuscated.as_str()
        ),
        ("net/minecraft/demo/Gadget", "helper", "c")
    );
    assert!(
        set.method("net/minecraft/demo/Widget", "helper", "()V")
            .is_none()
    );
    let to_string = set
        .method(
            "net/minecraft/demo/Widget",
            "toString",
            "()Ljava/lang/String;",
        )
        .unwrap();
    assert!(to_string.lines().is_empty());
    assert_eq!(
        set.method(
            "net/minecraft/demo/Gadget",
            "flags",
            "([Ljava/lang/String;J)[[Z"
        )
        .unwrap()
        .name(1),
        Some("b")
    );
    let part = set.class("net/minecraft/demo/Widget$Part").unwrap();
    assert_eq!(part.name(1), Some("a$a"));
    assert_eq!(
        set.field("net/minecraft/demo/Widget$Part", "matrix", Some("[[J"))
            .unwrap()
            .name(1),
        Some("a")
    );

    let text = round_trip(&set, proguard::write_mojang, proguard::parse_mojang);
    assert!(text.contains("    20:20:void tick():40:42 -> c\n"));
    assert!(text.contains("    21:21:void net.minecraft.demo.Gadget.helper():7:7 -> c\n"));
    assert!(text.contains("net.minecraft.demo.Widget$Part -> a$a:\n"));
}

#[test]
fn proguard_writes_from_any_namespace_order() {
    let set = proguard::parse_mojang(PROGUARD).unwrap();
    let obf_first = set.invert().unwrap();
    assert_eq!(obf_first.namespaces(), ["obf", "mojang"]);
    let text = proguard::write(&obf_first, "mojang", "obf").unwrap();
    let back = proguard::parse_mojang(&text).unwrap();
    // Projection keeps line ranges but drops inline frames.
    let expected = obf_first.invert().unwrap();
    assert_eq!(back, expected);
    assert!(
        back.class("net/minecraft/demo/Widget")
            .unwrap()
            .inline_frames()
            .is_empty()
    );
    assert_eq!(back.stats(), set.stats());
}

#[test]
fn proguard_cannot_write_missing_names() {
    let mut set = MappingSet::new(["mojang", "obf"]).unwrap();
    set.add_class(names(["a/B", ""])).unwrap();
    assert!(matches!(
        proguard::write_mojang(&set),
        Err(Error::Unrepresentable { .. })
    ));
    let mut dotted = MappingSet::new(["mojang", "obf"]).unwrap();
    dotted.add_class(names(["a.B", "c"])).unwrap();
    assert!(proguard::write_mojang(&dotted).is_err());
}

#[test]
fn tsrg_reads_and_round_trips() {
    let set = tsrg::parse_v1(TSRG, ["obf", "srg"]).unwrap();
    assert_eq!(set.packages().len(), 1);
    assert_eq!(set.packages()[0].primary(), "net/minecraft");
    assert_eq!(
        set.class("a").unwrap().name(1),
        Some("net/minecraft/demo/Widget")
    );
    let f = set.field("a", "b", None).unwrap();
    assert_eq!((f.name(1), f.desc()), (Some("f_2_b"), None));
    assert_eq!(
        set.method("a", "a", "(Lb;I)La;").unwrap().name(1),
        Some("m_1_a")
    );
    round_trip(&set, tsrg::write_v1, |t| tsrg::parse_v1(t, ["obf", "srg"]));
    assert!(tsrg::write_v1(&MappingSet::new(["a", "b", "c"]).unwrap()).is_err());
}

#[test]
fn tsrg2_reads_and_round_trips() {
    let set = tsrg::parse_v2(TSRG2).unwrap();
    assert_eq!(set.namespaces(), ["obf", "srg", "id"]);
    assert_eq!(set.packages()[0].names(), names(["net", "net", "net"]));
    assert_eq!(set.field("a", "a", None).unwrap().desc(), None);
    assert_eq!(set.field("a", "b", None).unwrap().desc(), Some("Lb;"));
    let m = set.method("a", "a", "(Lb;I)La;").unwrap();
    assert!(m.is_static());
    assert_eq!(m.name(2), Some("4"));
    assert_eq!(m.param(1).unwrap().names(), names(["o", "p_1_1_", "6"]));
    round_trip(&set, tsrg::write_v2, tsrg::parse_v2);
    // TSRG2 needs every name.
    let mut gap = MappingSet::new(["obf", "srg"]).unwrap();
    gap.add_class(names(["a", ""])).unwrap();
    assert!(tsrg::write_v2(&gap).is_err());
}

#[test]
fn intermediary_fixture_loads_as_obf_and_intermediary() {
    let set = intermediary::load(TINY_V2.as_bytes()).unwrap();
    assert_eq!(set.namespaces(), ["obf", "intermediary", "named"]);
    let v1 = intermediary::load(TINY_V1.as_bytes()).unwrap();
    assert_eq!(v1.namespaces(), ["obf", "intermediary"]);
    assert_eq!(intermediary::JAR_ENTRY, "mappings/mappings.tiny");
}

#[test]
fn every_writer_handles_a_hand_built_set() {
    let mut set = MappingSet::new(["obf", "srg"]).unwrap();
    let a = set.add_class(names(["a", "pkg/Alpha"])).unwrap();
    set.add_field(
        a,
        FieldMapping::new(names(["b", "count"]), Some("I".into())),
    )
    .unwrap();
    set.add_method(
        a,
        MethodMapping::new(names(["c", "run"]), "(I)V")
            .with_param(ParamMapping::new(1, names(["x", "times"]))),
    )
    .unwrap();
    round_trip(&set, tiny2::write, tiny2::parse);
    round_trip(&set, tsrg::write_v2, tsrg::parse_v2);
    // tiny v1, ProGuard and TSRG have no parameters; the names still come through.
    let v1 = tiny1::parse(&tiny1::write(&set).unwrap()).unwrap();
    assert_eq!(v1.method("a", "c", "(I)V").unwrap().name(1), Some("run"));
    let pg = proguard::parse(&proguard::write(&set, "srg", "obf").unwrap(), "srg", "obf").unwrap();
    assert_eq!(
        pg.method("pkg/Alpha", "run", "(I)V").unwrap().name(1),
        Some("c")
    );
    let t1 = tsrg::parse_v1(&tsrg::write_v1(&set).unwrap(), ["obf", "srg"]).unwrap();
    assert_eq!(t1.field("a", "b", None).unwrap().name(1), Some("count"));
}
