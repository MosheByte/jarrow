//! Property tests: inversion, composition, chains and format round trips over generated sets.
//!
//! A generated [`Spec`] describes classes with fields and methods whose descriptors refer to
//! each other, to primitives and to a JDK class. [`build`] turns it into a mapping set over any
//! namespaces, naming every entry after its namespace and position, so the expected result of
//! an inversion or a composition is simply the spec built over the expected namespaces.

// Helpers outside #[test] functions unwrap too: a failure there is a failing test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use jarrow_mappings::{
    FieldMapping, GlobalMemberMap, MappingSet, MethodMapping, ParamMapping, chain, compose,
    proguard, tiny1, tiny2, tsrg,
};
use proptest::collection::vec;
use proptest::option;
use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Base {
    Prim(usize),
    Class(usize),
    Jdk,
}

#[derive(Debug, Clone)]
struct Desc {
    dims: usize,
    base: Base,
}

#[derive(Debug, Clone)]
struct Method {
    args: Vec<Desc>,
    ret: Option<Desc>,
    params: Vec<u8>,
    comment: Option<usize>,
}

#[derive(Debug, Clone)]
struct Class {
    tag: u16,
    inner: bool,
    fields: Vec<Desc>,
    methods: Vec<Method>,
    comment: Option<usize>,
}

#[derive(Debug, Clone)]
struct Spec {
    classes: Vec<Class>,
}

const COMMENTS: [&str; 4] = ["plain", "tab\tand\nbreak", "back\\slash", "nul\0end"];

fn desc() -> impl Strategy<Value = Desc> {
    let base = prop_oneof![
        (0usize..8).prop_map(Base::Prim),
        any::<usize>().prop_map(Base::Class),
        Just(Base::Jdk),
    ];
    (0usize..3, base).prop_map(|(dims, base)| Desc { dims, base })
}

fn method() -> impl Strategy<Value = Method> {
    (
        vec(desc(), 0..3),
        option::of(desc()),
        vec(0u8..6, 0..3),
        option::of(0usize..COMMENTS.len()),
    )
        .prop_map(|(args, ret, params, comment)| Method {
            args,
            ret,
            params,
            comment,
        })
}

fn class() -> impl Strategy<Value = Class> {
    (
        any::<u16>(),
        any::<bool>(),
        vec(desc(), 0..4),
        vec(method(), 0..4),
        option::of(0usize..COMMENTS.len()),
    )
        .prop_map(|(tag, inner, fields, methods, comment)| Class {
            tag,
            inner,
            fields,
            methods,
            comment,
        })
}

fn spec() -> impl Strategy<Value = Spec> {
    vec(class(), 1..7).prop_map(|classes| Spec { classes })
}

#[derive(Clone, Copy)]
struct Extras {
    comments: bool,
    params: bool,
}

const NAMES_ONLY: Extras = Extras {
    comments: false,
    params: false,
};
const EVERYTHING: Extras = Extras {
    comments: true,
    params: true,
};
const PARAMS: Extras = Extras {
    comments: false,
    params: true,
};

impl Spec {
    fn class_name(&self, ns: &str, i: usize) -> String {
        let c = &self.classes[i];
        if c.inner && i > 0 {
            format!("{}${ns}I{i}x{}", self.class_name(ns, i - 1), c.tag)
        } else {
            format!("{ns}/p/C{i}x{}", c.tag)
        }
    }

    fn desc(&self, ns: &str, d: &Desc, mapped: &dyn Fn(usize) -> bool) -> String {
        let mut out = "[".repeat(d.dims);
        match d.base {
            Base::Prim(k) => out.push(char::from(b"BCDFIJSZ"[k])),
            Base::Class(k) => {
                let k = k % self.classes.len();
                let label = if mapped(k) { ns } else { "x" };
                out.push_str(&format!("L{};", self.class_name(label, k)));
            }
            Base::Jdk => out.push_str("Ljava/lang/String;"),
        }
        out
    }

    fn method_desc(&self, ns: &str, m: &Method, mapped: &dyn Fn(usize) -> bool) -> String {
        let mut out = String::from("(");
        for a in &m.args {
            out.push_str(&self.desc(ns, a, mapped));
        }
        out.push(')');
        match &m.ret {
            Some(r) => out.push_str(&self.desc(ns, r, mapped)),
            None => out.push('V'),
        }
        out
    }
}

