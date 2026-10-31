//! The capability-hook renderer: the per-capability part of gsd-core's `loop-resolver.cts` (`resolveLoopHooks`), so a
//! capability extension answers `<cap>_hooks <point>` as `gsd-tools loop render-hooks <point>` answered for its
//! capability (ADR gsd/0002, plan 16 D-S-3).
//!
//! The capability's own `capability.json` (`steps`, `contributions`, `gates`) is the input. A hook is active when its
//! `when` (a dotted settings key) is truthy and its `pointFrom`, if any, names the point. An inactive hook is absent,
//! as upstream's: `skippedHooks` is only ever what `--after-fingerprint` gates out, which has no counterpart here
//! (the extension fills it itself: see ADR gsd/0002), so this renderer's is empty.

use serde_json::{Map, Value, json};

/// The twelve points of the loop host contract (`loop-host-contract.cjs`).
pub const POINTS: &[&str] = &[
    "discuss:pre",
    "discuss:post",
    "plan:pre",
    "plan:post",
    "execute:pre",
    "execute:wave:pre",
    "execute:wave:post",
    "execute:post",
    "verify:pre",
    "verify:post",
    "ship:pre",
    "ship:post",
];

/// What `render` answers: `{activeHooks, skippedHooks}`.
pub fn render(
    capability: &Value,
    setting: &dyn Fn(&str) -> Option<Value>,
    point: &str,
) -> Result<Value, String> {
    if !POINTS.contains(&point) {
        return Err(format!(
            "Invalid loop point: \"{point}\". Valid points: {}",
            POINTS.join(", ")
        ));
    }
    let cap_id = capability["id"].as_str().unwrap_or_default();
    // the value a dotted key resolves to: the setting, else the capability's declared default
    let resolve =
        |key: &str| setting(key).or_else(|| capability["config"][key].get("default").cloned());
    let truthy = |key: &str| match resolve(key) {
        None | Some(Value::Null | Value::Bool(false)) => false,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    };
    let active = |hook: &Value| {
        if let Some(when) = hook.get("when").filter(|w| !w.is_null())
            && !when
                .as_str()
                .is_some_and(|key| !key.is_empty() && truthy(key))
        {
            return false;
        }
        match hook.get("pointFrom").filter(|p| !p.is_null()) {
            None => true,
            Some(key) => key
                .as_str()
                .filter(|k| !k.is_empty())
                .and_then(resolve)
                .is_some_and(|v| v == point),
        }
    };
    let mut hooks = Vec::new();
    for (list, kind) in [
        ("steps", "step"),
        ("contributions", "contribution"),
        ("gates", "gate"),
    ] {
        let found = capability[list].as_array().into_iter().flatten();
        for hook in found.filter(|h| h["point"] == point && active(h)) {
            hooks.push(hook_of(cap_id, kind, hook, &resolve));
        }
    }
    Ok(json!({ "activeHooks": hooks, "skippedHooks": [] }))
}

