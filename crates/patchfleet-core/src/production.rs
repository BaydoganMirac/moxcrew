//! Typed, bounded child tools; the Host owns permissions and publishes checked files.
use crate::{store, Error, Result};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

#[path = "production_image.rs"]
mod image_engine;
pub(crate) use image_engine::{configure_image_engine, image_readiness, remove_image_engine};

const CAPABILITIES: &str = include_str!("../assets/tools/production_capabilities.py");
const SCRIPT: &str = include_str!("../assets/tools/production.py");
const LIMIT: usize = 25 * 1024 * 1024;
pub(crate) const OPERATIONS: &[&str] = &[
    "presentations.create",
    "presentations.revise",
    "documents.create",
    "documents.revise",
    "data.clean",
    "data.analyze",
    "data.xlsx",
    "images.resize",
    "images.batch",
    "images.mask",
    "video.edit",
    "audio.cleanup",
    "audio.podcast",
    "resources.extract",
    "resources.ocr",
    "audio.transcribe",
    "documents.render",
    "video.frame",
    "video.package",
    "interactive.create",
    "images.generate",
    "images.edit",
    "browser.inspect",
    "browser.draft",
];
fn error(code: &str) -> Error {
    Error(code.into())
}

fn configured(data: &Path) -> Result<Value> {
    let path = data.join("production-tools.json");
    if !path.exists() {
        return Ok(json!({}));
    }
    let bytes = store::read_private_file(&path)?;
    if bytes.len() > 32 * 1024 {
        return Err(error("PRODUCTION_CONFIG_INVALID"));
    }
    let value: Value = serde_json::from_slice(&bytes)?;
    if !value.is_object() {
        return Err(error("PRODUCTION_CONFIG_INVALID"));
    }
    Ok(value)
}