/// The spec as a mapping set over `namespaces` (first one primary), keeping only the
/// classes `keep` accepts.
fn build(
    spec: &Spec,
    namespaces: &[&str],
    extras: Extras,
    keep: &dyn Fn(usize) -> bool,
) -> MappingSet {
    let mut set = MappingSet::new(namespaces).unwrap();
    let primary = namespaces[0];
    for (i, c) in spec.classes.iter().enumerate() {
        if !keep(i) {
            continue;
        }
        let ci = set
            .add_class(
                namespaces
                    .iter()
                    .map(|ns| Some(spec.class_name(ns, i)))
                    .collect(),
            )
            .unwrap();
        if extras.comments
            && let Some(k) = c.comment
        {
            set.class_at_mut(ci)
                .unwrap()
                .set_comment(Some(COMMENTS[k].to_owned()));
        }
        for (j, f) in c.fields.iter().enumerate() {
            let names = namespaces
                .iter()
                .map(|ns| Some(format!("{ns}_f{j}")))
                .collect();
            let desc = spec.desc(primary, f, &|_| true);
            set.add_field(ci, FieldMapping::new(names, Some(desc)))
                .unwrap();
        }
        for (j, m) in c.methods.iter().enumerate() {
            let names = namespaces
                .iter()
                .map(|ns| Some(format!("{ns}_m{j}")))
                .collect();
            let mut method = MethodMapping::new(names, spec.method_desc(primary, m, &|_| true));
            if extras.comments
                && let Some(k) = m.comment
            {
                method = method.with_comment(COMMENTS[k]);
            }
            if extras.params {
                for &p in &m.params {
                    let names = namespaces
                        .iter()
                        .map(|ns| Some(format!("{ns}_p{p}")))
                        .collect();
                    method = method.with_param(ParamMapping::new(u32::from(p), names));
                }
            }
            set.add_method(ci, method).unwrap();
        }
    }
    set
}

fn all(_: usize) -> bool {
    true
}

