//! The coach verb battery — the 2026-08-01 ad-hoc curl matrix
//! (`docs/proof/coach-verb-battery/REPORT.md`), committed as a drill so
//! it can never rot in a chat transcript again (S2 plan §4.4). Drives a
//! RUNNING daemon over localhost HTTP through all 21 cataloged verbs
//! plus the SSE observe stream, asserting the S2 `EffectContract`
//! shape on every `/verb` answer. Emits a dated markdown report on
//! stdout (redirect beside the old report); exits non-zero on any FAIL.
//!
//! ```sh
//! cargo run -p dpl-coach-daemon &            # the daemon under test
//! cargo run -p dpl-coach-daemon --example verb_battery \
//!   > docs/proof/coach-verb-battery/REPORT-<date>.md
//! ```
//!
//! Env: `DPL_COACH_URL` (default `http://127.0.0.1:8777`) and
//! `DPL_BATTERY_APP` (default `gnome-calculator` — the same seat app
//! the 2026-08-01 battery proved).

use std::time::Duration;

use serde_json::{json, Value};

struct Battery {
    agent: ureq::Agent,
    base: String,
    app: String,
    pass: u32,
    soft: u32,
    fail: u32,
}

/// One step's verdict + one-line detail + the raw wire body.
enum Verdict {
    Pass(String),
    Soft(String),
    Fail(String),
}

impl Battery {
    fn get(&self, path: &str) -> (u16, Value) {
        let url = format!("{}{path}", self.base);
        match self.agent.get(&url).call() {
            Ok(mut res) => {
                let code = res.status().as_u16();
                let body = res.body_mut().read_to_string().unwrap_or_default();
                (code, serde_json::from_str(&body).unwrap_or(Value::Null))
            }
            Err(e) => (0, json!({ "transport_error": e.to_string() })),
        }
    }

    fn post(&self, path: &str, body: Value) -> (u16, Value) {
        let url = format!("{}{path}", self.base);
        let sent = self
            .agent
            .post(&url)
            .header("Content-Type", "application/json")
            .send(body.to_string());
        match sent {
            Ok(mut res) => {
                let code = res.status().as_u16();
                let body = res.body_mut().read_to_string().unwrap_or_default();
                (code, serde_json::from_str(&body).unwrap_or(Value::Null))
            }
            Err(e) => (0, json!({ "transport_error": e.to_string() })),
        }
    }

    fn verb(&self, verb: &str, extra: Value) -> (u16, Value) {
        let mut body = json!({ "verb": verb, "app": self.app });
        if let (Some(map), Some(add)) = (body.as_object_mut(), extra.as_object()) {
            for (k, v) in add {
                map.insert(k.clone(), v.clone());
            }
        }
        self.post("/verb", body)
    }

    fn step(&mut self, name: &str, body: &Value, verdict: Verdict) {
        let (word, detail) = match &verdict {
            Verdict::Pass(d) => {
                self.pass += 1;
                ("PASS", d.clone())
            }
            Verdict::Soft(d) => {
                self.soft += 1;
                ("SOFT", d.clone())
            }
            Verdict::Fail(d) => {
                self.fail += 1;
                ("FAIL", d.clone())
            }
        };
        println!("### {name} — {word}\n\n{detail}\n\n```\n{body}\n```\n");
    }
}

/// PASS when the answer is an S2 `EffectContract` for `verb` with
/// `ok: true` — the shape assertion every mutating step shares.
fn contract(body: &Value, verb: &str) -> Result<(), String> {
    for key in ["ok", "verb", "app", "rung", "effect_observed"] {
        if body.get(key).is_none() {
            return Err(format!("missing EffectContract field `{key}`"));
        }
    }
    if body["verb"] != verb {
        return Err(format!("verb echo: {} != {verb}", body["verb"]));
    }
    if body["ok"] != Value::Bool(true) {
        return Err("ok=false".into());
    }
    Ok(())
}

