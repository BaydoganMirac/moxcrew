use patchfleet_core::retrieval;
use serde_json::{json, Value};

const PROJECT: &str = "project:10000000-0000-4000-8000-000000000001";
const OTHER: &str = "project:10000000-0000-4000-8000-000000000002";
const SHARED: &str = "shared:10000000-0000-4000-8000-000000000003";
fn row(n: u64, status: &str, title: &str, text: &str) -> Value {
    json!({"projectId":PROJECT,"decisionId":format!("decision:10000000-0000-4000-8000-{n:012}"),"revision":2,"status":status,"title":title,"decision":text,"context":"","rationale":"","consequences":""})
}
fn response(semantic: bool) -> Value {
    json!({"schemaVersion":1,"sharedProjectId":SHARED,"manifestRevision":2,"mode":if semantic {"semantic"} else {"keyword"},"model":if semantic {json!({"id":retrieval::MODEL_ID,"revision":retrieval::MODEL_REVISION,"dimensions":384,"normalization":"l2","dtype":"q8","pooling":"mean","chunkVersion":1})} else {Value::Null},"index":{"state":"ready","indexed":2,"total":2},"matches":[{"decisionId":row(1,"accepted","","")["decisionId"],"revision":2,"title":"Rust kararı","score":0.9},{"decisionId":row(2,"accepted","","")["decisionId"],"revision":3,"title":"SQLite","score":0.3}]})
}

#[test]
fn keyword_ranks_current_accepted_text_only_in_the_selected_project() {
    let mut foreign = row(6, "accepted", "Rust Rust Rust", "Rust");
    foreign["projectId"] = json!(OTHER);
    let projection = json!({"projects":[{"projectId":PROJECT,"revision":17},{"projectId":OTHER,"revision":3}],"decisions":[row(1,"accepted","Rust","Low memory"),row(2,"accepted","Other","Rust process"),row(3,"proposed","Rust","Rust"),row(4,"rejected","Rust","Rust"),row(5,"superseded","Rust","Rust"),foreign]});
    let page = retrieval::search_local(
        &projection,
        &json!({"projectId":PROJECT,"query":"  Rust  "}),
    )
    .unwrap();
    assert_eq!(page["revision"], 17);
    assert_eq!(page["query"], "Rust");
    assert_eq!(page["mode"], "keyword");
    assert_eq!(page["limit"], 5);
    assert_eq!(page["matches"].as_array().unwrap().len(), 2);
    assert_eq!(
        page["matches"][0]["decisionId"],
        row(1, "accepted", "", "")["decisionId"]
    );
    assert!(page["matches"][0].get("decision").is_none());
    assert!(retrieval::search_local(
        &projection,
        &json!({"projectId":"project:10000000-0000-4000-8000-000000000099","query":"Rust"})
    )
    .is_err());
}

#[test]
fn keyword_limits_ties_full_text_and_unicode_case_are_deterministic() {
    let projection = json!({"projects":[{"projectId":PROJECT,"revision":1}],"decisions":[row(2,"accepted","İŞLEM","Rust"),row(1,"accepted","işlem","Rust"),row(3,"accepted","Large",&format!("{} latechunk", "unrelated ".repeat(1500)))]});
    let page = retrieval::search_local(
        &projection,
        &json!({"projectId":PROJECT,"query":"IŞLEM","limit":1}),
    )
    .unwrap();
    assert_eq!(page["matches"].as_array().unwrap().len(), 1);
    assert_eq!(
        page["matches"][0]["decisionId"],
        row(1, "accepted", "", "")["decisionId"]
    );
    let late = retrieval::search_local(
        &projection,
        &json!({"projectId":PROJECT,"query":"latechunk"}),
    )
    .unwrap();
    assert_eq!(
        late["matches"][0]["decisionId"],
        row(3, "accepted", "", "")["decisionId"]
    );
    let punctuation =
        retrieval::search_local(&projection, &json!({"projectId":PROJECT,"query":"?!"})).unwrap();
    assert_eq!(punctuation["matches"], json!([]));
}