fn discover(name: &str) -> Option<PathBuf> {
    let candidate = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|p| p.join(candidate.as_str()))
        .find(|p| p.is_file())
        .and_then(|p| fs::canonicalize(p).ok())
}
pub(crate) fn paths(data: &Path) -> Result<Value> {
    let configured = configured(data)?;
    let mut paths = json!({});
    for (id, binary) in [
        ("python", if cfg!(windows) { "python" } else { "python3" }),
        ("ffmpeg", "ffmpeg"),
        ("ffprobe", "ffprobe"),
        ("pdftotext", "pdftotext"),
        ("pdftoppm", "pdftoppm"),
        ("tesseract", "tesseract"),
        ("whisper", "whisper-cli"),
        ("whisperModel", "moxcrew-no-default-whisper-model"),
        ("libreoffice", "soffice"),
        ("chrome", "google-chrome"),
    ] {
        let path = configured[id]
            .as_str()
            .map(PathBuf::from)
            .or_else(|| discover(binary))
            .or_else(|| match id {
                "libreoffice" => Some(PathBuf::from(
                    "/Applications/LibreOffice.app/Contents/MacOS/soffice",
                )),
                "chrome" => Some(PathBuf::from(
                    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
                )),
                _ => None,
            });
        if let Some(path) = path.filter(|p| p.is_absolute() && p.is_file()) {
            paths[id] = json!(path);
        }
    }
    Ok(paths)
}
pub(crate) fn configure(data: &Path, id: &str, selected: &str) -> Result<()> {
    if ![
        "python",
        "ffmpeg",
        "ffprobe",
        "pdftotext",
        "pdftoppm",
        "tesseract",
        "whisper",
        "whisperModel",
        "libreoffice",
        "chrome",
    ]
    .contains(&id)
        || selected.len() > 4096
        || !Path::new(selected).is_absolute()
    {
        return Err(error("PRODUCTION_TOOL_PATH_INVALID"));
    }
    let path = fs::canonicalize(selected).map_err(|_| error("PRODUCTION_TOOL_PATH_INVALID"))?;
    if !path.is_file() {
        return Err(error("PRODUCTION_TOOL_PATH_INVALID"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if id != "whisperModel" && fs::metadata(&path)?.permissions().mode() & 0o111 == 0 {
            return Err(error("PRODUCTION_TOOL_NOT_EXECUTABLE"));
        }
    }
    if id == "whisperModel"
        && (fs::metadata(&path)?.len() < 4 || fs::metadata(&path)?.len() > 8 * 1024 * 1024 * 1024)
    {
        return Err(error("PRODUCTION_MODEL_INVALID"));
    }
    let mut value = configured(data)?;
    value[id] = json!(path);
    store::atomic_write(
        &data.join("production-tools.json"),
        &serde_json::to_vec(&value)?,
    )
}
pub(crate) async fn readiness(data: &Path) -> Result<Value> {
    let paths = paths(data)?;
    let mut tasks = tokio::task::JoinSet::new();
    for (index, (id, name, setup)) in [
        (
            "python",
            "Python 3.9+ · Office / data",
            "https://www.python.org/downloads/",
        ),
        (
            "ffmpeg",
            "FFmpeg · media transforms",
            "https://ffmpeg.org/download.html",
        ),
        (
            "ffprobe",
            "FFprobe · media verification",
            "https://ffmpeg.org/download.html",
        ),
        (
            "pdftotext",
            "Poppler · PDF text",
            "https://poppler.freedesktop.org/",
        ),
        (
            "pdftoppm",
            "Poppler · page previews",
            "https://poppler.freedesktop.org/",
        ),
        (
            "tesseract",
            "Tesseract · image text",
            "https://tesseract-ocr.github.io/tessdoc/Installation.html",
        ),
        (
            "whisper",
            "whisper.cpp · transcription",
            "https://github.com/ggml-org/whisper.cpp",
        ),
        (
            "whisperModel",
            "whisper.cpp · local model file",
            "https://github.com/ggml-org/whisper.cpp/tree/master/models",
        ),
        (
            "libreoffice",
            "LibreOffice · document rendering",
            "https://www.libreoffice.org/download/",
        ),
        (
            "chrome",
            "Chrome · isolated browser work",
            "https://www.google.com/chrome/",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let path = paths[id].as_str().map(str::to_owned);
        tasks.spawn(async move {
            let available = if let Some(path) = path {
                if id == "whisperModel" {
                    fs::metadata(path).is_ok_and(|m| m.len() >= 4 && m.len() <= 8 * 1024 * 1024 * 1024)
                } else {
                    let mut command = tokio::process::Command::new(path);
                    match id {
                        "python" => { command.args(["-I", "-B", "-c", "import sys; sys.exit(0 if sys.version_info >= (3,9) else 1)"]); }
                        "pdftotext" | "pdftoppm" => { command.arg("-v"); }
                        "whisper" => { command.arg("--help"); }
                        "libreoffice" | "tesseract" | "chrome" => { command.arg("--version"); }
                        _ => { command.arg("-version"); }
                    }
                    crate::providers::typed_process(command, Vec::new(), Arc::new(AtomicBool::new(true)), Duration::from_secs(2), 64 * 1024).await.is_ok()
                }
            } else { false };
            (index, json!({"id":id,"name":name,"available":available,"reasonCode":if available {"PRODUCTION_TOOL_AVAILABLE"} else {"PRODUCTION_TOOL_SETUP_REQUIRED"},"setupUrl":setup}))
        });
    }
    let mut result = Vec::new();
    while let Some(entry) = tasks.join_next().await {
        result.push(entry.map_err(|_| error("PRODUCTION_TOOL_FAILED"))?);
    }
    result.sort_by_key(|entry| entry.0);
    let mut result: Vec<Value> = result.into_iter().map(|entry| entry.1).collect();
    let available = |id: &str| {
        result
            .iter()
            .any(|entry| entry["id"] == id && entry["available"] == true)
    };
    let office = available("libreoffice") && available("pdftoppm");
    let ffmpeg = available("ffmpeg");
    let mut libass = false;
    let mut overlay = false;
    if ffmpeg {
        let mut command = tokio::process::Command::new(paths["ffmpeg"].as_str().unwrap());
        command.args(["-hide_banner", "-filters"]);
        if let Ok(bytes) = crate::providers::typed_process(
            command,
            vec![],
            Arc::new(AtomicBool::new(true)),
            Duration::from_secs(2),
            128 * 1024,
        )
        .await
        {
            let text = String::from_utf8_lossy(&bytes);
            let filters: Vec<_> = text
                .lines()
                .filter_map(|line| line.split_whitespace().nth(1))
                .collect();
            libass = filters.contains(&"subtitles");
            overlay = filters.contains(&"overlay");
        }
    }
    for entry in &mut result {
        if entry["id"] == "ffmpeg" {
            entry["burnedCaptionsAvailable"] = json!(ffmpeg && (libass || (office && overlay)));
            entry["captionRenderer"] = json!(if libass {
                Some("libass")
            } else if ffmpeg && office && overlay {
                Some("office_overlay")
            } else {
                None
            });
            entry["captionFallbackLimits"] =
                json!({"maxCaptions":20,"maxCharacters":120,"maxLines":3});
        }
        if entry["id"] == "whisperModel" {
            entry["modelValidation"] = json!("file_presence_only");
        }
    }
    Ok(json!(result))
}

pub(crate) fn input_paths(arguments: &Value) -> Result<Vec<String>> {
    fn visit(value: &Value, output: &mut Vec<String>, depth: usize) -> Result<()> {
        if depth > 8 {
            return Err(error("PRODUCTION_INVALID_INPUT"));
        }
        if let Some(object) = value.as_object() {
            for (key, child) in object {
                if key == "source" {
                    let path = child
                        .as_str()
                        .ok_or_else(|| error("PRODUCTION_INVALID_INPUT"))?;
                    if let Some(value) = path.strip_prefix("resource:") {
                        uuid::Uuid::parse_str(value)
                            .map_err(|_| error("PRODUCTION_INPUT_PATH_INVALID"))?;
                    } else if let Some(value) = path.strip_prefix("delivery:") {
                        let (offer, index) = value
                            .split_once('/')
                            .ok_or_else(|| error("PRODUCTION_INPUT_PATH_INVALID"))?;
                        uuid::Uuid::parse_str(offer)
                            .map_err(|_| error("PRODUCTION_INPUT_PATH_INVALID"))?;
                        if index
                            .parse::<usize>()
                            .ok()
                            .is_none_or(|n| n >= 8 || n.to_string() != index)
                        {
                            return Err(error("PRODUCTION_INPUT_PATH_INVALID"));
                        }
                    } else {
                        crate::creation::relative_result(path).or_else(|_| {
                            if let Some(rest) = path.strip_prefix("inputs/") {
                                crate::creation::relative_result(&format!("outputs/{rest}"))
                            } else {
                                Err(error("PRODUCTION_INPUT_PATH_INVALID"))
                            }
                        })?;
                    }
                    if !output.iter().any(|p| p == path) {
                        output.push(path.to_owned());
                    }
                } else {
                    visit(child, output, depth + 1)?;
                }
            }
        } else if let Some(array) = value.as_array() {
            for child in array {
                visit(child, output, depth + 1)?;
            }
        }
        Ok(())
    }
    if serde_json::to_vec(arguments)?.len() > 256 * 1024 {
        return Err(error("PRODUCTION_INPUT_LIMIT"));
    }
    let mut result = Vec::new();
    visit(arguments, &mut result, 0)?;
    if result.len() > 12 {
        return Err(error("PRODUCTION_INPUT_LIMIT"));
    }
    Ok(result)
}

pub(crate) struct Output {
    pub value: Value,
    pub files: Vec<(String, Vec<u8>)>,
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn remap_sources(value: &mut Value, mapping: &[(String, String)]) {
    if let Some(object) = value.as_object_mut() {
        for (key, value) in object {
            if key == "source" {
                if let Some((_, target)) = mapping.iter().find(|(name, _)| value == name) {
                    *value = json!(target);
                }
            } else {
                remap_sources(value, mapping);
            }
        }
    } else if let Some(array) = value.as_array_mut() {
        for value in array {
            remap_sources(value, mapping);
        }
    }
}

pub(crate) async fn run_for_project(
    data: &Path,
    project: &str,
    operation: &str,
    arguments: &Value,
    inputs: &[(String, Vec<u8>)],
    active: Option<Arc<AtomicBool>>,
) -> Result<Output> {
    if matches!(operation, "images.generate" | "images.edit") {
        input_paths(arguments)?;
        if inputs.len() > 12 || inputs.iter().map(|(_, b)| b.len()).sum::<usize>() > LIMIT {
            return Err(error("PRODUCTION_INPUT_LIMIT"));
        }
        return image_engine::run(data, project, operation, arguments, inputs, active).await;
    }
    run(data, operation, arguments, inputs, active).await
}

pub(crate) async fn run(
    data: &Path,
    operation: &str,
    arguments: &Value,
    inputs: &[(String, Vec<u8>)],
    active: Option<Arc<AtomicBool>>,
) -> Result<Output> {
    if matches!(operation, "browser.inspect" | "browser.draft") {
        return crate::browser::run(data, operation, arguments, inputs, active).await;
    }
    if matches!(operation, "images.generate" | "images.edit") {
        return Err(error("PRODUCTION_IMAGE_PROJECT_REQUIRED"));
    }
    if !OPERATIONS.contains(&operation) && !["inspect", "images.verify"].contains(&operation) {
        return Err(error("PRODUCTION_OPERATION_UNSUPPORTED"));
    }
    if inputs.len() > 12 || inputs.iter().map(|(_, bytes)| bytes.len()).sum::<usize>() > LIMIT {
        return Err(error("PRODUCTION_INPUT_LIMIT"));
    }
    let tools = paths(data)?;
    let python = tools["python"]
        .as_str()
        .ok_or_else(|| error("PRODUCTION_PYTHON_REQUIRED"))?;
    let parent = fs::canonicalize(data)?.join("production-scratch");
    store::private_directory(&parent)?;
    let scratch = Scratch(parent.join(uuid::Uuid::new_v4().to_string()));
    store::private_directory(&scratch.0)?;
    store::private_directory(&scratch.0.join("inputs"))?;
    let mut args = arguments.clone();
    let mut mapping: Vec<(String, String)> = Vec::new();
    for (index, (name, bytes)) in inputs.iter().enumerate() {
        if mapping.iter().any(|(original, _)| original == name) {
            return Err(error("PRODUCTION_INPUT_PATH_INVALID"));
        }
        let extension = Path::new(name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        if extension.len() > 12 || !extension.bytes().all(|c| c.is_ascii_alphanumeric()) {
            return Err(error("PRODUCTION_INPUT_PATH_INVALID"));
        }
        let target = format!("inputs/input-{index}.{extension}");
        store::atomic_write(&scratch.0.join(&target), bytes)?;
        mapping.push((name.clone(), target));
    }
    remap_sources(&mut args, &mapping);
    store::atomic_write(&scratch.0.join("production.py"), SCRIPT.as_bytes())?;
    store::atomic_write(
        &scratch.0.join("production_capabilities.py"),
        CAPABILITIES.as_bytes(),
    )?;
    let request =
        serde_json::to_vec(&json!({"operation":operation,"arguments":args,"tools":tools}))?;
    let mut command = tokio::process::Command::new(python);
    command
        .args(["-I", "-B", "production.py"])
        .current_dir(&scratch.0)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let active = active.unwrap_or_else(|| Arc::new(AtomicBool::new(true)));
    let bytes = crate::providers::typed_process(
        command,
        request,
        active,
        Duration::from_secs(120),
        2 * 1024 * 1024,
    )
    .await
    .map_err(|code| {
        error(match code.as_str() {
            "EXTRACTION_CANCELLED" => "PRODUCTION_CANCELLED",
            "EXTRACTION_TIMEOUT" => "PRODUCTION_TIMEOUT",
            "EXTRACTION_INVALID_OUTPUT" => "PRODUCTION_OUTPUT_LIMIT",
            "EXTRACTION_PROVIDER_UNAVAILABLE" => "PRODUCTION_TOOL_UNAVAILABLE",
            _ => "PRODUCTION_TOOL_FAILED",
        })
    })?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| error("PRODUCTION_OUTPUT_INVALID"))?;
    if value["error"].is_string() {
        let code = value["error"]
            .as_str()
            .filter(|s| {
                s.starts_with("PRODUCTION_")
                    && s.len() < 64
                    && s.bytes().all(|c| c.is_ascii_uppercase() || c == b'_')
            })
            .unwrap_or("PRODUCTION_TOOL_FAILED");
        return Err(error(code));
    }
    let mut files = Vec::new();
    let mut total = 0;
    for entry in fs::read_dir(scratch.0.join("outputs"))? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let bytes = crate::conversation::copied_bytes(
            &scratch.0,
            &PathBuf::from("outputs").join(&name),
            (LIMIT - total) as u64,
        )?;
        total += bytes.len();
        files.push((name, bytes));
        if files.len() > 16 {
            return Err(error("PRODUCTION_OUTPUT_LIMIT"));
        }
    }
    Ok(Output { value, files })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn delivered_and_frozen_sources_keep_distinct_bytes_when_aliases_overlap() {
        let delivered = "delivery:550e8400-e29b-41d4-a716-446655440000/0";
        let frozen = "inputs/reference-01.txt";
        let mut args = json!({"clips":[{"source":delivered},{"source":frozen}]});
        remap_sources(
            &mut args,
            &[
                (delivered.into(), frozen.into()),
                (frozen.into(), "inputs/reference-02.txt".into()),
            ],
        );
        assert_eq!(args["clips"][0]["source"], frozen);
        assert_eq!(args["clips"][1]["source"], "inputs/reference-02.txt");
        let data = tempfile::tempdir().unwrap();
        let inputs = vec![
            (frozen.into(), b"Delivered source".to_vec()),
            ("inputs/input-0.txt".into(), b"Frozen source".to_vec()),
        ];
        for (source, expected) in &inputs {
            let output = run(
                data.path(),
                "resources.extract",
                &json!({"source":source}),
                &inputs,
                None,
            )
            .await
            .unwrap();
            assert!(output.files.iter().any(|(_, bytes)| bytes == expected));
        }
    }
    #[tokio::test]
    async fn office_files_are_real_and_targeted_revision_keeps_other_pages() {
        let data = tempfile::tempdir().unwrap();
        let first=run(data.path(),"presentations.create",&json!({"slides":[{"title":"First","bullets":["Keep me"]},{"title":"Second","bullets":["Change me"]}]}),&[],None).await.unwrap();
        let pptx = first
            .files
            .iter()
            .find(|(n, _)| n.ends_with(".pptx"))
            .unwrap();
        assert_eq!(&pptx.1[..4], b"PK\x03\x04");
        let revised=run(data.path(),"presentations.revise",&json!({"source":"inputs/deck.pptx","sectionId":"slide:2","title":"Updated","bullets":["Changed"]}),&[("inputs/deck.pptx".into(),pptx.1.clone())],None).await.unwrap();
        let after = revised
            .files
            .iter()
            .find(|(n, _)| n.ends_with(".pptx"))
            .unwrap();
        let view = run(
            data.path(),
            "inspect",
            &json!({"source":"inputs/deck.pptx"}),
            &[("inputs/deck.pptx".into(), after.1.clone())],
            None,
        )
        .await
        .unwrap();
        assert_eq!(view.value["pages"][0]["title"], "First");
        assert_eq!(view.value["pages"][0]["text"], "Keep me");
        assert_eq!(view.value["pages"][1]["title"], "Updated");
        let document = run(
            data.path(),
            "documents.create",
            &json!({"paragraphs":["First paragraph","Second paragraph"]}),
            &[],
            None,
        )
        .await
        .unwrap();
        assert!(document
            .files
            .iter()
            .any(|(n, b)| n.ends_with(".docx") && b.starts_with(b"PK")));
    }
    #[tokio::test]
    async fn csv_analysis_is_deterministic_and_sources_are_strict() {
        let data = tempfile::tempdir().unwrap();
        let output = run(
            data.path(),
            "data.analyze",
            &json!({"source":"inputs/source.csv","deduplicate":true,"numericColumns":["amount"]}),
            &[(
                "inputs/source.csv".into(),
                b"name,amount\n a ,2\nb,4\nb,4\n".to_vec(),
            )],
            None,
        )
        .await
        .unwrap();
        let stats: Value = serde_json::from_slice(
            &output
                .files
                .iter()
                .find(|(n, _)| n == "analysis.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(stats["columns"]["amount"]["sum"], 6.0);
        assert_eq!(stats["outputRows"], 2);
        assert!(input_paths(&json!({"source":"inputs/../../secret"})).is_err());
        assert!(run(data.path(), "shell.exec", &json!({}), &[], None)
            .await
            .is_err());
    }
    #[tokio::test]
    #[ignore = "requires explicitly provided MOXCREW_MEDIA_TEST_FFPROBE and FFmpeg executables"]
    async fn media_input_cannot_autodetect_a_network_playlist() {
        let directory = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("ffprobe", "MOXCREW_MEDIA_TEST_FFPROBE"),
            ("ffmpeg", "MOXCREW_MEDIA_TEST_FFMPEG"),
        ] {
            configure(
                directory.path(),
                id,
                &std::env::var(key).expect("explicit test tool path"),
            )
            .unwrap();
        }
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let playlist=format!("#EXTM3U\n#EXT-X-TARGETDURATION:2\n#EXTINF:2,\nhttp://{address}/private.ts\n#EXT-X-ENDLIST\n");
        let result = run(
            directory.path(),
            "inspect",
            &json!({"source":"inputs/disguised.mp4"}),
            &[("inputs/disguised.mp4".into(), playlist.into_bytes())],
            None,
        )
        .await;
        assert!(matches!(result,Err(Error(ref code)) if code=="PRODUCTION_TOOL_FAILED"));
        assert!(
            tokio::time::timeout(Duration::from_millis(200), listener.accept())
                .await
                .is_err()
        );
    }
}

#[cfg(all(test, unix))]
#[path = "production_capabilities_tests.rs"]
mod capabilities_tests;
