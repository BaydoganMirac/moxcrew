use super::*;
use base64::Engine;
use std::sync::atomic::Ordering;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::Mutex,
};

fn raster() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAAAHHRFWHRwcm9tcHQAUFJJVkFURV9FTkdJTkVfUFJPTVBUYA6ekwAAAHpJREFUeJztz1EJACAUwMAXxqBGNJIh/DiEwQLcZu3zdcMFDWhBA1rQgBY0oAUNaEEDWtCAFjSgBQ1oQQNa0IAWNKAFDWhBA1rQgBY0oAUNaEEDWtCAFjSgBQ1oQQNa0IAWNKAFDWhBA1rQgBY0oAUNaEEDWtCAFjx2AU8iQVoikf+xAAAAAElFTkSuQmCC").unwrap()
}
fn wav(seconds: u32) -> Vec<u8> {
    let sample_rate = 8_000_u32;
    let data_size = sample_rate * seconds * 2;
    let mut bytes = Vec::with_capacity(44 + data_size as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.resize(44 + data_size as usize, 0);
    bytes
}
struct Peer {
    url: String,
    calls: Arc<Mutex<Vec<(String, Value)>>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
async fn peer(mode: &'static str) -> Peer {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let calls = Arc::new(Mutex::new(vec![]));
    let saved = calls.clone();
    let task = tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut raw = vec![];
            let mut buffer = [0; 8192];
            let split;
            loop {
                let n = stream.read(&mut buffer).await.unwrap();
                if n == 0 {
                    return;
                }
                raw.extend_from_slice(&buffer[..n]);
                if let Some(index) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    split = index + 4;
                    break;
                }
                assert!(raw.len() < 16384);
            }
            let header = String::from_utf8_lossy(&raw[..split]).into_owned();
            let size = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|n| n.trim().parse::<usize>().ok())
                })
                .unwrap_or(0);
            assert!(size < 26 * 1024 * 1024);
            while raw.len() < split + size {
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                raw.extend_from_slice(&buffer[..n]);
            }
            let path = header
                .lines()
                .next()
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_owned();
            let body: Value =
                serde_json::from_slice(&raw[split..split + size]).unwrap_or(Value::Null);
            saved.lock().await.push((path.clone(), body.clone()));
            let mut status = "200 OK";
            let mut kind = "application/json";
            let response = if path.starts_with("/object_info/") {
                serde_json::to_vec(&json!({"CheckpointLoaderSimple":{"input":{"required":{"ckpt_name":[["fixture.safetensors"]]}}}})).unwrap()
            } else if path.starts_with("/api/jobs/") && path.ends_with("/cancel") {
                serde_json::to_vec(&json!({"cancelled":false})).unwrap()
            } else if path == "/upload/image" {
                let part = String::from_utf8_lossy(&raw[split..split + size]);
                let name = part
                    .split("filename=\"")
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap();
                serde_json::to_vec(&json!({"name":name,"subfolder":"","type":"input"})).unwrap()
            } else if path == "/prompt" {
                serde_json::to_vec(&json!({"prompt_id":if mode=="foreign" {json!(uuid::Uuid::new_v4().to_string())}else{body["prompt_id"].clone()},"node_errors":{}})).unwrap()
            } else if let Some(id) = path.strip_prefix("/history/") {
                if mode == "hold" {
                    b"{}".to_vec()
                } else {
                    serde_json::to_vec(&json!({id:{"status":{"completed":true,"status_str":"success"},"outputs":{"7":{"images":[{"filename":format!("moxcrew-{id}_00001_.png"),"subfolder":"","type":"output"}]}}}})).unwrap()
                }
            } else if path.starts_with("/view?") {
                kind = "image/png";
                if mode == "malformed" {
                    b"not a PNG".to_vec()
                } else {
                    raster()
                }
            } else {
                status = "404 Not Found";
                b"{}".to_vec()
            };
            let header=format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",response.len());
            let _ = stream.write_all(header.as_bytes()).await;
            let _ = stream.write_all(&response).await;
        }
    });
    Peer { url, calls, task }
}
fn project() -> String {
    format!("project:{}", uuid::Uuid::new_v4())
}
#[test]
fn image_configuration_is_private_project_scoped_and_loopback_only() {
    let data = tempfile::tempdir().unwrap();
    let p = project();
    let other = project();
    for url in [
        "https://127.0.0.1:8188",
        "http://localhost:8188",
        "http://192.168.1.2:8188",
        "http://user:secret@127.0.0.1:8188",
        "http://127.0.0.1:8188/other",
        "http://127.0.0.1:8188/?token=x",
    ] {
        assert!(configure_image_engine(data.path(), &p, url, "fixture.safetensors").is_err());
    }
    assert!(configure_image_engine(data.path(), &p, "http://127.0.0.1:8188", "../secret").is_err());
    assert!(configure_image_engine(
        data.path(),
        "../../escape",
        "http://127.0.0.1:8188",
        "fixture.safetensors"
    )
    .is_err());
    let value = configure_image_engine(
        data.path(),
        &p,
        "http://127.0.0.1:8188",
        "fixture.safetensors",
    )
    .unwrap();
    assert_eq!(value["configured"], true);
    assert!(!value.to_string().contains("127.0.0.1"));
    assert!(!data
        .path()
        .join(format!(
            "image-engine-{}.json",
            other.trim_start_matches("project:")
        ))
        .exists());
    assert_eq!(
        remove_image_engine(data.path(), &p).unwrap()["configured"],
        false
    );
    assert_eq!(
        remove_image_engine(data.path(), &p).unwrap()["configured"],
        false
    );
}
#[tokio::test]
async fn real_file_comfy_peer_generates_and_edits_fixed_graph_without_metadata_leak() {
    let server = peer("valid").await;
    let data = tempfile::tempdir().unwrap();
    let p = project();
    configure_image_engine(data.path(), &p, &server.url, "fixture.safetensors").unwrap();
    let ready = image_readiness(data.path(), &p).await.unwrap();
    assert_eq!(ready["available"], true);
    assert_eq!(ready["generationVerified"], false);
    for operation in ["images.generate", "images.edit"] {
        let mut args = json!({"prompt":"A synthetic protocol test only","width":64,"height":64,"seed":12,"steps":1});
        let inputs = if operation == "images.edit" {
            args["source"] = json!("inputs/reference.png");
            args["mask"] = json!({"source":"inputs/mask.png"});
            vec![
                ("inputs/reference.png".into(), raster()),
                ("inputs/mask.png".into(), raster()),
            ]
        } else {
            vec![]
        };
        let output = run_for_project(data.path(), &p, operation, &args, &inputs, None)
            .await
            .unwrap();
        assert_eq!(output.value["kind"], "image");
        assert_eq!(output.files.len(), 1);
        assert_eq!(&output.files[0].1[..8], b"\x89PNG\r\n\x1a\n");
        assert!(!String::from_utf8_lossy(&output.files[0].1).contains("PRIVATE_ENGINE_PROMPT"));
    }
    let calls = server.calls.lock().await;
    let prompts: Vec<_> = calls.iter().filter(|(path, _)| path == "/prompt").collect();
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts[1].1["prompt"]["4"]["class_type"],
        "VAEEncodeForInpaint"
    );
    assert_eq!(prompts[1].1["prompt"]["9"]["inputs"]["channel"], "red");
    assert_eq!(
        calls.iter().filter(|(p, _)| p == "/upload/image").count(),
        2
    );
    assert!(!calls.iter().any(|(p, _)| p == "/interrupt"));
}
#[tokio::test]
async fn image_failures_and_cancellation_never_publish_or_interrupt_other_jobs() {
    for mode in ["foreign", "malformed", "hold", "abort"] {
        let server = peer(if mode == "abort" { "hold" } else { mode }).await;
        let data = tempfile::tempdir().unwrap();
        let p = project();
        configure_image_engine(data.path(), &p, &server.url, "fixture.safetensors").unwrap();
        let active = Arc::new(AtomicBool::new(true));
        let child = active.clone();
        let path = data.path().to_owned();
        let task = tokio::spawn(async move {
            run_for_project(
                &path,
                &p,
                "images.generate",
                &json!({"prompt":"Fixture","width":64,"height":64}),
                &[],
                Some(child),
            )
            .await
        });
        let id = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some((_, body)) = server
                    .calls
                    .lock()
                    .await
                    .iter()
                    .find(|(p, _)| p == "/prompt")
                {
                    break body["prompt_id"].as_str().unwrap().to_owned();
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        if mode == "hold" {
            active.store(false, Ordering::Release);
        }
        if mode == "abort" {
            task.abort();
            assert!(matches!(task.await, Err(error) if error.is_cancelled()));
        } else {
            assert!(tokio::time::timeout(Duration::from_secs(5), task)
                .await
                .unwrap()
                .unwrap()
                .is_err());
        }
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if server
                    .calls
                    .lock()
                    .await
                    .iter()
                    .any(|(p, _)| p == &format!("/api/jobs/{id}/cancel"))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert!(!server
            .calls
            .lock()
            .await
            .iter()
            .any(|(p, _)| p == "/interrupt"));
    }
}
#[tokio::test]
async fn html_escapes_untrusted_cells_and_masks_are_actual_png_files() {
    let data = tempfile::tempdir().unwrap();
    let result=run(data.path(),"interactive.create",&json!({"kind":"table","title":"<Title>","columns":["Name"],"rows":[["</script><img src=x onerror=alert(1)>"]]}),&[],None).await.unwrap();
    let html = String::from_utf8(result.files[0].1.clone()).unwrap();
    assert!(html.contains("default-src &#x27;none&#x27;"));
    assert!(html.contains("connect-src &#x27;none&#x27;"));
    assert!(!html.contains("</script><img"));
    assert!(html.contains("\\u003c/script\\u003e"));
    assert!(!html.contains("innerHTML"));
    let mask=run(data.path(),"images.mask",&json!({"source":"inputs/reference.png","region":{"x":0.25,"y":0.25,"width":0.5,"height":0.5}}),&[("inputs/reference.png".into(),raster())],None).await.unwrap();
    assert_eq!(mask.value["width"], 64);
    assert_eq!(mask.files[0].0, "mask.png");
    assert!(run(data.path(),"images.mask",&json!({"source":"inputs/reference.png","region":{"x":0.8,"y":0.25,"width":0.5,"height":0.5}}),&[("inputs/reference.png".into(),raster())],None).await.is_err());
    assert!(run(
        data.path(),
        "audio.transcribe",
        &json!({"source":"inputs/a.wav"}),
        &[("inputs/a.wav".into(), vec![0; 8])],
        None
    )
    .await
    .is_err());
}

#[tokio::test]
async fn batch_image_conversion_reports_each_source_and_strips_metadata() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let ffmpeg = if let Ok(path) = std::env::var("MOXCREW_IMAGE_BATCH_TEST_FFMPEG") {
        path.into()
    } else {
        let path = data.path().join("fixture-ffmpeg");
        fs::write(
            &path,
            r#"#!/usr/bin/env python3
import pathlib,sys
args=sys.argv[1:]
if args==['-version']:
 print('fixture ffmpeg');sys.exit(0)
source=pathlib.Path(args[args.index('-i')+1]);target=pathlib.Path(args[-1])
data=source.read_bytes()
if not data.startswith(b'\x89PNG\r\n\x1a\n'): sys.exit(2)
target.write_bytes(data)
"#,
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    };
    configure(data.path(), "ffmpeg", ffmpeg.to_str().unwrap()).unwrap();
    let output = run(
        data.path(),
        "images.batch",
        &json!({"sources":[{"source":"inputs/good.png"},{"source":"inputs/bad.png"}],"width":64,"height":64}),
        &[
            ("inputs/good.png".into(), raster()),
            ("inputs/bad.png".into(), b"not an image".to_vec()),
        ],
        None,
    )
    .await
    .unwrap();
    assert_eq!(output.value["kind"], "image_batch");
    assert_eq!(output.value["successCount"], 1);
    assert_eq!(output.value["failedCount"], 1);
    assert_eq!(output.value["items"][0]["status"], "converted");
    assert_eq!(output.value["items"][0]["width"], 64);
    assert_eq!(output.value["items"][0]["height"], 64);
    assert_eq!(
        output.value["items"][1]["reasonCode"],
        "PRODUCTION_IMAGE_CONVERSION_FAILED"
    );
    assert!(output.files.iter().any(|(name, bytes)| {
        name == "image-01.png"
            && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
            && !String::from_utf8_lossy(bytes).contains("PRIVATE_ENGINE_PROMPT")
    }));
    assert!(!output.files.iter().any(|(name, _)| name == "image-02.png"));
    assert!(output.files.iter().any(|(name, bytes)| {
        name == "batch.json" && serde_json::from_slice::<Value>(bytes).unwrap()["failedCount"] == 1
    }));
    assert!(run(
        data.path(),
        "images.batch",
        &json!({"sources":[{"source":"inputs/good.png"}],"width":64,"height":64}),
        &[("inputs/good.png".into(), raster())],
        None,
    )
    .await
    .is_err());
    let too_many = json!({"sources":(0..13).map(|index|json!({"source":format!("inputs/{index}.png")})).collect::<Vec<_>>()});
    assert_eq!(
        input_paths(&too_many).unwrap_err().0,
        "PRODUCTION_INPUT_LIMIT"
    );
}

#[tokio::test]
async fn podcast_edit_packages_ordered_chapters_and_checked_audio() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let actual = std::env::var("MOXCREW_PODCAST_TEST_FFMPEG")
        .ok()
        .zip(std::env::var("MOXCREW_PODCAST_TEST_FFPROBE").ok());
    let (ffmpeg, ffprobe) = if let Some(paths) = actual {
        paths
    } else {
        let script = r#"#!/usr/bin/env python3
import json,pathlib,sys
kind=pathlib.Path(sys.argv[0]).name
args=sys.argv[1:]
if args==['-version']:
 print('fixture');sys.exit(0)
target=pathlib.Path(args[-1])
if 'ffprobe' in kind:
 duration='0.9' if target.name=='podcast.m4a' else '2.0'
 print(json.dumps({'format':{'duration':duration},'streams':[{'codec_type':'audio','sample_rate':'48000','channels':2}]}))
else:
 target.write_bytes(b'RIFF fixture' if target.suffix=='.wav' else b'0000ftypM4A fixture')
"#;
        let ffmpeg = data.path().join("fixture-ffmpeg");
        let ffprobe = data.path().join("fixture-ffprobe");
        for path in [&ffmpeg, &ffprobe] {
            fs::write(path, script).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        (
            ffmpeg.to_string_lossy().into_owned(),
            ffprobe.to_string_lossy().into_owned(),
        )
    };
    configure(data.path(), "ffmpeg", &ffmpeg).unwrap();
    configure(data.path(), "ffprobe", &ffprobe).unwrap();
    let arguments = json!({
        "title":"A bounded episode",
        "clips":[
            {"source":"inputs/raw.wav","start":0.1,"end":0.5,"title":"Opening"},
            {"source":"inputs/raw.wav","start":0.8,"end":1.3,"title":"Main point"}
        ]
    });
    let output = run(
        data.path(),
        "audio.podcast",
        &arguments,
        &[("inputs/raw.wav".into(), wav(2))],
        None,
    )
    .await
    .unwrap();
    assert_eq!(output.value["kind"], "podcast");
    assert_eq!(output.value["chapterCount"], 2);
    assert_eq!(output.value["chapters"][0]["start"], 0.0);
    assert_eq!(output.value["chapters"][0]["end"], 0.4);
    assert_eq!(output.value["chapters"][1]["start"], 0.4);
    assert_eq!(output.value["chapters"][1]["end"], 0.9);
    assert!(output.files.iter().any(|(name, bytes)| {
        name == "podcast.m4a" && bytes.windows(4).any(|window| window == b"ftyp")
    }));
    let chapters = output
        .files
        .iter()
        .find(|(name, _)| name == "chapters.json")
        .map(|(_, bytes)| serde_json::from_slice::<Value>(bytes).unwrap())
        .unwrap();
    assert_eq!(chapters["chapters"][1]["title"], "Main point");
    assert!(!chapters.to_string().contains("inputs/"));
    let invalid = json!({"title":"Invalid","clips":[{"source":"inputs/raw.wav","start":0,"end":3,"title":"Too long"}]});
    assert!(run(
        data.path(),
        "audio.podcast",
        &invalid,
        &[("inputs/raw.wav".into(), wav(2))],
        None,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn csv_chart_aggregates_checked_values_and_escapes_static_outputs() {
    let data = tempfile::tempdir().unwrap();
    let source = b"region,revenue\nNorth,10\n<script>,4\nNorth,2\n".to_vec();
    let output = run(
        data.path(),
        "data.chart",
        &json!({
            "source":"inputs/sales.csv",
            "title":"Revenue <overview>\n# injected",
            "categoryColumn":"region",
            "valueColumn":"revenue",
            "aggregation":"sum"
        }),
        &[("inputs/sales.csv".into(), source.clone())],
        None,
    )
    .await
    .unwrap();
    assert_eq!(output.value["kind"], "data_chart");
    assert_eq!(output.value["rowsAnalyzed"], 3);
    assert_eq!(output.value["categoryCount"], 2);
    assert_eq!(output.value["series"][0]["label"], "North");
    assert_eq!(output.value["series"][0]["value"], 12.0);
    assert_eq!(output.value["series"][1]["value"], 4.0);
    let svg = output
        .files
        .iter()
        .find(|(name, _)| name == "chart.svg")
        .map(|(_, bytes)| String::from_utf8(bytes.clone()).unwrap())
        .unwrap();
    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("Revenue &lt;overview&gt;"));
    assert!(svg.contains("&lt;script&gt;"));
    assert!(!svg.contains("<script>"));
    assert!(!svg.contains("href=") && !svg.contains("<foreignObject"));
    let report = output
        .files
        .iter()
        .find(|(name, _)| name == "report.md")
        .map(|(_, bytes)| String::from_utf8(bytes.clone()).unwrap())
        .unwrap();
    assert!(report.contains("| North | 12 | 2 |"));
    assert!(report.contains("&lt;script&gt;"));
    assert!(!report.contains("<script>"));
    assert!(report.contains("\\# injected"));
    assert!(!report.contains("\n# injected"));
    assert!(!output.value.to_string().contains("inputs/"));
    assert!(run(
        data.path(),
        "data.chart",
        &json!({"source":"inputs/sales.csv","title":"Invalid","categoryColumn":"region","valueColumn":"revenue","aggregation":"average"}),
        &[(
            "inputs/sales.csv".into(),
            b"region,revenue\nNorth,not-a-number\n".to_vec()
        )],
        None,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn scenario_workbook_has_checked_totals_fixed_formulas_and_bounded_inputs() {
    let data = tempfile::tempdir().unwrap();
    let source = b"line_item,base_plan,growth_case,lean_case\nHosting,100,125,80\nSales <script>,200,260,150\n".to_vec();
    let args = json!({
        "source":"inputs/budget.csv",
        "title":"2027 <Plan>\n# draft",
        "itemColumn":"line_item",
        "baseColumn":"base_plan",
        "upsideColumn":"growth_case",
        "downsideColumn":"lean_case",
        "currency":"USD"
    });
    let output = run(
        data.path(),
        "data.scenario",
        &args,
        &[("inputs/budget.csv".into(), source)],
        None,
    )
    .await
    .unwrap();
    assert_eq!(output.value["kind"], "data_scenario");
    assert_eq!(output.value["rowCount"], 2);
    assert_eq!(output.value["formulaCells"], 3);
    assert_eq!(output.value["recalculation"], "automatic");
    assert_eq!(output.value["scenarios"][0]["total"], 300.0);
    assert_eq!(output.value["scenarios"][1]["total"], 385.0);
    assert_eq!(output.value["scenarios"][2]["total"], 230.0);
    assert_eq!(output.value["items"][1]["label"], "Sales <script>");
    assert!(!output.value.to_string().contains("inputs/"));
    let workbook = output
        .files
        .iter()
        .find(|(name, _)| name == "scenario.xlsx")
        .map(|(_, bytes)| bytes)
        .unwrap();
    assert!(workbook.starts_with(b"PK\x03\x04"));
    let report = output
        .files
        .iter()
        .find(|(name, _)| name == "report.md")
        .map(|(_, bytes)| String::from_utf8(bytes.clone()).unwrap())
        .unwrap();
    assert!(report.contains("| Base | 300 |"));
    assert!(report.contains("2027 &lt;Plan&gt; \\# draft"));
    assert!(!report.contains("\n# draft"));
    assert!(run(
        data.path(),
        "data.scenario",
        &args,
        &[(
            "inputs/budget.csv".into(),
            b"line_item,base_plan,growth_case,lean_case\nHosting,nope,125,80\n".to_vec()
        )],
        None,
    )
    .await
    .is_err());
    let mut too_many = b"line_item,base_plan,growth_case,lean_case\n".to_vec();
    for index in 0..=5000 {
        too_many.extend_from_slice(format!("Item {index},1,2,3\n").as_bytes());
    }
    assert!(run(
        data.path(),
        "data.scenario",
        &args,
        &[("inputs/budget.csv".into(), too_many)],
        None,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn structured_extraction_requires_literal_evidence_and_exposes_uncertainty_and_missing() {
    use sha2::{Digest, Sha256};
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let script = data.path().join("extract-peer");
    fs::write(
        &script,
        r#"#!/usr/bin/python3
import pathlib,sys
kind=pathlib.Path(sys.argv[0]).name
args=sys.argv[1:]
if kind=='pdftotext':
 assert args[0]=='-layout' and args[-1]=='-'
 print('Vendor: Northwind AI\nRenewal: late Q2')
elif kind=='tesseract':
 assert args[1:]==['stdout','--psm','3']
 print('Vendor: Paper Kite\nSeats: about 40')
else: sys.exit(2)
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    for tool in ["pdftotext", "tesseract"] {
        let path = data.path().join(tool);
        fs::copy(&script, &path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        configure(data.path(), tool, path.to_str().unwrap()).unwrap();
    }
    let sources = vec![
        (
            "inputs/atlas.txt".into(),
            b"Vendor: =Atlas Cloud\nRenewal: 2027-04-30\nSeats: 120\n".to_vec(),
        ),
        ("inputs/northwind.pdf".into(), b"fixture pdf".to_vec()),
        ("inputs/paper.png".into(), raster()),
    ];
    let args = json!({
        "title":"Vendor review <2027>\n# checked",
        "sources":[
            {"source":"inputs/atlas.txt","label":"Atlas brief"},
            {"source":"inputs/northwind.pdf","label":"Northwind PDF"},
            {"source":"inputs/paper.png","label":"Paper scan"}
        ],
        "columns":[
            {"key":"vendor","label":"Vendor"},
            {"key":"renewal","label":"Renewal"},
            {"key":"seats","label":"Seats"}
        ],
        "rows":[
            {"source":"inputs/atlas.txt","cells":{
                "vendor":{"status":"confirmed","value":"=Atlas Cloud","evidence":"Vendor: =Atlas Cloud"},
                "renewal":{"status":"confirmed","value":"2027-04-30","evidence":"Renewal: 2027-04-30"},
                "seats":{"status":"confirmed","value":"120","evidence":"Seats: 120"}
            }},
            {"source":"inputs/northwind.pdf","cells":{
                "vendor":{"status":"confirmed","value":"Northwind AI","evidence":"Vendor: Northwind AI"},
                "renewal":{"status":"uncertain","value":"late Q2","evidence":"Renewal: late Q2"},
                "seats":{"status":"missing","value":null,"evidence":null}
            }},
            {"source":"inputs/paper.png","cells":{
                "vendor":{"status":"confirmed","value":"Paper Kite","evidence":"Vendor: Paper Kite"},
                "renewal":{"status":"missing","value":null,"evidence":null},
                "seats":{"status":"uncertain","value":"about 40","evidence":"Seats: about 40"}
            }}
        ]
    });
    let output = run(data.path(), "data.extract", &args, &sources, None)
        .await
        .unwrap();
    assert_eq!(output.value["kind"], "data_extraction");
    assert_eq!(output.value["rowCount"], 3);
    assert_eq!(output.value["cellCount"], 9);
    assert_eq!(output.value["statusCounts"]["confirmed"], 5);
    assert_eq!(output.value["statusCounts"]["uncertain"], 2);
    assert_eq!(output.value["statusCounts"]["missing"], 2);
    assert_eq!(output.value["rows"][0]["cells"]["renewal"]["line"], 2);
    let atlas = b"Vendor: =Atlas Cloud\nRenewal: 2027-04-30\nSeats: 120\n";
    assert_eq!(
        output.value["sources"][0]["sourceSha256"],
        format!("{:x}", Sha256::digest(atlas))
    );
    assert_eq!(
        output.value["sources"][0]["textSha256"],
        format!("{:x}", Sha256::digest(atlas))
    );
    assert_eq!(
        output.value["rows"][1]["cells"]["seats"]["value"],
        Value::Null
    );
    assert!(!output.value.to_string().contains("inputs/"));
    assert!(!output
        .value
        .to_string()
        .contains(data.path().to_string_lossy().as_ref()));
    let csv = output
        .files
        .iter()
        .find(|(name, _)| name == "extracted-table.csv")
        .map(|(_, bytes)| String::from_utf8(bytes.clone()).unwrap())
        .unwrap();
    assert!(csv.contains("Renewal status"));
    assert!(csv.contains("'=Atlas Cloud"));
    assert!(csv.contains("late Q2,uncertain,2"));
    assert!(csv.contains(",missing,"));
    let report = output
        .files
        .iter()
        .find(|(name, _)| name == "report.md")
        .map(|(_, bytes)| String::from_utf8(bytes.clone()).unwrap())
        .unwrap();
    assert!(report.contains("Vendor review &lt;2027&gt; \\# checked"));
    assert!(report.contains("- Uncertain cells: 2"));
    let mut fabricated = args.clone();
    fabricated["rows"][0]["cells"]["vendor"]["evidence"] = json!("Vendor: Fabricated");
    assert!(
        run(data.path(), "data.extract", &fabricated, &sources, None)
            .await
            .is_err()
    );
    let mut hidden_missing = args;
    hidden_missing["rows"][1]["cells"]["seats"]["value"] = json!("unknown");
    assert!(
        run(data.path(), "data.extract", &hidden_missing, &sources, None)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn attributed_brief_keeps_literal_sources_open_questions_and_unassigned_actions() {
    use sha2::{Digest, Sha256};
    let data = tempfile::tempdir().unwrap();
    let atlas = b"Decision: Launch pilot in May\nOwner: Ece\nBudget approval is pending\n";
    let northwind = b"Risk: Security review has not started\nNext step: Schedule review\n";
    let sources = vec![
        ("inputs/atlas.md".into(), atlas.to_vec()),
        ("inputs/northwind.txt".into(), northwind.to_vec()),
    ];
    let args = json!({
        "title":"Pilot brief <review>",
        "sources":[
            {"source":"inputs/atlas.md","label":"Atlas <notes>"},
            {"source":"inputs/northwind.txt","label":"Northwind notes"}
        ],
        "findings":[
            {"source":"inputs/atlas.md","summary":"The pilot is intended for May.","evidence":"Decision: Launch pilot in May"},
            {"source":"inputs/northwind.txt","summary":"Security review remains outstanding.","evidence":"Risk: Security review has not started"}
        ],
        "questions":[
            {"question":"Who approves the budget?","reason":"The notes say approval is pending but name no approver."}
        ],
        "actions":[
            {"source":"inputs/northwind.txt","task":"Schedule the security review","evidence":"Next step: Schedule review","owner":null,"due":null},
            {"source":"inputs/atlas.md","task":"Confirm the pilot owner","evidence":"Owner: Ece","owner":"Ece","due":"2027-05-01"}
        ]
    });
    let output = run(data.path(), "documents.brief", &args, &sources, None)
        .await
        .unwrap();
    assert_eq!(output.value["kind"], "attributed_brief");
    assert_eq!(
        output.value["counts"],
        json!({"sources":2,"findings":2,"questions":1,"actions":2})
    );
    assert_eq!(output.value["findings"][0]["line"], 1);
    assert_eq!(output.value["actions"][0]["owner"], Value::Null);
    assert_eq!(output.value["actions"][0]["due"], Value::Null);
    assert_eq!(
        output.value["sources"][0]["sourceSha256"],
        format!("{:x}", Sha256::digest(atlas))
    );
    assert!(!output.value.to_string().contains("inputs/"));
    assert!(!output
        .value
        .to_string()
        .contains(data.path().to_string_lossy().as_ref()));
    let brief: Value = serde_json::from_slice(
        &output
            .files
            .iter()
            .find(|(name, _)| name == "brief.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(brief["kind"], output.value["kind"]);
    assert_eq!(brief["sources"], output.value["sources"]);
    assert_eq!(brief["findings"], output.value["findings"]);
    assert_eq!(brief["questions"], output.value["questions"]);
    assert_eq!(brief["actions"], output.value["actions"]);
    assert!(brief.get("files").is_none());
    let document = output
        .files
        .iter()
        .find(|(name, _)| name == "document.docx")
        .unwrap();
    assert!(document.1.starts_with(b"PK"));
    let reopened = run(
        data.path(),
        "inspect",
        &json!({"source":"inputs/brief.docx"}),
        &[("inputs/brief.docx".into(), document.1.clone())],
        None,
    )
    .await
    .unwrap();
    let reopened = reopened.value["pages"].to_string();
    assert!(reopened.contains("Atlas <notes>"));
    assert!(reopened.contains("Who approves the budget?"));
    assert!(reopened.contains("Owner · Unassigned"));
    assert!(reopened.contains("Due · Not set"));
    let mut fabricated = args.clone();
    fabricated["findings"][0]["evidence"] = json!("Decision: Fabricated");
    assert!(
        run(data.path(), "documents.brief", &fabricated, &sources, None)
            .await
            .is_err()
    );
    let mut too_large = args.clone();
    too_large["findings"] = json!(vec![args["findings"][0].clone(); 100]);
    too_large["questions"] = json!(vec![args["questions"][0].clone(); 50]);
    too_large["actions"] = json!(vec![args["actions"][0].clone(); 100]);
    assert!(
        run(data.path(), "documents.brief", &too_large, &sources, None)
            .await
            .is_err()
    );
    let mut invalid_date = args;
    invalid_date["actions"][1]["due"] = json!("2027-02-29");
    assert!(run(
        data.path(),
        "documents.brief",
        &invalid_date,
        &sources,
        None
    )
    .await
    .is_err());
}

#[tokio::test]
async fn document_rewrite_pins_exact_source_segments_and_preserves_stable_facts() {
    use sha2::{Digest, Sha256};
    let data = tempfile::tempdir().unwrap();
    let source =
        b"Moxcrew launches on 2027-03-15.\nDetails: https://mox.example/launch\nBudget: 1,250 USD."
            .to_vec();
    let segments = json!([
        {"source":"Moxcrew launches on 2027-03-15.","result":"Moxcrew 2027-03-15 tarihinde kullanıma açılıyor."},
        {"source":"Details: https://mox.example/launch","result":"Ayrıntılar: https://mox.example/launch"},
        {"source":"Budget: 1,250 USD.","result":"Bütçe: 1,250 USD."}
    ]);
    let output = run(
        data.path(),
        "documents.rewrite",
        &json!({
            "source":"inputs/launch.txt",
            "mode":"translate",
            "target":"Turkish",
            "preserve":["Moxcrew"],
            "segments":segments
        }),
        &[("inputs/launch.txt".into(), source.clone())],
        None,
    )
    .await
    .unwrap();
    assert_eq!(output.value["kind"], "document_rewrite");
    assert_eq!(output.value["mode"], "translate");
    assert_eq!(output.value["sourceFormat"], "txt");
    assert_eq!(output.value["segmentCount"], 3);
    assert_eq!(
        output.value["sourceSha256"],
        format!("{:x}", Sha256::digest(&source))
    );
    assert!(output.value["automaticFacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "2027-03-15"));
    assert!(output.value["automaticFacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "https://mox.example/launch"));
    assert!(!output.value.to_string().contains("inputs/"));
    assert!(!output
        .value
        .to_string()
        .contains(data.path().to_string_lossy().as_ref()));
    let document = output
        .files
        .iter()
        .find(|(name, _)| name == "document.docx")
        .unwrap();
    let reopened = run(
        data.path(),
        "inspect",
        &json!({"source":"inputs/result.docx"}),
        &[("inputs/result.docx".into(), document.1.clone())],
        None,
    )
    .await
    .unwrap();
    let pages = reopened.value["pages"].to_string();
    assert!(pages.contains("Moxcrew 2027-03-15 tarihinde"));
    assert!(pages.contains("https://mox.example/launch"));
    assert!(pages.contains("1,250 USD"));
    let audit: Value = serde_json::from_slice(
        &output
            .files
            .iter()
            .find(|(name, _)| name == "rewrite-audit.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(audit["segments"], output.value["segments"]);
    let comparison = String::from_utf8(
        output
            .files
            .iter()
            .find(|(name, _)| name == "rewrite-comparison.md")
            .unwrap()
            .1
            .clone(),
    )
    .unwrap();
    assert!(comparison.contains("**Source**"));
    assert!(comparison.contains("**Result**"));

    let mut changed_source = segments.clone();
    changed_source[0]["source"] = json!("A different source sentence.");
    assert!(run(
        data.path(),
        "documents.rewrite",
        &json!({"source":"inputs/launch.txt","mode":"translate","target":"Turkish","preserve":["Moxcrew"],"segments":changed_source}),
        &[("inputs/launch.txt".into(), source.clone())],
        None,
    )
    .await
    .is_err());
    let mut changed_fact = segments.clone();
    changed_fact[1]["result"] = json!("Ayrıntılar daha sonra paylaşılacak.");
    assert!(run(
        data.path(),
        "documents.rewrite",
        &json!({"source":"inputs/launch.txt","mode":"translate","target":"Turkish","preserve":["Moxcrew"],"segments":changed_fact}),
        &[("inputs/launch.txt".into(), source.clone())],
        None,
    )
    .await
    .is_err());
    let unchanged = json!([
        {"source":"Moxcrew launches on 2027-03-15.","result":"Moxcrew launches on 2027-03-15."},
        {"source":"Details: https://mox.example/launch","result":"Details: https://mox.example/launch"},
        {"source":"Budget: 1,250 USD.","result":"Budget: 1,250 USD."}
    ]);
    assert!(run(
        data.path(),
        "documents.rewrite",
        &json!({"source":"inputs/launch.txt","mode":"rewrite","target":"Concise","preserve":["Moxcrew"],"segments":unchanged}),
        &[("inputs/launch.txt".into(), source.clone())],
        None,
    )
    .await
    .is_err());

    let created = run(
        data.path(),
        "documents.create",
        &json!({"paragraphs":["Owner: Moxcrew","Deadline: 2027-03-15"]}),
        &[],
        None,
    )
    .await
    .unwrap();
    let docx = created
        .files
        .iter()
        .find(|(name, _)| name == "document.docx")
        .unwrap();
    let rewritten = run(
        data.path(),
        "documents.rewrite",
        &json!({
            "source":"inputs/source.docx",
            "mode":"rewrite",
            "target":"Concise executive tone",
            "preserve":["Moxcrew"],
            "segments":[
                {"source":"Owner: Moxcrew","result":"Owner · Moxcrew"},
                {"source":"Deadline: 2027-03-15","result":"Deadline · 2027-03-15"}
            ]
        }),
        &[("inputs/source.docx".into(), docx.1.clone())],
        None,
    )
    .await
    .unwrap();
    assert_eq!(rewritten.value["sourceFormat"], "docx");
    assert_eq!(rewritten.value["mode"], "rewrite");
}

#[tokio::test]
async fn docx_template_fill_replaces_exact_fields_and_keeps_missing_values_visible() {
    use sha2::{Digest, Sha256};
    let data = tempfile::tempdir().unwrap();
    let created = run(
        data.path(),
        "documents.create",
        &json!({"title":"Partner onboarding","paragraphs":[
            "Client: {{client_name}}",
            "Approval date: {{approval_date}}",
            "Contact: {{contact_email}}"
        ]}),
        &[],
        None,
    )
    .await
    .unwrap();
    let template = created
        .files
        .iter()
        .find(|(name, _)| name == "document.docx")
        .unwrap()
        .1
        .clone();
    let fields = json!([
        {"key":"client_name","status":"provided","value":"Atlas <Cloud>"},
        {"key":"approval_date","status":"missing","value":null},
        {"key":"contact_email","status":"provided","value":"ops@atlas.example"}
    ]);
    let output = run(
        data.path(),
        "documents.template",
        &json!({"source":"inputs/onboarding.docx","fields":fields}),
        &[("inputs/onboarding.docx".into(), template.clone())],
        None,
    )
    .await
    .unwrap();
    assert_eq!(output.value["kind"], "filled_template");
    assert_eq!(output.value["fieldCount"], 3);
    assert_eq!(output.value["providedCount"], 2);
    assert_eq!(output.value["missingCount"], 1);
    assert_eq!(
        output.value["templateSha256"],
        format!("{:x}", Sha256::digest(&template))
    );
    assert!(!output.value.to_string().contains("inputs/"));
    assert!(!output
        .value
        .to_string()
        .contains(data.path().to_string_lossy().as_ref()));
    let filled = output
        .files
        .iter()
        .find(|(name, _)| name == "filled-template.docx")
        .unwrap();
    let reopened = run(
        data.path(),
        "inspect",
        &json!({"source":"inputs/filled.docx"}),
        &[("inputs/filled.docx".into(), filled.1.clone())],
        None,
    )
    .await
    .unwrap();
    let reopened = reopened.value["pages"].to_string();
    assert!(reopened.contains("Atlas <Cloud>"));
    assert!(reopened.contains("[MISSING: approval_date]"));
    assert!(reopened.contains("ops@atlas.example"));
    assert!(!reopened.contains("{{"));
    let audit: Value = serde_json::from_slice(
        &output
            .files
            .iter()
            .find(|(name, _)| name == "template-fill.json")
            .unwrap()
            .1,
    )
    .unwrap();
    assert_eq!(audit["fields"], output.value["fields"]);
    assert!(audit.get("files").is_none());
    let split_template = base64::engine::general_purpose::STANDARD.decode("UEsDBBQAAAAIAHazZF33S4B1wgAAAHYBAAATAAAAW0NvbnRlbnRfVHlwZXNdLnhtbH2QuQ7CMAyGX6XKiqgRAwOiLMAKDLyAlbptRC7FLsfbk3INCBjt//gsLw7XSFxcnPVcqU4kzgFYd+SQyxDJZ6UJyaHkMbUQUR+xJZhOJjPQwQt5GcvQoZaLNTXYWyk2l7xmE3ylEllWxephHFiVwhit0ShZh5OvPyjjJ6HMybuHOxN5lA0KvhIG5TfgmdudKCVTU7HHJFt02QXnkGqog+5dTpb/a77cGZrGaHrnh7aYgiZm41tny7fi0PjX/XB/9/IGUEsDBBQAAAAIAHazZF1hey9DiQAAAPIAAAALAAAAX3JlbHMvLnJlbHONzzsOAiEQBuCrEA6ws1pYGKCy2dZ4AQLDIy6PDBj19lJYrMbCcuaffH9GnHHVPZbcQqyNPdKam+Sh93oEaCZg0m0qFfNIXKGk+xjJQ9Xmqj3Cfp4PQFuDK7E12WIlp8XuOLs8K/5jF+eiwVMxt4S5/6j4uhiyJo9d8nshC/a9ngbLQQn4eFG9AFBLAwQUAAAACAB2s2Rd7wgntZMAAADfAAAAEQAAAHdvcmQvZG9jdW1lbnQueG1sVU9BDoMwDPsK4gEL2mGHivWyD/CDqbQdIDVtlWbqJsTfB2Vi2sWO5cRW2ixM0E+0nqsXOp9EvtYjcxQASY8WVTqFaP3qPQKh4lXSADmQiRS0TWnyAzo4N80FUE2+lm0WfTDvjeMGVKAr1INs4VAsb25am0U1z7pM983lfee4ZekV2mX58+CbnqzmjvbYvRZ+L8kPUEsBAhQDFAAAAAgAdrNkXfdLgHXCAAAAdgEAABMAAAAAAAAAAAAAAIABAAAAAFtDb250ZW50X1R5cGVzXS54bWxQSwECFAMUAAAACAB2s2RdYXsvQ4kAAADyAAAACwAAAAAAAAAAAAAAgAHzAAAAX3JlbHMvLnJlbHNQSwECFAMUAAAACAB2s2Rd7wgntZMAAADfAAAAEQAAAAAAAAAAAAAAgAGlAQAAd29yZC9kb2N1bWVudC54bWxQSwUGAAAAAAMAAwC5AAAAZwIAAAAA").unwrap();
    let split = run(
        data.path(),
        "documents.template",
        &json!({"source":"inputs/split.docx","fields":[{"key":"client_name","status":"provided","value":"Split Run Ltd"}]}),
        &[("inputs/split.docx".into(), split_template)],
        None,
    )
    .await
    .unwrap();
    let split_document = split
        .files
        .iter()
        .find(|(name, _)| name == "filled-template.docx")
        .unwrap();
    let split_reopened = run(
        data.path(),
        "inspect",
        &json!({"source":"inputs/split-filled.docx"}),
        &[("inputs/split-filled.docx".into(), split_document.1.clone())],
        None,
    )
    .await
    .unwrap();
    assert!(split_reopened.value["pages"]
        .to_string()
        .contains("Client: Split Run Ltd"));
    let mut incomplete = fields.clone();
    incomplete.as_array_mut().unwrap().pop();
    assert!(run(
        data.path(),
        "documents.template",
        &json!({"source":"inputs/onboarding.docx","fields":incomplete}),
        &[("inputs/onboarding.docx".into(), template.clone())],
        None,
    )
    .await
    .is_err());
    let mut extra = fields.clone();
    extra.as_array_mut().unwrap().push(json!({
        "key":"unknown_field","status":"provided","value":"not in template"
    }));
    assert!(run(
        data.path(),
        "documents.template",
        &json!({"source":"inputs/onboarding.docx","fields":extra}),
        &[("inputs/onboarding.docx".into(), template.clone())],
        None,
    )
    .await
    .is_err());
    let malformed = run(
        data.path(),
        "documents.create",
        &json!({"paragraphs":["Client: {{Client Name}}"]}),
        &[],
        None,
    )
    .await
    .unwrap();
    let malformed = malformed
        .files
        .iter()
        .find(|(name, _)| name == "document.docx")
        .unwrap();
    assert!(run(
        data.path(),
        "documents.template",
        &json!({"source":"inputs/malformed.docx","fields":[
            {"key":"client_name","status":"provided","value":"Atlas Cloud"}
        ]}),
        &[("inputs/malformed.docx".into(), malformed.1.clone())],
        None,
    )
    .await
    .is_err());
    let mut hidden_missing = fields;
    hidden_missing[1]["value"] = json!("unknown");
    assert!(run(
        data.path(),
        "documents.template",
        &json!({"source":"inputs/onboarding.docx","fields":hidden_missing}),
        &[("inputs/onboarding.docx".into(), template)],
        None,
    )
    .await
    .is_err());
}

#[tokio::test]
async fn caption_renderer_is_required_and_fallback_rejects_unbounded_work_before_encoding() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let ffmpeg = data.path().join("fixture-ffmpeg");
    let renderer = data.path().join("fixture-renderer");
    fs::write(&renderer, "#!/bin/sh\nexit 91\n").unwrap();
    fs::set_permissions(&renderer, fs::Permissions::from_mode(0o700)).unwrap();
    configure(data.path(), "libreoffice", renderer.to_str().unwrap()).unwrap();
    configure(data.path(), "pdftoppm", renderer.to_str().unwrap()).unwrap();
    for (filters, expected) in [
        ("", "PRODUCTION_CAPTION_RENDERER_REQUIRED"),
        ("... overlay VV->V", "PRODUCTION_CAPTION_RENDERER_LIMIT"),
    ] {
        fs::write(
            &ffmpeg,
            format!("#!/bin/sh\n[ \"$2\" = '-filters' ] || exit 92\nprintf '%s\\n' '{filters}'\n"),
        )
        .unwrap();
        fs::set_permissions(&ffmpeg, fs::Permissions::from_mode(0o700)).unwrap();
        configure(data.path(), "ffmpeg", ffmpeg.to_str().unwrap()).unwrap();
        let result = run(
            data.path(),
            "video.package",
            &json!({"clips":[{"source":"inputs/video.mp4","start":0,"end":1}],
                "aspects":["square"],"title":"Bounded caption",
                "captions":[{"start":0,"end":1,"text":"a".repeat(121)}]}),
            &[("inputs/video.mp4".into(), vec![0; 8])],
            None,
        )
        .await;
        assert_eq!(result.err().unwrap().0, expected);
    }
}

#[tokio::test]
async fn ocr_and_transcript_adapters_preserve_engine_text_and_timing_without_native_details() {
    use std::os::unix::fs::PermissionsExt;
    let data = tempfile::tempdir().unwrap();
    let bin = data.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let script = r#"#!/usr/bin/python3
import json,pathlib,sys
kind=pathlib.Path(sys.argv[0]).name
args=sys.argv[1:]
if args in [['--version'],['--help'],['-version']]: print('fixture');sys.exit(0)
if kind=='tesseract':
 assert args[1]=='stdout' and args[2:]==['-l','eng','--psm','3']
 print('Observed OCR text')
elif kind=='ffprobe':
 print(json.dumps({'format':{'duration':'0.5'},'streams':[{'codec_type':'audio','sample_rate':'16000','channels':1}]}))
elif kind=='ffmpeg':
 assert args[args.index('-ar')+1]=='16000'
 pathlib.Path(args[-1]).write_bytes(pathlib.Path(args[args.index('-i')+1]).read_bytes())
elif kind=='whisper':
 assert args[args.index('-l')+1]=='en' and '-oj' in args and '-np' in args
 out=pathlib.Path(args[args.index('-of')+1]+'.json')
 out.write_text(json.dumps({'params':{'model':'/private/DO_NOT_EXPORT_MODEL'},'result':{'language':'en'},'transcription':[{'offsets':{'from':0,'to':400},'text':' Engine transcription '}]}))
else: sys.exit(2)
"#;
    for id in ["tesseract", "ffmpeg", "ffprobe", "whisper"] {
        let path = bin.join(id);
        fs::write(&path, script).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        configure(data.path(), id, path.to_str().unwrap()).unwrap();
    }
    let model = data.path().join("fixture-model.bin");
    fs::write(&model, b"ggml").unwrap();
    configure(data.path(), "whisperModel", model.to_str().unwrap()).unwrap();
    let ocr = run(
        data.path(),
        "resources.ocr",
        &json!({"source":"inputs/image.png"}),
        &[("inputs/image.png".into(), raster())],
        None,
    )
    .await
    .unwrap();
    assert_eq!(ocr.value["text"], "Observed OCR text\n");
    let transcript = run(
        data.path(),
        "audio.transcribe",
        &json!({"source":"inputs/speech.wav","language":"en"}),
        &[(
            "inputs/speech.wav".into(),
            b"RIFF deterministic adapter peer input".to_vec(),
        )],
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        transcript.value["segments"][0],
        json!({"start":0.0,"end":0.4,"text":"Engine transcription"})
    );
    assert!(!transcript.value.to_string().contains("DO_NOT_EXPORT_MODEL"));
    assert!(transcript
        .files
        .iter()
        .any(|(name, data)| name == "captions.srt"
            && String::from_utf8_lossy(data).contains("00:00:00,000 --> 00:00:00,400")));
    for (_, bytes) in transcript.files {
        assert!(!String::from_utf8_lossy(&bytes).contains("DO_NOT_EXPORT_MODEL"));
    }
}
