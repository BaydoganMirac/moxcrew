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
    "presentations.brand",
    "presentations.narrate",
    "documents.create",
    "documents.revise",
    "documents.brief",
    "documents.rewrite",
    "documents.template",
    "research.claims",
    "research.compare",
    "research.synthesis",
    "research.changes",
    "data.clean",
    "data.analyze",
    "data.xlsx",
    "data.chart",
    "data.scenario",
    "data.extract",
    "images.resize",
    "images.batch",
    "images.mask",
    "video.edit",
    "audio.cleanup",
    "audio.podcast",
    "resources.extract",
    "resources.ocr",
    "pdf.extract",
    "pdf.merge",
    "audio.transcribe",
    "documents.render",
    "video.frame",
    "video.package",
    "video.highlights",
    "video.compose",
    "video.motion",
    "interactive.create",
    "web.site",
    "images.generate",
    "images.edit",
    "images.campaign",
    "browser.inspect",
    "browser.draft",
    "browser.research",
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
        ("pdfinfo", "pdfinfo"),
        ("pdfunite", "pdfunite"),
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
        "pdfinfo",
        "pdfunite",
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
            "pdfinfo",
            "Poppler · PDF structure",
            "https://poppler.freedesktop.org/",
        ),
        (
            "pdfunite",
            "Poppler · ordered PDF merge",
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
                        "pdftotext" | "pdftoppm" | "pdfinfo" | "pdfunite" => { command.arg("-v"); }
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
    if matches!(
        operation,
        "browser.inspect" | "browser.draft" | "browser.research" | "images.campaign"
    ) {
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
    async fn branded_presentations_keep_editable_layouts_notes_and_exact_revisions() {
        let data = tempfile::tempdir().unwrap();
        let first=run(data.path(),"presentations.create",&json!({
            "brand":{"name":"Northstar","primary":"2648E8","accent":"F06A2A","background":"FAF8F2","text":"17213A","muted":"526079","font":"Arial"},
            "slides":[
                {"layout":"cover","eyebrow":"Launch brief","title":"A clearer way to run ambitious work","subtitle":"One local project, one inspectable crew","notes":"Open with the owner problem and keep the promise concrete."},
                {"layout":"content","title":"What owners need","body":"The work should stay inspectable from request to delivery.","bullets":["Exact sources and versions","Visible preparation before execution","Reviewable output evidence"]},
                {"layout":"bar-chart","title":"Pilot teams complete more work","chart":{"unit":"%","sourceLabel":"Source: checked pilot dataset","series":[{"label":"Before","value":42},{"label":"With Mox","value":78}]},"notes":"State the sample limits before discussing the difference."}
            ]
        }),&[],None).await.unwrap();
        let pptx = first
            .files
            .iter()
            .find(|(n, _)| n.ends_with(".pptx"))
            .unwrap();
        assert_eq!(&pptx.1[..4], b"PK\x03\x04");
        let created_manifest: Value = serde_json::from_slice(
            &first
                .files
                .iter()
                .find(|(name, _)| name == "presentation-manifest.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(created_manifest["slideCount"], 3);
        assert_eq!(created_manifest["slides"][2]["layout"], "bar-chart");
        assert_eq!(
            created_manifest["slides"][2]["chart"]["series"][1]["value"],
            78
        );
        assert_eq!(created_manifest["slides"][0]["notes"], true);
        let revised = run(
            data.path(),
            "presentations.revise",
            &json!({
                "source":"inputs/deck.pptx","sectionId":"slide:2","layout":"statement",
                "eyebrow":"Owner control","title":"The plan stays visible before work starts",
                "subtitle":"A deliberate handoff replaces hidden execution."
            }),
            &[("inputs/deck.pptx".into(), pptx.1.clone())],
            None,
        )
        .await
        .unwrap();
        let after = revised
            .files
            .iter()
            .find(|(n, _)| n.ends_with(".pptx"))
            .unwrap();
        let revised_manifest: Value = serde_json::from_slice(
            &revised
                .files
                .iter()
                .find(|(name, _)| name == "presentation-manifest.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(revised_manifest["targetSlide"], 2);
        assert_eq!(
            revised_manifest["verification"]["nonTargetSlidesPreserved"],
            2
        );
        let view = run(
            data.path(),
            "inspect",
            &json!({"source":"inputs/deck.pptx"}),
            &[("inputs/deck.pptx".into(), after.1.clone())],
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            view.value["pages"][0]["title"],
            "A clearer way to run ambitious work"
        );
        assert_eq!(
            view.value["pages"][1]["title"],
            "The plan stays visible before work starts"
        );
        assert!(view.value["pages"][2]["text"]
            .as_str()
            .unwrap()
            .contains("With Mox"));
        let branded=run(data.path(),"presentations.brand",&json!({
            "source":"inputs/revised.pptx",
            "brand":{"name":"Night Shift","primary":"9BAEFF","accent":"FF9B66","background":"090D18","text":"F2F5FF","muted":"A7B3CC","font":"Arial"}
        }),&[("inputs/revised.pptx".into(),after.1.clone())],None).await.unwrap();
        let brand_manifest: Value = serde_json::from_slice(
            &branded
                .files
                .iter()
                .find(|(name, _)| name == "presentation-manifest.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(brand_manifest["operation"], "brand");
        assert_eq!(brand_manifest["theme"]["background"], "090D18");
        assert!(
            brand_manifest["coverage"]["schemeAwareElements"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(
            brand_manifest["coverage"]["partialForExplicitColors"],
            false
        );
        assert!(
            brand_manifest["verification"]["slideNotesAndMediaPartsPreserved"]
                .as_u64()
                .unwrap()
                >= 6
        );
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
    async fn presentation_density_and_contrast_fail_before_publication() {
        let data = tempfile::tempdir().unwrap();
        let low_contrast = run(
            data.path(),
            "presentations.create",
            &json!({
                "brand":{"background":"FFFFFF","text":"F8F8F8"},
                "slides":[{"layout":"cover","title":"Unreadable"}]
            }),
            &[],
            None,
        )
        .await;
        assert!(
            matches!(low_contrast,Err(Error(ref code)) if code=="PRODUCTION_PRESENTATION_CONTRAST")
        );
        let crowded = run(
            data.path(),
            "presentations.create",
            &json!({"slides":[{"layout":"content","title":"Crowded","bullets":["word ".repeat(220)]}]}),
            &[],
            None,
        )
        .await;
        assert!(
            matches!(crowded,Err(Error(ref code)) if code=="PRODUCTION_INVALID_INPUT" || code=="PRODUCTION_PRESENTATION_TEXT_OVERFLOW")
        );
    }
    #[tokio::test]
    async fn narrated_presentations_require_exact_order_and_notes_before_encoding() {
        let data = tempfile::tempdir().unwrap();
        let noted = run(
            data.path(),
            "presentations.create",
            &json!({"slides":[
                {"layout":"cover","title":"Opening","notes":"Approved opening narration."},
                {"layout":"statement","title":"Close","notes":"Approved closing narration."}
            ]}),
            &[],
            None,
        )
        .await
        .unwrap();
        let noted = noted
            .files
            .iter()
            .find(|(name, _)| name == "presentation.pptx")
            .unwrap()
            .1
            .clone();
        let count = run(
            data.path(),
            "presentations.narrate",
            &json!({"source":"inputs/deck.pptx","segments":[
                {"slide":1,"source":"inputs/one.m4a"}
            ]}),
            &[("inputs/deck.pptx".into(), noted.clone())],
            None,
        )
        .await;
        assert!(
            matches!(count,Err(Error(ref code)) if code=="PRODUCTION_PRESENTATION_NARRATION_COUNT")
        );
        let order = run(
            data.path(),
            "presentations.narrate",
            &json!({"source":"inputs/deck.pptx","segments":[
                {"slide":2,"source":"inputs/two.m4a"},
                {"slide":1,"source":"inputs/one.m4a"}
            ]}),
            &[("inputs/deck.pptx".into(), noted)],
            None,
        )
        .await;
        assert!(
            matches!(order,Err(Error(ref code)) if code=="PRODUCTION_PRESENTATION_NARRATION_ORDER")
        );
        let missing = run(
            data.path(),
            "presentations.create",
            &json!({"slides":[
                {"layout":"cover","title":"Opening","notes":"Approved opening narration."},
                {"layout":"statement","title":"Close"}
            ]}),
            &[],
            None,
        )
        .await
        .unwrap();
        let missing = missing
            .files
            .iter()
            .find(|(name, _)| name == "presentation.pptx")
            .unwrap()
            .1
            .clone();
        let notes = run(
            data.path(),
            "presentations.narrate",
            &json!({"source":"inputs/deck.pptx","segments":[
                {"slide":1,"source":"inputs/one.m4a"},
                {"slide":2,"source":"inputs/two.m4a"}
            ]}),
            &[("inputs/deck.pptx".into(), missing)],
            None,
        )
        .await;
        assert!(
            matches!(notes,Err(Error(ref code)) if code=="PRODUCTION_PRESENTATION_NOTES_REQUIRED")
        );
    }
    #[tokio::test]
    #[ignore = "requires real LibreOffice and Poppler paths plus an evidence destination"]
    async fn branded_presentation_renders_every_slide_with_real_tools() {
        let libreoffice = std::env::var("MOXCREW_TEST_LIBREOFFICE").unwrap();
        let pdftoppm = std::env::var("MOXCREW_TEST_PDFTOPPM").unwrap();
        let logo = std::env::var("MOXCREW_PRESENTATION_TEST_LOGO").unwrap();
        let evidence = PathBuf::from(std::env::var("MOXCREW_PRESENTATION_EVIDENCE_DIR").unwrap());
        let data = tempfile::tempdir().unwrap();
        configure(data.path(), "libreoffice", &libreoffice).unwrap();
        configure(data.path(), "pdftoppm", &pdftoppm).unwrap();
        let logo_bytes = fs::read(logo).unwrap();
        let output = run(
            data.path(),
            "presentations.create",
            &json!({
                "brand":{"name":"Moxcrew","primary":"2854F4","accent":"FF7A2F","background":"F8F7F3","text":"15223D","muted":"56627A","font":"Arial","logo":{"source":"inputs/mox-master.png","alt":"Moxcrew mascot"}},
                "slides":[
                    {"layout":"cover","eyebrow":"Local-first AI crew","title":"Build ambitious work without losing control","subtitle":"One inspectable project. One visible plan. Every output reviewable.","image":{"source":"inputs/mox-master.png","alt":"Moxcrew mascot","fit":"contain"},"notes":"Open with the owner promise: powerful work without hidden execution."},
                    {"layout":"statement","eyebrow":"The operating principle","title":"Nothing moves before the owner can see what will happen.","subtitle":"Preparation stays separate from execution, and every handoff leaves evidence."},
                    {"layout":"split","title":"A crew that stays legible","body":"Moxcrew turns a complex request into an owner-visible production loop.","bullets":["Ground work in exact project sources","Prepare actions before execution","Keep versions, notes and evidence together"],"image":{"source":"inputs/mox-master.png","alt":"Moxcrew mascot","fit":"contain"}},
                    {"layout":"bar-chart","title":"Pilot teams finish more of the work they start","chart":{"unit":"%","sourceLabel":"Illustrative pilot dataset · owner review required","series":[{"label":"Before","value":42},{"label":"With Moxcrew","value":78}]},"notes":"Call out that the values are illustrative and require owner review."}
                ],
                "render":true
            }),
            &[("inputs/mox-master.png".into(), logo_bytes)],
            None,
        )
        .await
        .unwrap();
        fs::create_dir_all(&evidence).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "presentation.pptx",
            "presentation-manifest.json",
            "slides.pdf",
            "slide-01.png",
            "slide-02.png",
            "slide-03.png",
            "slide-04.png",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
    }
    #[tokio::test]
    #[ignore = "requires real media/render tools, local narration clips and an evidence destination"]
    async fn noted_presentation_and_real_audio_render_a_checked_video() {
        let data = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("libreoffice", "MOXCREW_TEST_LIBREOFFICE"),
            ("pdftoppm", "MOXCREW_TEST_PDFTOPPM"),
            ("ffmpeg", "MOXCREW_TEST_FFMPEG"),
            ("ffprobe", "MOXCREW_TEST_FFPROBE"),
        ] {
            configure(data.path(), id, &std::env::var(key).unwrap()).unwrap();
        }
        let audio =
            PathBuf::from(std::env::var("MOXCREW_PRESENTATION_NARRATION_AUDIO_DIR").unwrap());
        let evidence =
            PathBuf::from(std::env::var("MOXCREW_PRESENTATION_NARRATION_EVIDENCE_DIR").unwrap());
        let created = run(
            data.path(),
            "presentations.create",
            &json!({
                "brand":{"name":"Moxcrew","primary":"2854F4","accent":"FF7A2F","background":"F8F7F3","text":"15223D","muted":"56627A","font":"Arial"},
                "slides":[
                    {"layout":"cover","eyebrow":"Narrated brief","title":"A crew you can see and hear","subtitle":"Every slide stays timed, sourced and reviewable.","notes":"Moxcrew turns exact speaker notes and approved audio into a reviewable presentation video."},
                    {"layout":"statement","eyebrow":"Owner control","title":"The deck remains the source of truth.","subtitle":"Narration timing is checked without rewriting the presentation.","notes":"The original deck stays unchanged while each narration clip is matched to its exact slide."},
                    {"layout":"bar-chart","title":"Every narration segment is accounted for","chart":{"unit":"%","sourceLabel":"Deterministic presentation-video contract","series":[{"label":"Untracked","value":0},{"label":"Checked","value":100}]},"notes":"Moxcrew renders every slide, probes every audio clip, decodes the full video, and reports what still needs human review."}
                ]
            }),
            &[],
            None,
        )
        .await
        .unwrap();
        let deck = created
            .files
            .iter()
            .find(|(name, _)| name == "presentation.pptx")
            .unwrap()
            .1
            .clone();
        let audio_files = (1..=3)
            .map(|index| fs::read(audio.join(format!("narration-{index:02}.wav"))).unwrap())
            .collect::<Vec<_>>();
        let output = run(
            data.path(),
            "presentations.narrate",
            &json!({"source":"inputs/deck.pptx","segments":[
                {"slide":1,"source":"inputs/narration-01.wav"},
                {"slide":2,"source":"inputs/narration-02.wav"},
                {"slide":3,"source":"inputs/narration-03.wav"}
            ],"fps":30}),
            &[
                ("inputs/deck.pptx".into(), deck),
                ("inputs/narration-01.wav".into(), audio_files[0].clone()),
                ("inputs/narration-02.wav".into(), audio_files[1].clone()),
                ("inputs/narration-03.wav".into(), audio_files[2].clone()),
            ],
            None,
        )
        .await
        .unwrap();
        fs::create_dir_all(&evidence).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "presentation-video.mp4",
            "presentation-video-manifest.json",
            "slides.pdf",
            "slide-01.png",
            "slide-02.png",
            "slide-03.png",
            "video-frame-start.png",
            "video-frame-middle.png",
            "video-frame-end.png",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn audio_cleanup_rejects_unknown_treatment_before_tools() {
        let data = tempfile::tempdir().unwrap();
        let result = run(
            data.path(),
            "audio.cleanup",
            &json!({"source":"inputs/source.wav","targetLufs":-16,"treatment":"magic"}),
            &[("inputs/source.wav".into(), b"audio fixture".to_vec())],
            None,
        )
        .await;
        assert!(
            matches!(result,Err(Error(ref code)) if code=="PRODUCTION_AUDIO_CLEANUP_TREATMENT_INVALID")
        );
        let target = run(
            data.path(),
            "audio.cleanup",
            &json!({"source":"inputs/source.wav","targetLufs":-30,"treatment":"voice"}),
            &[("inputs/source.wav".into(), b"audio fixture".to_vec())],
            None,
        )
        .await;
        assert!(
            matches!(target,Err(Error(ref code)) if code=="PRODUCTION_AUDIO_CLEANUP_TARGET_INVALID")
        );
    }
    #[tokio::test]
    #[ignore = "requires real FFmpeg/FFprobe, one exact audio input and an evidence destination"]
    async fn exact_audio_renders_a_measured_checked_cleanup() {
        let data = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("ffmpeg", "MOXCREW_TEST_FFMPEG"),
            ("ffprobe", "MOXCREW_TEST_FFPROBE"),
        ] {
            configure(data.path(), id, &std::env::var(key).unwrap()).unwrap();
        }
        let source = fs::read(std::env::var("MOXCREW_AUDIO_CLEANUP_SOURCE").unwrap()).unwrap();
        let evidence = PathBuf::from(std::env::var("MOXCREW_AUDIO_CLEANUP_EVIDENCE_DIR").unwrap());
        let output = run(
            data.path(),
            "audio.cleanup",
            &json!({"source":"inputs/source.wav","targetLufs":-16,"treatment":"voice"}),
            &[("inputs/source.wav".into(), source.clone())],
            None,
        )
        .await
        .unwrap();
        fs::create_dir_all(&evidence).unwrap();
        fs::write(evidence.join("source-fixture.wav"), source).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "source-fixture.wav",
            "cleaned-audio.wav",
            "audio-cleanup-manifest.json",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
        let manifest: Value = serde_json::from_slice(
            &fs::read(evidence.join("audio-cleanup-manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["policy"]["targetLufs"], -16);
        assert_eq!(manifest["verification"]["targetMatched"], true);
        assert_eq!(manifest["verification"]["fullAudioDecoded"], true);
        assert_eq!(manifest["verification"]["subjectiveQualityReviewed"], false);
        assert!(
            manifest["measurement"]["targetDifferenceLu"]
                .as_f64()
                .unwrap()
                <= 1.0
        );
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn motion_video_rejects_unknown_treatment_before_tools() {
        let data = tempfile::tempdir().unwrap();
        let result = run(
            data.path(),
            "video.motion",
            &json!({"title":"Visible work","scenes":[],"motion":"random", "fps":30}),
            &[],
            None,
        )
        .await;
        assert!(matches!(result,Err(Error(ref code)) if code=="PRODUCTION_VIDEO_MOTION_INVALID"));
    }
    #[tokio::test]
    #[ignore = "requires real LibreOffice, Poppler, FFmpeg/FFprobe and an evidence destination"]
    async fn approved_scenes_render_a_checked_motion_video() {
        let data = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("libreoffice", "MOXCREW_TEST_LIBREOFFICE"),
            ("pdftoppm", "MOXCREW_TEST_PDFTOPPM"),
            ("ffmpeg", "MOXCREW_TEST_FFMPEG"),
            ("ffprobe", "MOXCREW_TEST_FFPROBE"),
        ] {
            configure(data.path(), id, &std::env::var(key).unwrap()).unwrap();
        }
        let evidence = PathBuf::from(std::env::var("MOXCREW_VIDEO_MOTION_EVIDENCE_DIR").unwrap());
        let output = run(
            data.path(),
            "video.motion",
            &json!({"title":"Mox keeps work visible","motion":"drift","fps":30,"scenes":[
                {"layout":"cover","duration":3,"eyebrow":"MEET MOX","title":"A crew you can actually see","subtitle":"Plan, produce and review in one local workspace."},
                {"layout":"statement","duration":3,"eyebrow":"OWNER CONTROL","title":"Every handoff stays visible.","subtitle":"Nothing starts until you say so."},
                {"layout":"statement","duration":3,"eyebrow":"REAL OUTPUTS","title":"Files, checks and revisions—not mystery progress.","subtitle":"Each delivery keeps its evidence."},
                {"layout":"cover","duration":3,"eyebrow":"MOXCREW","title":"Ship with proof.","subtitle":"Keep the final call."}
            ]}),
            &[],
            None,
        )
        .await
        .unwrap();
        fs::create_dir_all(&evidence).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "motion-video.mp4",
            "motion-video-manifest.json",
            "motion-storyboard.pptx",
            "motion-storyboard.json",
            "slides.pdf",
            "slide-01.png",
            "slide-02.png",
            "slide-03.png",
            "slide-04.png",
            "motion-frame-start.png",
            "motion-frame-middle.png",
            "motion-frame-end.png",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(evidence.join("motion-video-manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["scenes"].as_array().unwrap().len(), 4);
        assert_eq!(manifest["verification"]["motionFramesDiffer"], true);
        assert_eq!(manifest["verification"]["fullVideoDecoded"], true);
        assert_eq!(manifest["verification"]["visualQualityReviewed"], false);
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn video_highlights_reject_unknown_emphasis_before_tools() {
        let data = tempfile::tempdir().unwrap();
        let result = run(
            data.path(),
            "video.highlights",
            &json!({"source":"inputs/source.mp4","targetSeconds":8,
                "clipSeconds":4,"emphasis":"semantic"}),
            &[],
            None,
        )
        .await;
        assert!(matches!(result,Err(Error(ref code)) if code=="PRODUCTION_VIDEO_EMPHASIS_INVALID"));
    }
    #[tokio::test]
    #[ignore = "requires real FFmpeg/FFprobe, one exact video input and an evidence destination"]
    async fn measured_signals_render_a_checked_highlight_reel() {
        let data = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("ffmpeg", "MOXCREW_TEST_FFMPEG"),
            ("ffprobe", "MOXCREW_TEST_FFPROBE"),
        ] {
            configure(data.path(), id, &std::env::var(key).unwrap()).unwrap();
        }
        let source = fs::read(std::env::var("MOXCREW_VIDEO_HIGHLIGHT_SOURCE").unwrap()).unwrap();
        let evidence =
            PathBuf::from(std::env::var("MOXCREW_VIDEO_HIGHLIGHT_EVIDENCE_DIR").unwrap());
        let output = run(
            data.path(),
            "video.highlights",
            &json!({"source":"inputs/source.mp4","targetSeconds":8,
                "clipSeconds":4,"emphasis":"balanced"}),
            &[("inputs/source.mp4".into(), source)],
            None,
        )
        .await
        .unwrap();
        fs::create_dir_all(&evidence).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "highlights.mp4",
            "highlight-manifest.json",
            "edit-plan.json",
            "validation.json",
            "highlight-frame-01.png",
            "highlight-frame-02.png",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(evidence.join("highlight-manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["selections"].as_array().unwrap().len(), 2);
        assert_eq!(manifest["verification"]["fullVideoDecoded"], true);
        assert_eq!(manifest["verification"]["humanMeaningReviewRequired"], true);
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn composed_video_rejects_unknown_placement_and_missing_video_streams() {
        use std::os::unix::fs::PermissionsExt;
        let data = tempfile::tempdir().unwrap();
        let invalid = run(
            data.path(),
            "video.compose",
            &json!({"source":"inputs/source.mp4","image":{"source":"inputs/logo.png"},
                "audio":{"source":"inputs/music.wav"},"title":"Visible title","titleSeconds":1,
                "overlay":{"start":0,"end":1,"position":"center"},"audioMode":"replace","audioLevel":0.5}),
            &[],
            None,
        )
        .await;
        assert!(
            matches!(invalid,Err(Error(ref code)) if code=="PRODUCTION_VIDEO_POSITION_INVALID")
        );

        let ffprobe = data.path().join("ffprobe-fixture");
        fs::write(
            &ffprobe,
            "#!/bin/sh\nprintf '%s' '{\"format\":{\"duration\":\"2\"},\"streams\":[]}'\n",
        )
        .unwrap();
        fs::set_permissions(&ffprobe, fs::Permissions::from_mode(0o700)).unwrap();
        configure(data.path(), "ffprobe", ffprobe.to_str().unwrap()).unwrap();
        let missing = run(
            data.path(),
            "video.compose",
            &json!({"source":"inputs/source.mp4","image":{"source":"inputs/logo.png"},
                "audio":{"source":"inputs/music.wav"},"title":"Visible title","titleSeconds":1,
                "overlay":{"start":0,"end":1,"position":"bottom-right"},"audioMode":"replace","audioLevel":0.5}),
            &[
                ("inputs/source.mp4".into(), b"video fixture".to_vec()),
                ("inputs/logo.png".into(), b"image fixture".to_vec()),
                ("inputs/music.wav".into(), b"audio fixture".to_vec()),
            ],
            None,
        )
        .await;
        assert!(matches!(missing,Err(Error(ref code)) if code=="PRODUCTION_VIDEO_REQUIRED"));
    }
    #[tokio::test]
    #[ignore = "requires real media/render tools, exact video/image/audio inputs and an evidence destination"]
    async fn exact_title_image_and_audio_render_a_checked_composed_video() {
        let data = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("libreoffice", "MOXCREW_TEST_LIBREOFFICE"),
            ("pdftoppm", "MOXCREW_TEST_PDFTOPPM"),
            ("ffmpeg", "MOXCREW_TEST_FFMPEG"),
            ("ffprobe", "MOXCREW_TEST_FFPROBE"),
        ] {
            configure(data.path(), id, &std::env::var(key).unwrap()).unwrap();
        }
        let source = fs::read(std::env::var("MOXCREW_VIDEO_COMPOSE_SOURCE").unwrap()).unwrap();
        let image = fs::read(std::env::var("MOXCREW_VIDEO_COMPOSE_IMAGE").unwrap()).unwrap();
        let audio = fs::read(std::env::var("MOXCREW_VIDEO_COMPOSE_AUDIO").unwrap()).unwrap();
        let evidence = PathBuf::from(std::env::var("MOXCREW_VIDEO_COMPOSE_EVIDENCE_DIR").unwrap());
        let output = run(
            data.path(),
            "video.compose",
            &json!({"source":"inputs/source.mp4","image":{"source":"inputs/mox.png"},
                "audio":{"source":"inputs/music.wav"},"title":"Every handoff stays visible",
                "titleSeconds":2.5,"overlay":{"start":3,"end":12,"position":"bottom-right"},
                "audioMode":"replace","audioLevel":0.8}),
            &[
                ("inputs/source.mp4".into(), source.clone()),
                ("inputs/mox.png".into(), image.clone()),
                ("inputs/music.wav".into(), audio.clone()),
            ],
            None,
        )
        .await
        .unwrap();
        fs::create_dir_all(&evidence).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "composed-video.mp4",
            "composition-manifest.json",
            "video-frame-start.png",
            "video-frame-middle.png",
            "video-frame-overlay.png",
            "video-frame-end.png",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
        let mixed = run(
            data.path(),
            "video.compose",
            &json!({"source":"inputs/source.mp4","image":{"source":"inputs/mox.png"},
                "audio":{"source":"inputs/music.wav"},"title":"Every handoff stays visible",
                "titleSeconds":2.5,"overlay":{"start":3,"end":12,"position":"bottom-right"},
                "audioMode":"mix","audioLevel":0.25}),
            &[
                ("inputs/source.mp4".into(), source),
                ("inputs/mox.png".into(), image),
                ("inputs/music.wav".into(), audio),
            ],
            None,
        )
        .await
        .unwrap();
        let manifest: Value = serde_json::from_slice(
            &mixed
                .files
                .iter()
                .find(|(name, _)| name == "composition-manifest.json")
                .unwrap()
                .1,
        )
        .unwrap();
        assert_eq!(manifest["audio"]["mode"], "mix");
        assert_eq!(manifest["verification"]["fullVideoDecoded"], true);
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
        assert_eq!(stats["trimmedCells"], 1);
        assert_eq!(stats["verification"]["exactValuesMatched"], true);
        let workbook = run(
            data.path(),
            "data.xlsx",
            &json!({"source":"inputs/source.csv","deduplicate":true,"numericColumns":["amount"]}),
            &[(
                "inputs/source.csv".into(),
                b"name,amount\n a ,2\nb,4\nb,4\n".to_vec(),
            )],
            None,
        )
        .await
        .unwrap();
        assert!(workbook.files.iter().any(|(name, _)| name == "data.xlsx"));
        let duplicate_headers = run(
            data.path(),
            "data.analyze",
            &json!({"source":"inputs/source.csv"}),
            &[("inputs/source.csv".into(), b"name,name\na,b\n".to_vec())],
            None,
        )
        .await;
        assert!(
            matches!(duplicate_headers,Err(Error(ref code)) if code=="PRODUCTION_DATA_HEADER_INVALID")
        );
        let unknown_column = run(
            data.path(),
            "data.analyze",
            &json!({"source":"inputs/source.csv","numericColumns":["missing"]}),
            &[("inputs/source.csv".into(), b"name,amount\na,2\n".to_vec())],
            None,
        )
        .await;
        assert!(
            matches!(unknown_column,Err(Error(ref code)) if code=="PRODUCTION_DATA_COLUMN_NOT_FOUND")
        );
        assert!(input_paths(&json!({"source":"inputs/../../secret"})).is_err());
        assert!(run(data.path(), "shell.exec", &json!({}), &[], None)
            .await
            .is_err());
    }
    #[tokio::test]
    #[ignore = "writes retained checked CSV/XLSX evidence to an explicit destination"]
    async fn exact_csv_produces_checked_cleaned_and_profiled_outputs() {
        let data = tempfile::tempdir().unwrap();
        let source =
            b"region,revenue,units\n North ,42000,12\nSouth,,9\nSouth,,9\nWest,38750,11\n".to_vec();
        let arguments = json!({"source":"inputs/source.csv","deduplicate":true,
            "numericColumns":["revenue","units"]});
        let csv = run(
            data.path(),
            "data.analyze",
            &arguments,
            &[("inputs/source.csv".into(), source.clone())],
            None,
        )
        .await
        .unwrap();
        let xlsx = run(
            data.path(),
            "data.xlsx",
            &arguments,
            &[("inputs/source.csv".into(), source.clone())],
            None,
        )
        .await
        .unwrap();
        let evidence = PathBuf::from(std::env::var("MOXCREW_DATA_PROFILE_EVIDENCE_DIR").unwrap());
        fs::create_dir_all(&evidence).unwrap();
        fs::write(evidence.join("source.csv"), source).unwrap();
        for (output, names) in [
            (
                &csv,
                [
                    ("data.csv", "cleaned-data.csv"),
                    ("analysis.json", "analysis.json"),
                ],
            ),
            (
                &xlsx,
                [
                    ("data.xlsx", "cleaned-data.xlsx"),
                    ("analysis.json", "xlsx-analysis.json"),
                ],
            ),
        ] {
            for (source_name, target_name) in names {
                let bytes = &output
                    .files
                    .iter()
                    .find(|(name, _)| name == source_name)
                    .unwrap()
                    .1;
                fs::write(evidence.join(target_name), bytes).unwrap();
            }
        }
        let analysis: Value =
            serde_json::from_slice(&fs::read(evidence.join("analysis.json")).unwrap()).unwrap();
        assert_eq!(analysis["inputRows"], 4);
        assert_eq!(analysis["outputRows"], 3);
        assert_eq!(analysis["removedDuplicates"], 1);
        assert_eq!(analysis["missingByColumn"]["revenue"], 1);
        assert_eq!(analysis["columns"]["revenue"]["sum"], 80750.0);
        assert_eq!(analysis["verification"]["outputReopened"], true);
    }
    #[cfg(unix)]
    #[tokio::test]
    #[ignore = "requires real FFmpeg/FFprobe, one exact audio input and an evidence destination"]
    async fn exact_audio_and_whisper_peer_produce_checked_transcription_outputs() {
        use std::os::unix::fs::PermissionsExt;

        let data = tempfile::tempdir().unwrap();
        for (id, key) in [
            ("ffmpeg", "MOXCREW_TEST_FFMPEG"),
            ("ffprobe", "MOXCREW_TEST_FFPROBE"),
        ] {
            configure(data.path(), id, &std::env::var(key).unwrap()).unwrap();
        }
        let whisper = data.path().join("fixture-whisper");
        fs::write(
            &whisper,
            r#"#!/usr/bin/env python3
import json,pathlib,sys
args=sys.argv[1:]
if args==['--help']:
 print('fixture whisper protocol peer');sys.exit(0)
assert args[args.index('-l')+1]=='en' and '-oj' in args and '-np' in args
out=pathlib.Path(args[args.index('-of')+1]+'.json')
out.write_text(json.dumps({'result':{'language':'en'},'transcription':[
 {'offsets':{'from':0,'to':1600},'text':' Verified fixture segment one. '},
 {'offsets':{'from':1600,'to':3200},'text':' Verified fixture segment two. '}
]}))
"#,
        )
        .unwrap();
        fs::set_permissions(&whisper, fs::Permissions::from_mode(0o700)).unwrap();
        configure(data.path(), "whisper", whisper.to_str().unwrap()).unwrap();
        let model = data.path().join("fixture-model.bin");
        fs::write(&model, b"ggml protocol peer model").unwrap();
        configure(data.path(), "whisperModel", model.to_str().unwrap()).unwrap();
        let source = fs::read(std::env::var("MOXCREW_TRANSCRIPTION_SOURCE").unwrap()).unwrap();
        let output = run(
            data.path(),
            "audio.transcribe",
            &json!({"source":"inputs/source.wav","language":"en"}),
            &[("inputs/source.wav".into(), source.clone())],
            None,
        )
        .await
        .unwrap();
        let evidence = PathBuf::from(std::env::var("MOXCREW_TRANSCRIPTION_EVIDENCE_DIR").unwrap());
        fs::create_dir_all(&evidence).unwrap();
        fs::write(evidence.join("source-fixture.wav"), source).unwrap();
        for (name, bytes) in output.files {
            fs::write(evidence.join(name), bytes).unwrap();
        }
        for name in [
            "source-fixture.wav",
            "transcript.json",
            "transcript.txt",
            "captions.srt",
            "captions.vtt",
            "transcript-manifest.json",
        ] {
            assert!(
                evidence.join(name).is_file(),
                "missing evidence file {name}"
            );
        }
        let manifest: Value =
            serde_json::from_slice(&fs::read(evidence.join("transcript-manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest["segmentCount"], 2);
        assert_eq!(manifest["verification"]["originalPreserved"], true);
        assert_eq!(manifest["verification"]["normalizedAudioDecoded"], true);
        assert_eq!(manifest["verification"]["subtitleFilesReopened"], true);
        assert_eq!(manifest["verification"]["contentAccuracyReviewed"], false);
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
