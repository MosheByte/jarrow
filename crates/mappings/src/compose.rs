//! Composition of mapping sets through a shared namespace, and chains across several sets.

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};

use crate::error::{Error, Result};
use crate::model::{
    ClassMapping, FieldMapping, MappingSet, MethodMapping, Names, ParamMapping, VarMapping,
};

/// Joins two sets on a namespace they share.
///
/// The result's namespaces are `shared`, then the other namespaces of `a` in order, then the
/// other namespaces of `b` in order; a namespace other than `shared` that both sets have is
/// an [`Error::DuplicateNamespace`]. Entries are matched by their name in `shared` (members
/// by name and descriptor in `shared`). An entry found on one side only keeps its names from
/// that side and has no names in the other side's namespaces.
///
/// For `a = (obf, intermediary)` and `b = (mojang, obf)`, `compose(a, b, "obf")` gives
/// `(obf, intermediary, mojang)`; [`MappingSet::project`] then picks the direction.
pub fn compose(a: &MappingSet, b: &MappingSet, shared: &str) -> Result<MappingSet> {
    let a = primary_on(a, shared)?;
    let b = primary_on(b, shared)?;
    let mut namespaces: Vec<&str> = vec![shared];
    for ns in a.namespaces()[1..].iter().chain(&b.namespaces()[1..]) {
        if namespaces.contains(&ns.as_str()) {
            return Err(Error::DuplicateNamespace(ns.clone()));
        }
        namespaces.push(ns);
    }
    let na = a.namespaces().len() - 1;
    let nb = b.namespaces().len() - 1;
    let join =
        |left: Option<&[Option<String>]>, right: Option<&[Option<String>]>, key: &str| -> Names {
            let mut names = Vec::with_capacity(1 + na + nb);
            names.push(Some(key.to_owned()));
            match left {
                Some(l) => names.extend(l[1..].iter().cloned()),
                None => names.extend(std::iter::repeat_n(None, na)),
            }
            match right {
                Some(r) => names.extend(r[1..].iter().cloned()),
                None => names.extend(std::iter::repeat_n(None, nb)),
            }
            names
        };

    let mut out = MappingSet::new(&namespaces)?;
    let mut b_packages_used = vec![false; b.packages().len()];
    for p in a.packages() {
        let other = b.packages().iter().position(|q| q.primary() == p.primary());
        if let Some(i) = other {
            b_packages_used[i] = true;
        }
        let right = other.map(|i| b.packages()[i].names());
        out.add_package(join(Some(p.names()), right, p.primary()))?;
    }
    for (i, q) in b.packages().iter().enumerate() {
        if !b_packages_used[i] {
            out.add_package(join(None, Some(q.names()), q.primary()))?;
        }
    }

    let mut b_used = vec![false; b.classes().len()];
    for ac in a.classes() {
        let bi = b.class_position(ac.primary());
        if let Some(i) = bi {
            b_used[i] = true;
        }
        let bc = bi.and_then(|i| b.class_at(i));
        join_class(&mut out, Some(ac), bc, &join)?;
    }
    for (i, bc) in b.classes().iter().enumerate() {
        if !b_used[i] {
            join_class(&mut out, None, Some(bc), &join)?;
        }
    }
    Ok(out)
}

type Join<'j> = dyn Fn(Option<&[Option<String>]>, Option<&[Option<String>]>, &str) -> Names + 'j;

fn join_class(
    out: &mut MappingSet,
    ac: Option<&ClassMapping>,
    bc: Option<&ClassMapping>,
    join: &Join<'_>,
) -> Result<()> {
    let Some(key) = ac.or(bc).map(ClassMapping::primary) else {
        return Ok(());
    };
    let ci = out.add_class(join(
        ac.map(ClassMapping::names),
        bc.map(ClassMapping::names),
        key,
    ))?;
    let comment = ac
        .and_then(ClassMapping::comment)
        .or_else(|| bc.and_then(ClassMapping::comment));
    if let (Some(comment), Some(class)) = (comment, out.class_at_mut(ci)) {
        class.set_comment(Some(comment.to_owned()));
    }

    let empty_fields: &[FieldMapping] = &[];
    let empty_methods: &[MethodMapping] = &[];
    let b_fields = bc.map_or(empty_fields, ClassMapping::fields);
    let b_methods = bc.map_or(empty_methods, ClassMapping::methods);

    let mut b_field_used = vec![false; b_fields.len()];
    for af in ac.map_or(empty_fields, ClassMapping::fields) {
        let bi = bc.and_then(|c| c.field_position(af.primary(), af.desc()));
        if let Some(i) = bi {
            b_field_used[i] = true;
        }
        let bf = bi.map(|i| &b_fields[i]);
        out.add_field(ci, join_field(Some(af), bf, join))?;
    }
    for (i, bf) in b_fields.iter().enumerate() {
        if !b_field_used[i] {
            out.add_field(ci, join_field(None, Some(bf), join))?;
        }
    }

    let mut b_method_used = vec![false; b_methods.len()];
    for am in ac.map_or(empty_methods, ClassMapping::methods) {
        let bi = bc.and_then(|c| c.method_position(am.primary(), am.desc()));
        if let Some(i) = bi {
            b_method_used[i] = true;
        }
        let bm = bi.map(|i| &b_methods[i]);
        out.add_method(ci, join_method(Some(am), bm, join))?;
    }
    for (i, bm) in b_methods.iter().enumerate() {
        if !b_method_used[i] {
            out.add_method(ci, join_method(None, Some(bm), join))?;
        }
    }
    Ok(())
}