fn effect(body: &Value, verb: &str) -> Verdict {
    match contract(body, verb) {
        Ok(()) if body["effect_observed"] == Value::Bool(true) => {
            Verdict::Pass(format!("{verb} effect_observed on rung {}", body["rung"]))
        }
        Ok(()) => Verdict::Fail(format!("{verb} ok but effect_observed=false")),
        Err(e) => Verdict::Fail(e),
    }
}

fn main() {
    let base =
        std::env::var("DPL_COACH_URL").unwrap_or_else(|_| "http://127.0.0.1:8777".to_string());
    let app = std::env::var("DPL_BATTERY_APP").unwrap_or_else(|_| "gnome-calculator".to_string());
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();
    let mut b = Battery {
        agent,
        base: base.clone(),
        app: app.clone(),
        pass: 0,
        soft: 0,
        fail: 0,
    };

    println!("# Coach verb battery — committed drill (S2)\n");
    println!("URL={base} APP={app}\n");

    // -- health + introspection ------------------------------------
    let (_, body) = b.get("/health");
    let v = if body["ok"] == Value::Bool(true) {
        Verdict::Pass("daemon ok".into())
    } else {
        Verdict::Fail("daemon not answering /health".into())
    };
    b.step("health", &body, v);
    if b.fail > 0 {
        println!("\n**BATTERY: ABORT — no daemon at {base}**");
        std::process::exit(1);
    }

    let (_, body) = b.get("/verbs");
    let names: Vec<&str> = body["verbs"]
        .as_array()
        .map(|a| a.iter().filter_map(|v| v["name"].as_str()).collect())
        .unwrap_or_default();
    let v = if names.len() == 21 && names.contains(&"geom") && names.contains(&"drag") {
        Verdict::Pass(format!(
            "catalog {} entries from the protocol (geom cataloged — S2 correction)",
            names.len()
        ))
    } else {
        Verdict::Fail(format!("catalog {names:?} — expected 21 with geom + drag"))
    };
    b.step("verbs", &body, v);

    let (_, body) = b.get("/inventory");
    let apps = body["apps"].as_array().map(Vec::len).unwrap_or(0);
    let v = if apps > 0 {
        Verdict::Pass(format!("apps={apps}"))
    } else {
        Verdict::Fail("empty inventory".into())
    };
    b.step("inventory", &body, v);

    // -- lifecycle up ------------------------------------------------
    let _ = b.verb("close", json!({})); // start from a known-unreal seat
    std::thread::sleep(Duration::from_millis(800));

    let (_, body) = b.get(&format!("/presence?app={app}"));
    let v = if body["real"] == Value::Bool(false) {
        Verdict::Pass("real=false before open".into())
    } else {
        Verdict::Fail("app still real after close".into())
    };
    b.step("presence_unreal", &body, v);

    let (_, body) = b.verb("assert_state", json!({ "expect": "unreal" }));
    b.step("assert_unreal", &body, effect(&body, "assert_state"));

    let (_, body) = b.verb("open", json!({}));
    let v = match contract(&body, "open") {
        Ok(())
            if body["effect_observed"] == Value::Bool(true)
                && body["after"]["running"] == Value::Bool(true) =>
        {
            Verdict::Pass("effect_observed + after.running=true".into())
        }
        Ok(()) => Verdict::Fail("open ok but no observed running=true".into()),
        Err(e) => Verdict::Fail(e),
    };
    b.step("open", &body, v);

    let (_, body) = b.get(&format!("/presence?app={app}"));
    let v = if body["real"] == Value::Bool(true) {
        Verdict::Pass(format!("real, pid={}", body["pid"]))
    } else {
        Verdict::Fail("not real after open".into())
    };
    b.step("presence_real", &body, v);

    let (_, body) = b.verb(
        "wait_until",
        json!({ "expect": "real", "timeout_ms": 5000 }),
    );
    b.step("wait_until", &body, effect(&body, "wait_until"));

    let (_, body) = b.verb("assert_state", json!({ "expect": "real" }));
    b.step("assert_real", &body, effect(&body, "assert_state"));

    // -- window verbs -------------------------------------------------
    let (_, body) = b.verb("geom", json!({}));
    b.step("geom", &body, effect(&body, "geom"));

    let (_, body) = b.verb("move", json!({ "x": 280, "y": 220 }));
    let v = match contract(&body, "move") {
        Ok(()) if body["after"]["rect"]["x"] == json!(280) => {
            Verdict::Pass("after.rect.x=280 typed in the contract".into())
        }
        Ok(()) => Verdict::Fail(format!("after rect {} != x=280", body["after"]["rect"])),
        Err(e) => Verdict::Fail(e),
    };
    b.step("move", &body, v);

    let (_, body) = b.verb("resize", json!({ "w": 560, "h": 700 }));
    let v = match contract(&body, "resize") {
        Ok(()) if body["after"]["rect"]["w"] == json!(560) => {
            Verdict::Pass("after.rect.w=560".into())
        }
        Ok(()) => Verdict::Fail(format!("after rect {} != w=560", body["after"]["rect"])),
        Err(e) => Verdict::Fail(e),
    };
    b.step("resize", &body, v);

    let (_, body) = b.verb("focus", json!({}));
    b.step("focus", &body, effect(&body, "focus"));

    // -- observe: SSE stream with a concurrent mover ------------------
    let mover = {
        let base = base.clone();
        let app = app.clone();
        std::thread::spawn(move || {
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .timeout_global(Some(Duration::from_secs(10)))
                .http_status_as_error(false)
                .build()
                .into();
            std::thread::sleep(Duration::from_millis(1200));
            let _ = agent
                .post(format!("{base}/verb"))
                .header("Content-Type", "application/json")
                .send(json!({ "verb": "move", "app": app, "x": 300, "y": 240 }).to_string());
            std::thread::sleep(Duration::from_millis(1200));
            let _ = agent
                .post(format!("{base}/verb"))
                .header("Content-Type", "application/json")
                .send(json!({ "verb": "resize", "app": app, "w": 580, "h": 720 }).to_string());
        })
    };
    let sse = match b
        .agent
        .get(format!("{base}/events?app={app}&seconds=5"))
        .call()
    {
        Ok(mut res) => res.body_mut().read_to_string().unwrap_or_default(),
        Err(e) => format!("transport_error: {e}"),
    };
    let _ = mover.join();
    let ok = sse.contains("\"kind\":\"observe_start\"")
        && sse.contains("\"kind\":\"moved\"")
        && sse.contains("\"kind\":\"resized\"")
        && sse.contains("\"kind\":\"observe_end\"");
    let v = if ok {
        Verdict::Pass(
            "typed SeamEvent stream: observe_start → moved → resized → observe_end".into(),
        )
    } else {
        Verdict::Fail("SSE stream missing typed moved/resized facts".into())
    };
    b.step("observe", &json!({ "sse": sse }), v);

    // -- text/key/select/scroll ---------------------------------------
    let (_, body) = b.verb("type_text", json!({ "text": "3" }));
    b.step("type_text", &body, effect(&body, "type_text"));

    let (_, body) = b.verb("press_key", json!({ "key": "5" }));
    b.step("press_key", &body, effect(&body, "press_key"));

    let (_, body) = b.verb("select", json!({ "name": "Basic" }));
    b.step("select", &body, effect(&body, "select"));

    let (_, body) = b.verb("scroll", json!({ "direction": "down", "delta": 2 }));
    let v = match contract(&body, "scroll") {
        Ok(()) if body["effect_observed"] == Value::Bool(true) => {
            Verdict::Pass("scroll effect_observed".into())
        }
        // The 2026-08-01 battery graded this SOFT: delivery ok, but
        // Calculator exposes no scroll offset to observe.
        Ok(()) => Verdict::Soft(format!(
            "delivered via {} but effect_observed=false (known Calculator limit)",
            body["extra"]["via"]
        )),
        Err(e) => Verdict::Fail(e),
    };
    b.step("scroll", &body, v);

    // -- fabric verbs (routes untouched by S2 — old shapes) ------------
    // The battery IS the calculator script, so the seed is explicit —
    // the daemon stopped sniffing seeds from app names in S3b.
    let (_, body) = b.post("/import", json!({ "name": app, "seed": "7" }));
    let session = body["session"].as_str().unwrap_or_default().to_string();
    let seed = body["seed_path"].as_str().unwrap_or_default().to_string();
    let v = if body["ok"] == Value::Bool(true) && !session.is_empty() {
        Verdict::Pass(format!("session={session} nodes={}", body["nodes"]))
    } else {
        Verdict::Fail("import refused".into())
    };
    b.step("import", &body, v);

    let (_, body) = b.post("/resolve", json!({ "session": session, "path": seed }));
    let name = body["name"].as_str().unwrap_or_default().to_string();
    let v = if body["ok"] == Value::Bool(true) {
        Verdict::Pass(format!("seed name={name}"))
    } else {
        Verdict::Fail("resolve refused".into())
    };
    b.step("resolve", &body, v);

    let (_, body) = b.post(
        "/invoke",
        json!({ "session": session, "path": seed, "action": "Click" }),
    );
    let v = if body["ok"] == Value::Bool(true) && body["effect_observed"] == Value::Bool(true) {
        Verdict::Pass(format!(
            "click effect_observed display {}→{}",
            body["display_before"], body["display_after"]
        ))
    } else {
        Verdict::Fail("invoke click had no observed effect".into())
    };
    b.step("invoke_click", &body, v);

    let (_, body) = b.post(
        "/mount",
        json!({ "session": session, "path": seed, "spawn_overlay": false }),
    );
    let v = if body["ok"] == Value::Bool(true) {
        Verdict::Pass(format!(
            "window_rect={} (no overlay spawn)",
            body["window_rect"]
        ))
    } else {
        Verdict::Fail("mount refused".into())
    };
    b.step("mount", &body, v);

    // -- refusals are typed too ----------------------------------------
    let (code, body) = b.verb("drag", json!({}));
    let v = if code == 409 && body["refused"] == Value::Bool(true) {
        Verdict::Pass("planned verb correctly refused (409)".into())
    } else {
        Verdict::Fail(format!("drag answered {code}, wanted 409 refusal"))
    };
    b.step("drag_planned", &body, v);

    let (code, body) = b.verb("teleport", json!({}));
    let v = if code == 409 && body["refused"] == Value::Bool(true) {
        Verdict::Pass("unknown verb refused at the typed edge (409)".into())
    } else {
        Verdict::Fail(format!("teleport answered {code}, wanted 409 refusal"))
    };
    b.step("unknown_verb", &body, v);

    // -- lifecycle down --------------------------------------------------
    let (_, body) = b.verb("close", json!({}));
    let v = match contract(&body, "close") {
        Ok(())
            if body["effect_observed"] == Value::Bool(true)
                && body["after"]["running"] == Value::Bool(false) =>
        {
            Verdict::Pass("real→unreal observed".into())
        }
        Ok(()) => Verdict::Fail("close ok but app still observed running".into()),
        Err(e) => Verdict::Fail(e),
    };
    b.step("close", &body, v);

    let (_, body) = b.verb("assert_state", json!({ "expect": "unreal" }));
    b.step("assert_unreal_final", &body, effect(&body, "assert_state"));

    println!("## Summary\n");
    println!("- PASS: {}", b.pass);
    println!("- SOFT: {}", b.soft);
    println!("- FAIL: {}", b.fail);
    println!(
        "\n**BATTERY: {}**",
        if b.fail == 0 { "GREEN" } else { "RED" }
    );
    if b.fail > 0 {
        std::process::exit(1);
    }
}
