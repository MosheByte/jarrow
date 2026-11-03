//! The Fabric to NeoForge chain (intermediary -> obf -> mojang) on the hand-written fixtures,
//! with descriptor and signature remapping and the global member map.

// Helpers outside #[test] functions unwrap too: a failure there is a failing test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use jarrow_mappings::{
    GlobalLookup, GlobalMemberMap, MappingSet, chain, intermediary, is_intermediary_member_name,
    ns, proguard,
};

const INTERMEDIARY: &[u8] = include_bytes!("fixtures/sample-v2.tiny");
const MOJANG: &str = include_str!("fixtures/sample-proguard.txt");

fn fabric_to_neoforge() -> MappingSet {
    let intermediary = intermediary::load(INTERMEDIARY).unwrap();
    let mojang = proguard::parse_mojang(MOJANG).unwrap();
    chain(ns::INTERMEDIARY, ns::MOJANG, &[&intermediary, &mojang]).unwrap()
}

#[test]
fn classes_fields_and_methods() {
    let set = fabric_to_neoforge();
    assert_eq!(set.namespaces(), ["intermediary", "mojang"]);
    let m = set.mapper(ns::INTERMEDIARY, ns::MOJANG).unwrap();
    assert_eq!(
        m.map_class("net/minecraft/class_1"),
        Some("net/minecraft/demo/Widget")
    );
    assert_eq!(
        m.map_class("net/minecraft/class_2"),
        Some("net/minecraft/demo/Gadget")
    );
    assert_eq!(
        m.map_class("net/minecraft/class_1$class_3"),
        Some("net/minecraft/demo/Widget$Part")
    );
    assert_eq!(m.map_class("com/example/ModClass"), None);
    assert_eq!(
        m.map_field("net/minecraft/class_1", "field_1", Some("I")),
        Some("size")
    );
    assert_eq!(
        m.map_field(
            "net/minecraft/class_1",
            "field_2",
            Some("Lnet/minecraft/class_2;")
        ),
        Some("gadget")
    );
    assert_eq!(
        m.map_field("net/minecraft/class_1$class_3", "field_3", None),
        Some("matrix")
    );
    assert_eq!(
        m.map_method(
            "net/minecraft/class_1",
            "method_1",
            "(Lnet/minecraft/class_2;I)Lnet/minecraft/class_1;"
        ),
        Some("combine")
    );
    assert_eq!(
        m.map_method(
            "net/minecraft/class_2",
            "method_2",
            "()Lnet/minecraft/class_2;"
        ),
        Some("copy")
    );
    assert_eq!(
        m.map_method(
            "net/minecraft/class_2",
            "method_3",
            "([Ljava/lang/String;J)[[Z"
        ),
        Some("flags")
    );
    // Members are stored with intermediary descriptors after the chain.
    let combine = set
        .method(
            "net/minecraft/class_1",
            "method_1",
            "(Lnet/minecraft/class_2;I)Lnet/minecraft/class_1;",
        )
        .unwrap();
    assert_eq!(combine.name(1), Some("combine"));
    // Mojang lists toString; the intermediary fixture does not, so it is not in the chain (it
    // keeps its name anyway). The fixture does list the constructor.
    assert!(
        set.method("net/minecraft/class_1", "toString", "()Ljava/lang/String;")
            .is_none()
    );
    assert_eq!(
        m.map_method("net/minecraft/class_1", "<init>", "()V"),
        Some("<init>")
    );
    // tick() is in Mojang's file only, under obf name c.
    assert!(
        set.class("net/minecraft/class_1")
            .unwrap()
            .methods_named("c")
            .next()
            .is_none()
    );
}