fn join_field(
    af: Option<&FieldMapping>,
    bf: Option<&FieldMapping>,
    join: &Join<'_>,
) -> FieldMapping {
    let key = af.or(bf).map(FieldMapping::primary).unwrap_or_default();
    let desc = af
        .and_then(FieldMapping::desc)
        .or_else(|| bf.and_then(FieldMapping::desc))
        .map(str::to_owned);
    let mut field = FieldMapping::new(
        join(
            af.map(FieldMapping::names),
            bf.map(FieldMapping::names),
            key,
        ),
        desc,
    );
    let comment = af
        .and_then(FieldMapping::comment)
        .or_else(|| bf.and_then(FieldMapping::comment));
    field.set_comment(comment.map(str::to_owned));
    field
}

fn join_method(
    am: Option<&MethodMapping>,
    bm: Option<&MethodMapping>,
    join: &Join<'_>,
) -> MethodMapping {
    let first = am.or(bm);
    let key = first.map(MethodMapping::primary).unwrap_or_default();
    let desc = first.map(MethodMapping::desc).unwrap_or_default();
    let mut method = MethodMapping::new(
        join(
            am.map(MethodMapping::names),
            bm.map(MethodMapping::names),
            key,
        ),
        desc,
    )
    .with_static(
        am.is_some_and(MethodMapping::is_static) || bm.is_some_and(MethodMapping::is_static),
    );
    let comment = am
        .and_then(MethodMapping::comment)
        .or_else(|| bm.and_then(MethodMapping::comment));
    method.set_comment(comment.map(str::to_owned));
    for side in [am, bm].into_iter().flatten() {
        for line in side.lines() {
            method = method.with_lines(*line);
        }
    }

    // Parameters join by local-variable index, variables by slot. Their primary names may be
    // missing, so the shared column comes from whichever side has it.
    let a_params = am.map_or(&[][..], MethodMapping::params);
    let b_params = bm.map_or(&[][..], MethodMapping::params);
    let mut b_param_used = vec![false; b_params.len()];
    for ap in a_params {
        let bi = b_params
            .iter()
            .position(|bp| bp.lv_index() == ap.lv_index());
        if let Some(i) = bi {
            b_param_used[i] = true;
        }
        let bp = bi.map(|i| &b_params[i]);
        method = method.with_param(join_param(Some(ap), bp, join));
    }
    for (i, bp) in b_params.iter().enumerate() {
        if !b_param_used[i] {
            method = method.with_param(join_param(None, Some(bp), join));
        }
    }
    let a_vars = am.map_or(&[][..], MethodMapping::vars);
    let b_vars = bm.map_or(&[][..], MethodMapping::vars);
    let mut b_var_used = vec![false; b_vars.len()];
    let same_slot = |x: &VarMapping, y: &VarMapping| {
        (x.lv_index(), x.start_offset(), x.lvt_index())
            == (y.lv_index(), y.start_offset(), y.lvt_index())
    };
    for av in a_vars {
        let bi = b_vars.iter().position(|bv| same_slot(av, bv));
        if let Some(i) = bi {
            b_var_used[i] = true;
        }
        let bv = bi.map(|i| &b_vars[i]);
        method = method.with_var(join_var(Some(av), bv, join));
    }
    for (i, bv) in b_vars.iter().enumerate() {
        if !b_var_used[i] {
            method = method.with_var(join_var(None, Some(bv), join));
        }
    }
    method
}