proptest! {
    #[test]
    fn inverting_swaps_namespaces(spec in spec()) {
        let ab = build(&spec, &["a", "b"], EVERYTHING, &all);
        let ba = build(&spec, &["b", "a"], EVERYTHING, &all);
        prop_assert_eq!(ab.invert().unwrap(), ba.clone());
        prop_assert_eq!(ba.invert().unwrap().invert().unwrap(), ba);
        let abc = build(&spec, &["a", "b", "c"], EVERYTHING, &all);
        prop_assert_eq!(abc.invert().unwrap(), build(&spec, &["c", "b", "a"], EVERYTHING, &all));
        prop_assert_eq!(abc.with_primary("b").unwrap(), build(&spec, &["b", "a", "c"], EVERYTHING, &all));
    }

    #[test]
    fn mappers_answer_for_every_entry(spec in spec()) {
        let set = build(&spec, &["a", "b"], NAMES_ONLY, &all);
        let forward = set.mapper("b", "a").unwrap();
        for (i, c) in spec.classes.iter().enumerate() {
            let owner = spec.class_name("b", i);
            let target = spec.class_name("a", i);
            prop_assert_eq!(forward.map_class(&owner), Some(target.as_str()));
            for (j, f) in c.fields.iter().enumerate() {
                let d = spec.desc("b", f, &|_| true);
                let expected = format!("a_f{j}");
                prop_assert_eq!(forward.map_field(&owner, &format!("b_f{j}"), Some(&d)), Some(expected.as_str()));
                prop_assert_eq!(forward.map_field_desc(&d).unwrap(), spec.desc("a", f, &|_| true));
            }
            for (j, m) in c.methods.iter().enumerate() {
                let d = spec.method_desc("b", m, &|_| true);
                let expected = format!("a_m{j}");
                prop_assert_eq!(forward.map_method(&owner, &format!("b_m{j}"), &d), Some(expected.as_str()));
                prop_assert_eq!(forward.map_method_desc(&d).unwrap(), spec.method_desc("a", m, &|_| true));
            }
        }
    }

    #[test]
    fn composition_maps_through_the_shared_namespace(spec in spec(), mask in vec(any::<bool>(), 7)) {
        let in_b = |i: usize| mask[i];
        let xs = build(&spec, &["x", "s"], NAMES_ONLY, &all);
        let sy = build(&spec, &["s", "y"], NAMES_ONLY, &in_b);
        let joined = compose(&xs, &sy, "s").unwrap();
        prop_assert_eq!(joined.namespaces(), ["s", "x", "y"]);
        let xy = joined.project(&["x", "y"]).unwrap();
        prop_assert_eq!(&chain("x", "y", &[&xs, &sy]).unwrap(), &xy);
        let mapper = xy.mapper("x", "y").unwrap();
        for (i, c) in spec.classes.iter().enumerate() {
            let owner = spec.class_name("x", i);
            let expected = in_b(i).then(|| spec.class_name("y", i));
            prop_assert_eq!(mapper.map_class(&owner), expected.as_deref());
            for (j, m) in c.methods.iter().enumerate() {
                let d = spec.method_desc("x", m, &|_| true);
                let expected = in_b(i).then(|| format!("y_m{j}"));
                prop_assert_eq!(mapper.map_method(&owner, &format!("x_m{j}"), &d), expected.as_deref());
                // Classes with no y name keep their x name in remapped descriptors.
                prop_assert_eq!(mapper.map_method_desc(&d).unwrap(), spec.method_desc("y", m, &in_b));
            }
        }
    }

    #[test]
    fn chains_of_three_equal_the_direct_set(spec in spec()) {
        let wx = build(&spec, &["w", "x"], EVERYTHING, &all);
        let xy = build(&spec, &["x", "y"], EVERYTHING, &all);
        let yz = build(&spec, &["y", "z"], EVERYTHING, &all);
        let direct = build(&spec, &["w", "z"], EVERYTHING, &all);
        // Order of the sets does not matter, and sets off the path are ignored.
        let unrelated = build(&spec, &["q", "r"], NAMES_ONLY, &all);
        prop_assert_eq!(&chain("w", "z", &[&yz, &unrelated, &wx, &xy]).unwrap(), &direct);
        let left = compose(&compose(&wx, &xy, "x").unwrap().project(&["y", "w"]).unwrap(), &yz, "y")
            .unwrap()
            .project(&["w", "z"])
            .unwrap();
        let right = compose(&wx, &compose(&xy, &yz, "y").unwrap().project(&["x", "z"]).unwrap(), "x")
            .unwrap()
            .project(&["w", "z"])
            .unwrap();
        prop_assert_eq!(&left, &direct);
        prop_assert_eq!(&right, &direct);
        prop_assert_eq!(chain("z", "w", &[&wx, &xy, &yz]).unwrap(), direct.invert().unwrap());
    }

    #[test]
    fn every_format_round_trips(spec in spec()) {
        let full = build(&spec, &["obf", "srg", "id"], EVERYTHING, &all);
        prop_assert_eq!(&tiny2::parse(&tiny2::write(&full).unwrap()).unwrap(), &full);
        // TSRG2 holds parameters but no comments.
        let with_params = build(&spec, &["obf", "srg", "id"], PARAMS, &all);
        prop_assert_eq!(&tsrg::parse_v2(&tsrg::write_v2(&with_params).unwrap()).unwrap(), &with_params);
        let names_only = build(&spec, &["obf", "srg", "id"], NAMES_ONLY, &all);
        prop_assert_eq!(&tiny1::parse(&tiny1::write(&names_only).unwrap()).unwrap(), &names_only);
        let two = build(&spec, &["mojang", "obf"], NAMES_ONLY, &all);
        prop_assert_eq!(&proguard::parse_mojang(&proguard::write_mojang(&two).unwrap()).unwrap(), &two);
        // ProGuard from the other namespace order goes through a projection.
        let pg = proguard::write(&two.invert().unwrap(), "mojang", "obf").unwrap();
        prop_assert_eq!(&proguard::parse_mojang(&pg).unwrap(), &two);
        // TSRG v1 has no field descriptors.
        let srg = build(&spec, &["obf", "srg"], NAMES_ONLY, &all);
        let back = tsrg::parse_v1(&tsrg::write_v1(&srg).unwrap(), ["obf", "srg"]).unwrap();
        for c in srg.classes() {
            for f in c.fields() {
                let other = back.field(c.primary(), f.primary(), None).unwrap();
                prop_assert_eq!(other.name(1), f.name(1));
                prop_assert_eq!(other.desc(), None);
            }
            for m in c.methods() {
                prop_assert_eq!(back.method(c.primary(), m.primary(), m.desc()).unwrap().name(1), m.name(1));
            }
        }
    }

    #[test]
    fn global_member_map_has_no_conflicts_for_consistent_names(spec in spec()) {
        let set = build(&spec, &["intermediary", "mojang"], NAMES_ONLY, &all);
        let map = GlobalMemberMap::build(&set, "intermediary", "mojang").unwrap();
        prop_assert!(map.conflicts().is_empty());
        for c in set.classes() {
            for f in c.fields() {
                prop_assert_eq!(map.map_field(f.primary()), f.name(1));
            }
            for m in c.methods() {
                prop_assert_eq!(map.map_method(m.primary()), m.name(1));
            }
        }
    }
}