#[test]
fn query_and_limit_validation_follow_utf16_and_printable_read_bounds() {
    for query in [
        "",
        "  \t\n",
        "bad\u{0}",
        "bad\r",
        "\u{85}",
        &"🦀".repeat(201),
    ] {
        assert!(retrieval::request(&json!({"projectId":PROJECT,"query":query})).is_err());
    }
    for query in [
        "🦀".repeat(200),
        "rust\tkarar\nneden".into(),
        "a".repeat(400),
    ] {
        assert!(retrieval::request(&json!({"projectId":PROJECT,"query":query})).is_ok());
    }
    for limit in [
        json!(0),
        json!(11),
        json!(1.5),
        json!(null),
        json!("5"),
        json!(true),
    ] {
        assert!(
            retrieval::request(&json!({"projectId":PROJECT,"query":"Rust","limit":limit})).is_err()
        );
    }
    for payload in [
        json!({"projectId":PROJECT}),
        json!({"projectId":PROJECT,"query":"Rust","vector":[]}),
        json!({"projectId":"project:NOT-UUID","query":"Rust"}),
        json!({"projectId":PROJECT,"query":false}),
    ] {
        assert!(retrieval::request(&payload).is_err());
    }
}

#[test]
fn semantic_response_requires_exact_model_space_and_complete_index() {
    let good = response(true);
    retrieval::validate_search_response(&good, &json!(SHARED), 5).unwrap();
    for (path, value) in [
        ("/model/id", json!("other")),
        ("/model/revision", json!("main")),
        ("/model/dimensions", json!(768)),
        ("/model/dtype", json!("fp32")),
        ("/model/pooling", json!("cls")),
        ("/model/normalization", json!("none")),
        ("/model/chunkVersion", json!(2)),
        ("/index/state", json!("pending")),
        ("/index/indexed", json!(1)),
        ("/matches/0/score", json!(1.01)),
        ("/matches/1/score", json!(-1.01)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            retrieval::validate_search_response(&bad, &json!(SHARED), 5).is_err(),
            "{path}"
        );
    }
    let mut bad = good.clone();
    bad["model"]["extra"] = json!(true);
    assert!(retrieval::validate_search_response(&bad, &json!(SHARED), 5).is_err());
}

#[test]
fn search_response_rejects_scope_unknown_fields_order_duplicates_and_bounds() {
    let good = response(false);
    retrieval::validate_search_response(&good, &json!(SHARED), 5).unwrap();
    for (path, value) in [
        ("/schemaVersion", json!(2)),
        ("/manifestRevision", json!(9_007_199_254_740_992u64)),
        ("/mode", json!("vector")),
        ("/model", json!({})),
        ("/index/total", json!(65)),
        ("/index/indexed", json!(3)),
        ("/index/state", json!("unknown")),
        ("/matches/0/revision", json!(0)),
        ("/matches/0/title", json!(" ")),
        ("/matches/0/decisionId", json!("decision:invalid")),
        ("/matches/0/score", json!(-0.01)),
        ("/matches/1/score", json!(1.0)),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            retrieval::validate_search_response(&bad, &json!(SHARED), 5).is_err(),
            "{path}"
        );
    }
    for path in ["", "/index", "/matches/0"] {
        let mut bad = good.clone();
        bad.pointer_mut(path).unwrap()["extra"] = json!("no source text");
        assert!(retrieval::validate_search_response(&bad, &json!(SHARED), 5).is_err());
    }
    let mut duplicate = good.clone();
    duplicate["matches"][1]["decisionId"] = duplicate["matches"][0]["decisionId"].clone();
    assert!(retrieval::validate_search_response(&duplicate, &json!(SHARED), 5).is_err());
    assert!(retrieval::validate_search_response(&good, &json!(SHARED), 1).is_err());
    assert!(retrieval::validate_search_response(&good, &json!(OTHER), 5).is_err());
    let mut ties = good.clone();
    ties["matches"][1]["score"] = ties["matches"][0]["score"].clone();
    retrieval::validate_search_response(&ties, &json!(SHARED), 5).unwrap();
    ties["matches"].as_array_mut().unwrap().reverse();
    assert!(retrieval::validate_search_response(&ties, &json!(SHARED), 5).is_err());
}