fn shared_name<'n>(
    a: Option<&'n [Option<String>]>,
    b: Option<&'n [Option<String>]>,
) -> Option<&'n str> {
    a.and_then(|n| n[0].as_deref())
        .or_else(|| b.and_then(|n| n[0].as_deref()))
}

fn join_param(
    ap: Option<&ParamMapping>,
    bp: Option<&ParamMapping>,
    join: &Join<'_>,
) -> ParamMapping {
    let (an, bn) = (ap.map(ParamMapping::names), bp.map(ParamMapping::names));
    let mut names = join(an, bn, shared_name(an, bn).unwrap_or_default());
    if names[0].as_deref() == Some("") {
        names[0] = None;
    }
    let lv = ap.or(bp).map_or(0, ParamMapping::lv_index);
    let mut param = ParamMapping::new(lv, names);
    let comment = ap
        .and_then(ParamMapping::comment)
        .or_else(|| bp.and_then(ParamMapping::comment));
    param.set_comment(comment.map(str::to_owned));
    param
}

fn join_var(av: Option<&VarMapping>, bv: Option<&VarMapping>, join: &Join<'_>) -> VarMapping {
    let (an, bn) = (av.map(VarMapping::names), bv.map(VarMapping::names));
    let mut names = join(an, bn, shared_name(an, bn).unwrap_or_default());
    if names[0].as_deref() == Some("") {
        names[0] = None;
    }
    let first = av.or(bv);
    let mut var = VarMapping::new(
        first.map_or(0, VarMapping::lv_index),
        first.map_or(0, VarMapping::start_offset),
        first.and_then(VarMapping::lvt_index),
        names,
    );
    let comment = av
        .and_then(VarMapping::comment)
        .or_else(|| bv.and_then(VarMapping::comment));
    var.set_comment(comment.map(str::to_owned));
    var
}

fn primary_on<'s>(set: &'s MappingSet, ns: &str) -> Result<Cow<'s, MappingSet>> {
    if set.primary_namespace() == ns {
        Ok(Cow::Borrowed(set))
    } else {
        Ok(Cow::Owned(set.with_primary(ns)?))
    }
}