/// One active hook with upstream's keys; an empty or absent list is left out.
fn hook_of(
    cap_id: &str,
    kind: &str,
    hook: &Value,
    resolve: &dyn Fn(&str) -> Option<Value>,
) -> Value {
    let mut out = Map::new();
    out.insert("capId".into(), cap_id.into());
    out.insert("kind".into(), kind.into());
    let copy = |out: &mut Map<String, Value>, key: &str, wanted: fn(&Value) -> bool| {
        if let Some(v) = hook.get(key).filter(|v| wanted(v)) {
            out.insert(key.into(), v.clone());
        }
    };
    let object: fn(&Value) -> bool = |v| v.is_object();
    let text: fn(&Value) -> bool = Value::is_string;
    let flag: fn(&Value) -> bool = Value::is_boolean;
    let strings = |out: &mut Map<String, Value>, key: &str| {
        let items: Vec<&Value> = hook[key]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v.is_string())
            .collect();
        if !items.is_empty() {
            out.insert(key.into(), json!(items));
        }
    };
    match kind {
        "step" => {
            copy(&mut out, "ref", object);
        }
        "contribution" => copy(&mut out, "into", text),
        _ => {}
    }
    if kind != "gate" {
        let inline = hook["fragment"]["inline"].as_str();
        let path = hook["fragment"]["path"].as_str();
        let mut fragment = Map::new();
        for (key, value) in [("inline", inline), ("path", path)] {
            if let Some(v) = value {
                fragment.insert(key.into(), v.into());
            }
        }
        if !fragment.is_empty() {
            out.insert("fragment".into(), fragment.into());
        }
    }
    copy(&mut out, "when", text);
    if kind != "gate" {
        strings(&mut out, "produces");
        strings(&mut out, "consumes");
    }
    if kind == "gate" {
        if let Some(check) = hook.get("check") {
            out.insert("check".into(), check.clone());
        }
        copy(&mut out, "blocking", flag);
    }
    copy(&mut out, "onError", text);
    if kind == "step" && hook["supportsReviewerLanes"] == true {
        out.insert("supportsReviewerLanes".into(), true.into());
    }
    if kind == "contribution" {
        let mut values = Map::new();
        for (alias, key) in hook["configValues"].as_object().into_iter().flatten() {
            if matches!(alias.as_str(), "__proto__" | "constructor" | "prototype") {
                continue;
            }
            if let Some(value) = key.as_str().and_then(resolve) {
                values.insert(alias.clone(), value);
            }
        }
        if !values.is_empty() {
            out.insert("configValues".into(), values.into());
        }
    }
    out.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capability() -> Value {
        json!({
            "id": "demo",
            "config": { "demo.on": { "default": true }, "demo.level": { "default": 1 } },
            "steps": [{ "point": "verify:post", "ref": { "skill": "s" }, "produces": ["A.md"], "consumes": [],
                         "when": "demo.on", "onError": "halt" }],
            "contributions": [{ "point": "plan:pre", "into": "planner", "fragment": { "inline": "text" },
                                 "configValues": { "level": "demo.level", "gone": "demo.absent" },
                                 "produces": [], "consumes": ["C.md"], "when": "demo.on" }],
            "gates": [{ "point": "ship:pre", "check": { "predicate": { "kind": "k" } }, "when": "demo.on",
                         "blocking": true, "onError": "halt" }]
        })
    }

    fn none(_: &str) -> Option<Value> {
        None
    }

    fn active(point: &str, setting: &dyn Fn(&str) -> Option<Value>) -> Vec<Value> {
        let out = render(&capability(), setting, point).expect("a point");
        assert_eq!(out["skippedHooks"], json!([]));
        out["activeHooks"].as_array().expect("a list").clone()
    }

    #[test]
    fn each_kind_lands_under_its_point_with_upstreams_keys() {
        assert_eq!(
            active("verify:post", &none),
            [
                json!({ "capId": "demo", "kind": "step", "ref": { "skill": "s" }, "when": "demo.on",
                     "produces": ["A.md"], "onError": "halt" })
            ]
        );
        assert_eq!(
            active("plan:pre", &none),
            [
                json!({ "capId": "demo", "kind": "contribution", "into": "planner", "fragment": { "inline": "text" },
                     "when": "demo.on", "consumes": ["C.md"], "configValues": { "level": 1 } })
            ]
        );
        assert_eq!(
            active("ship:pre", &none),
            [
                json!({ "capId": "demo", "kind": "gate", "when": "demo.on", "check": { "predicate": { "kind": "k" } },
                     "blocking": true, "onError": "halt" })
            ]
        );
    }

    #[test]
    fn a_falsy_when_leaves_the_hook_out_and_another_point_has_none() {
        let off = |key: &str| (key == "demo.on").then_some(json!(false));
        for point in ["verify:post", "plan:pre", "ship:pre"] {
            assert!(active(point, &off).is_empty(), "{point}");
        }
        assert!(active("execute:pre", &none).is_empty());
    }

    #[test]
    fn a_setting_beats_the_default_and_an_invalid_point_is_refused() {
        let two = |key: &str| (key == "demo.level").then(|| json!(2));
        assert_eq!(active("plan:pre", &two)[0]["configValues"]["level"], 2);
        let why = render(&capability(), &none, "nowhere").expect_err("refused");
        assert!(
            why.contains("Invalid loop point") && why.contains("ship:post"),
            "{why}"
        );
    }

    #[test]
    fn point_from_selects_the_registration() {
        let mut cap = capability();
        cap["steps"][0]["pointFrom"] = json!("demo.at");
        cap["config"]["demo.at"] = json!({ "default": "verify:post" });
        assert_eq!(
            render(&cap, &none, "verify:post").expect("ok")["activeHooks"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        let elsewhere = |key: &str| (key == "demo.at").then(|| json!("execute:post"));
        assert_eq!(
            render(&cap, &elsewhere, "verify:post").expect("ok")["activeHooks"],
            json!([])
        );
    }
}