#[test]
fn descriptors_and_signatures() {
    let set = fabric_to_neoforge();
    let m = set.mapper(ns::INTERMEDIARY, ns::MOJANG).unwrap();
    assert_eq!(
        m.map_method_desc(
            "(Lnet/minecraft/class_2;[Lnet/minecraft/class_1$class_3;Lcom/example/Own;)V"
        )
        .unwrap(),
        "(Lnet/minecraft/demo/Gadget;[Lnet/minecraft/demo/Widget$Part;Lcom/example/Own;)V"
    );
    assert_eq!(
        m.map_field_desc("[[Lnet/minecraft/class_1;").unwrap(),
        "[[Lnet/minecraft/demo/Widget;"
    );
    assert_eq!(
        m.map_signature("Ljava/util/Map<Lnet/minecraft/class_2;+Ljava/util/List<-Lnet/minecraft/class_1$class_3;>;>;")
            .unwrap(),
        "Ljava/util/Map<Lnet/minecraft/demo/Gadget;+Ljava/util/List<-Lnet/minecraft/demo/Widget$Part;>;>;"
    );
    assert_eq!(
        m.map_signature("<T:Lnet/minecraft/class_1;>(TT;Lnet/minecraft/class_1<TT;>.class_3;)V^Ljava/io/IOException;")
            .unwrap(),
        "<T:Lnet/minecraft/demo/Widget;>(TT;Lnet/minecraft/demo/Widget<TT;>.Part;)V^Ljava/io/IOException;"
    );
    assert_eq!(
        m.map_signature("Ljava/lang/Object;Ljava/lang/Comparable<Lnet/minecraft/class_2;>;")
            .unwrap(),
        "Ljava/lang/Object;Ljava/lang/Comparable<Lnet/minecraft/demo/Gadget;>;"
    );
    assert_eq!(
        m.map_class_or_array("[Lnet/minecraft/class_2;").unwrap(),
        "[Lnet/minecraft/demo/Gadget;"
    );
    for bad in [
        "(Lnet/minecraft/class_2",
        "Lnet/minecraft/class_1<>;",
        "[",
        "",
    ] {
        assert!(m.map_signature(bad).is_err(), "{bad:?} should fail");
    }
}

#[test]
fn global_member_map_over_the_chain() {
    let set = fabric_to_neoforge();
    let map = GlobalMemberMap::build_filtered(
        &set,
        ns::INTERMEDIARY,
        ns::MOJANG,
        is_intermediary_member_name,
    )
    .unwrap();
    assert!(map.conflicts().is_empty(), "{:?}", map.conflicts());
    assert_eq!(map.map_field("field_3"), Some("matrix"));
    assert_eq!(map.map_method("method_3"), Some("flags"));
    assert_eq!(map.method("method_99"), GlobalLookup::Missing);
}

#[test]
fn obfuscated_names_are_not_globally_unique() {
    let mojang = proguard::parse_mojang(MOJANG).unwrap();
    let map = GlobalMemberMap::build(&mojang, ns::OBF, ns::MOJANG).unwrap();
    let GlobalLookup::Conflict(field_a) = map.field("a") else {
        panic!("obf field a should conflict");
    };
    let targets: Vec<_> = field_a
        .targets
        .iter()
        .map(|t| (t.owner.as_str(), t.target.as_str()))
        .collect();
    assert_eq!(targets, [("a", "size"), ("a$a", "matrix")]);
    assert!(matches!(map.method("a"), GlobalLookup::Conflict(_)));
    assert_eq!(map.map_method("toString"), Some("toString"));
}

#[test]
fn the_chain_runs_in_either_order_and_direction() {
    let intermediary = intermediary::load(INTERMEDIARY).unwrap();
    let mojang = proguard::parse_mojang(MOJANG).unwrap();
    let forward = chain(ns::INTERMEDIARY, ns::MOJANG, &[&mojang, &intermediary]).unwrap();
    assert_eq!(forward, fabric_to_neoforge());
    let back = chain(ns::MOJANG, ns::INTERMEDIARY, &[&intermediary, &mojang]).unwrap();
    let m = back.mapper(ns::MOJANG, ns::INTERMEDIARY).unwrap();
    assert_eq!(
        m.map_method(
            "net/minecraft/demo/Widget",
            "combine",
            "(Lnet/minecraft/demo/Gadget;I)Lnet/minecraft/demo/Widget;"
        ),
        Some("method_1")
    );
    // Yarn's column rides along in the fixture and is reachable too.
    let yarn = chain("named", ns::MOJANG, &[&intermediary, &mojang]).unwrap();
    assert_eq!(
        yarn.mapper("named", ns::MOJANG).unwrap().map_field(
            "net/minecraft/demo/Widget",
            "size",
            None
        ),
        Some("size")
    );
}
