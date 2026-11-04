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