/// Builds the two-namespace set `source -> target` from whichever of `sets` connect them.
///
/// The sets form a graph whose nodes are namespaces; each set links all of its namespaces.
/// `chain` takes the shortest path from `source` to `target` (ties go to the set listed first),
/// composes the sets along it through the namespaces they share, and projects the result to
/// `(source, target)` with `source` primary. Sets off the path are not used.
///
/// The Fabric to NeoForge chain on 1.21.1 and 1.21.11 is
/// `chain("intermediary", "mojang", &[&intermediary, &mojang])`, where `intermediary` is
/// `(obf, intermediary)` from intermediary's tiny v2 file and `mojang` is `(mojang, obf)` from
/// Mojang's ProGuard file.
pub fn chain(source: &str, target: &str, sets: &[&MappingSet]) -> Result<MappingSet> {
    if source == target {
        return Err(Error::DuplicateNamespace(source.to_owned()));
    }
    let no_path = || Error::NoPath {
        source: source.to_owned(),
        target: target.to_owned(),
    };
    // Breadth-first search over namespaces. `reached[ns] = (set, previous namespace)`.
    let mut reached: HashMap<&str, Option<(usize, &str)>> = HashMap::new();
    let mut queue = VecDeque::new();
    reached.insert(source, None);
    queue.push_back(source);
    while let Some(ns) = queue.pop_front() {
        if ns == target {
            break;
        }
        for (si, set) in sets.iter().enumerate() {
            if set.namespace_index(ns).is_none() {
                continue;
            }
            for next in set.namespaces() {
                if !reached.contains_key(next.as_str()) {
                    reached.insert(next, Some((si, ns)));
                    queue.push_back(next);
                }
            }
        }
    }
    if !reached.contains_key(target) {
        return Err(no_path());
    }
    // Walk back: steps are (set, enter namespace, exit namespace), source first.
    let mut steps = Vec::new();
    let mut at = target;
    while let Some(Some((si, prev))) = reached.get(at).copied() {
        steps.push((si, prev, at));
        at = prev;
    }
    steps.reverse();
    let Some(&(first_set, _, first_exit)) = steps.first() else {
        return Err(no_path());
    };
    if steps.len() == 1 {
        return sets[first_set].project(&[source, target]);
    }
    let mut acc = sets[first_set].project(&[first_exit, source])?;
    for (k, &(si, enter, exit)) in steps.iter().enumerate().skip(1) {
        let next = sets[si].project(&[enter, exit])?;
        acc = compose(&acc, &next, enter)?;
        if k + 1 < steps.len() {
            acc = acc.project(&[exit, source])?;
        }
    }
    acc.project(&[source, target])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::names;

    fn intermediary() -> MappingSet {
        let mut set = MappingSet::new(["obf", "intermediary"]).unwrap();
        let a = set
            .add_class(names(["a", "net/minecraft/class_1"]))
            .unwrap();
        set.add_class(names(["b", "net/minecraft/class_2"]))
            .unwrap();
        set.add_class(names(["c", "net/minecraft/class_3"]))
            .unwrap();
        set.add_field(
            a,
            FieldMapping::new(names(["a", "field_1"]), Some("Lb;".into())),
        )
        .unwrap();
        set.add_method(a, MethodMapping::new(names(["a", "method_1"]), "(Lb;)La;"))
            .unwrap();
        set
    }

    fn mojang() -> MappingSet {
        let mut set = MappingSet::new(["mojang", "obf"]).unwrap();
        let alpha = set.add_class(names(["net/minecraft/Alpha", "a"])).unwrap();
        set.add_class(names(["net/minecraft/Beta", "b"])).unwrap();
        set.add_class(names(["net/minecraft/OnlyMojang", "z"]))
            .unwrap();
        set.add_field(
            alpha,
            FieldMapping::new(names(["beta", "a"]), Some("Lnet/minecraft/Beta;".into())),
        )
        .unwrap();
        set.add_method(
            alpha,
            MethodMapping::new(
                names(["make", "a"]),
                "(Lnet/minecraft/Beta;)Lnet/minecraft/Alpha;",
            ),
        )
        .unwrap();
        set.add_method(
            alpha,
            MethodMapping::new(names(["<init>", "<init>"]), "()V"),
        )
        .unwrap();
        set
    }

    #[test]
    fn compose_joins_on_the_shared_namespace() {
        let set = compose(&intermediary(), &mojang(), "obf").unwrap();
        assert_eq!(set.namespaces(), ["obf", "intermediary", "mojang"]);
        let a = set.class("a").unwrap();
        assert_eq!(a.name(1), Some("net/minecraft/class_1"));
        assert_eq!(a.name(2), Some("net/minecraft/Alpha"));
        assert_eq!(set.class("c").unwrap().name(2), None);
        assert_eq!(set.class("z").unwrap().name(1), None);
        assert_eq!(
            set.method("a", "a", "(Lb;)La;").unwrap().name(2),
            Some("make")
        );
        assert_eq!(set.method("a", "<init>", "()V").unwrap().name(1), None);
        assert!(matches!(
            compose(&intermediary(), &intermediary(), "obf"),
            Err(Error::DuplicateNamespace(_))
        ));
        assert!(compose(&intermediary(), &mojang(), "named").is_err());
    }

    #[test]
    fn chain_intermediary_to_mojang() {
        let i = intermediary();
        let m = mojang();
        let set = chain("intermediary", "mojang", &[&i, &m]).unwrap();
        assert_eq!(set.namespaces(), ["intermediary", "mojang"]);
        let mapper = set.mapper("intermediary", "mojang").unwrap();
        assert_eq!(
            mapper.map_class("net/minecraft/class_1"),
            Some("net/minecraft/Alpha")
        );
        assert_eq!(
            mapper.map_method(
                "net/minecraft/class_1",
                "method_1",
                "(Lnet/minecraft/class_2;)Lnet/minecraft/class_1;"
            ),
            Some("make")
        );
        assert_eq!(
            mapper.map_field(
                "net/minecraft/class_1",
                "field_1",
                Some("Lnet/minecraft/class_2;")
            ),
            Some("beta")
        );
        assert_eq!(mapper.map_class("net/minecraft/class_3"), None);
        // The reverse chain uses the same sets.
        let back = chain("mojang", "intermediary", &[&m, &i]).unwrap();
        assert_eq!(
            back.mapper("mojang", "intermediary")
                .unwrap()
                .map_class("net/minecraft/Beta"),
            Some("net/minecraft/class_2")
        );
        // Within one set, chain is a projection.
        let direct = chain("obf", "intermediary", &[&i, &m]).unwrap();
        assert_eq!(direct, i);
        assert!(matches!(
            chain("intermediary", "srg", &[&i, &m]),
            Err(Error::NoPath { .. })
        ));
        assert!(chain("obf", "obf", &[&i]).is_err());
    }
}
