//! Fixed local ComfyUI graph. No user graph, API node, redirect, proxy or global interrupt.
use super::{error, Output, LIMIT};
use crate::{store, Result};
use reqwest::{Client, Method, Url};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

fn config_path(data: &Path, project: &str) -> Result<PathBuf> {
    let id =
        uuid::Uuid::parse_str(project).map_err(|_| error("PRODUCTION_IMAGE_PROJECT_INVALID"))?;
    if id.to_string() != project {
        return Err(error("PRODUCTION_IMAGE_PROJECT_INVALID"));
    }
    Ok(data.join(format!("image-engine-{id}.json")))
}
fn endpoint(value: &str) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| error("PRODUCTION_IMAGE_URL_INVALID"))?;
    if value.len() > 256
        || url.scheme() != "http"
        || !matches!(url.host_str(), Some("127.0.0.1" | "[::1]" | "::1"))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || url.port_or_known_default().is_none()
    {
        return Err(error("PRODUCTION_IMAGE_URL_INVALID"));
    }
    Ok(url)
}
fn checkpoint(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || value.contains(['\\', '\0', ':'])
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(error("PRODUCTION_IMAGE_MODEL_INVALID"));
    }
    Ok(())
}
fn read(data: &Path, project: &str) -> Result<Option<Value>> {
    let path = config_path(data, project)?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = store::read_private_file(&path)?;
    if bytes.len() > 4096 {
        return Err(error("PRODUCTION_IMAGE_CONFIG_INVALID"));
    }
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| error("PRODUCTION_IMAGE_CONFIG_INVALID"))?;
    endpoint(
        value["url"]
            .as_str()
            .ok_or_else(|| error("PRODUCTION_IMAGE_CONFIG_INVALID"))?,
    )?;
    checkpoint(
        value["checkpoint"]
            .as_str()
            .ok_or_else(|| error("PRODUCTION_IMAGE_CONFIG_INVALID"))?,
    )?;
    Ok(Some(value))
}
pub(crate) fn configure_image_engine(
    data: &Path,
    project: &str,
    url: &str,
    model: &str,
) -> Result<Value> {
    let url = endpoint(url)?;
    checkpoint(model)?;
    let path = config_path(data, project)?;
    store::private_directory(data)?;
    store::atomic_write(
        &path,
        &serde_json::to_vec(&json!({"url":url.as_str(),"checkpoint":model}))?,
    )?;
    Ok(
        json!({"configured":true,"available":false,"checkpoint":model,"reasonCode":"PRODUCTION_IMAGE_CONFIGURED_UNVERIFIED"}),
    )
}
pub(crate) fn remove_image_engine(data: &Path, project: &str) -> Result<Value> {
    let path = config_path(data, project)?;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(json!({"configured":false,"available":false,"reasonCode":"PRODUCTION_IMAGE_SETUP_REQUIRED"}))
}
fn client() -> Result<Client> {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| error("PRODUCTION_IMAGE_ENGINE_UNAVAILABLE"))
}
async fn bytes(
    client: &Client,
    method: Method,
    url: Url,
    body: Option<(String, Vec<u8>)>,
    limit: usize,
) -> Result<Vec<u8>> {
    let mut request = client.request(method, url);
    if let Some((kind, body)) = body {
        request = request.header("Content-Type", kind).body(body);
    }
    let mut response = request
        .send()
        .await
        .map_err(|_| error("PRODUCTION_IMAGE_ENGINE_UNAVAILABLE"))?;
    if !response.status().is_success() {
        return Err(error("PRODUCTION_IMAGE_ENGINE_REJECTED"));
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(error("PRODUCTION_IMAGE_OUTPUT_LIMIT"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| error("PRODUCTION_IMAGE_ENGINE_UNAVAILABLE"))?
    {
        if bytes.len() + chunk.len() > limit {
            return Err(error("PRODUCTION_IMAGE_OUTPUT_LIMIT"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
async fn request(
    client: &Client,
    base: &Url,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> Result<Value> {
    let body = body
        .map(|v| serde_json::to_vec(&v).map(|b| ("application/json".into(), b)))
        .transpose()?;
    let raw = bytes(
        client,
        method,
        base.join(path)
            .map_err(|_| error("PRODUCTION_IMAGE_URL_INVALID"))?,
        body,
        1024 * 1024,
    )
    .await?;
    serde_json::from_slice(&raw).map_err(|_| error("PRODUCTION_IMAGE_PROTOCOL_INVALID"))
}
async fn catalog(client: &Client, base: &Url, model: &str) -> Result<Vec<String>> {
    let info = request(
        client,
        base,
        Method::GET,
        "object_info/CheckpointLoaderSimple",
        None,
    )
    .await?;
    let list = info["CheckpointLoaderSimple"]["input"]["required"]["ckpt_name"][0]
        .as_array()
        .ok_or_else(|| error("PRODUCTION_IMAGE_PROTOCOL_INVALID"))?;
    if list.len() > 512 {
        return Err(error("PRODUCTION_IMAGE_OUTPUT_LIMIT"));
    }
    let names: Vec<_> = list
        .iter()
        .filter_map(Value::as_str)
        .filter(|s| checkpoint(s).is_ok())
        .map(str::to_owned)
        .collect();
    if !names.iter().any(|s| s == model) {
        return Err(error("PRODUCTION_IMAGE_MODEL_MISSING"));
    }
    // New ComfyUI's per-ID cancel is atomic. Older global /interrupt is deliberately unsupported.
    let probe = uuid::Uuid::new_v4();
    let cancelled = request(
        client,
        base,
        Method::POST,
        &format!("api/jobs/{probe}/cancel"),
        Some(json!({})),
    )
    .await
    .map_err(|_| error("PRODUCTION_IMAGE_SCOPED_CANCEL_REQUIRED"))?;
    if cancelled["cancelled"] != false {
        return Err(error("PRODUCTION_IMAGE_PROTOCOL_INVALID"));
    }
    Ok(names)
}
pub(crate) async fn image_readiness(data: &Path, project: &str) -> Result<Value> {
    let Some(config) = read(data, project)? else {
        return Ok(
            json!({"configured":false,"available":false,"reasonCode":"PRODUCTION_IMAGE_SETUP_REQUIRED","setupUrl":"https://docs.comfy.org/installation/system_requirements"}),
        );
    };
    let client = client()?;
    let url = endpoint(config["url"].as_str().unwrap())?;
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        catalog(&client, &url, config["checkpoint"].as_str().unwrap()),
    )
    .await
    .unwrap_or_else(|_| Err(error("PRODUCTION_IMAGE_ENGINE_UNAVAILABLE")));
    Ok(match result {
        Ok(models) => {
            json!({"configured":true,"available":true,"checkpoint":config["checkpoint"],"models":models,"reasonCode":"PRODUCTION_IMAGE_ENGINE_AVAILABLE","generationVerified":false})
        }
        Err(reason) => {
            json!({"configured":true,"available":false,"checkpoint":config["checkpoint"],"reasonCode":reason.0})
        }
    })
}
struct Cancel {
    client: Client,
    base: Url,
    id: String,
    armed: bool,
}
impl Drop for Cancel {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let client = self.client.clone();
        let base = self.base.clone();
        let id = self.id.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let _ = request(
                    &client,
                    &base,
                    Method::POST,
                    &format!("api/jobs/{id}/cancel"),
                    Some(json!({})),
                )
                .await;
            });
        }
    }
}
fn integer(args: &Value, name: &str, default: u64, low: u64, high: u64) -> Result<u64> {
    let value = if args.get(name).is_some() {
        args[name].as_u64()
    } else {
        Some(default)
    };
    value
        .filter(|n| *n >= low && *n <= high)
        .ok_or_else(|| error("PRODUCTION_INVALID_INPUT"))
}
fn text(args: &Value, name: &str, optional: bool) -> Result<String> {
    let value = if optional && args.get(name).is_none() {
        ""
    } else {
        args[name]
            .as_str()
            .ok_or_else(|| error("PRODUCTION_INVALID_INPUT"))?
    };
    if value.len() > 8000
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
        || (!optional && value.trim().is_empty())
    {
        return Err(error("PRODUCTION_INVALID_INPUT"));
    }
    Ok(value.into())
}
async fn upload(client: &Client, base: &Url, data: &[u8], name: &str) -> Result<String> {
    let boundary = format!("moxcrew{}", uuid::Uuid::new_v4().simple());
    let mut body = format!("--{boundary}\r\nContent-Disposition: form-data; name=\"image\"; filename=\"{name}\"\r\nContent-Type: image/png\r\n\r\n").into_bytes();
    body.extend_from_slice(data);
    body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"type\"\r\n\r\ninput\r\n--{boundary}--\r\n").as_bytes());
    let raw = bytes(
        client,
        Method::POST,
        base.join("upload/image").unwrap(),
        Some((format!("multipart/form-data; boundary={boundary}"), body)),
        4096,
    )
    .await?;
    let value: Value =
        serde_json::from_slice(&raw).map_err(|_| error("PRODUCTION_IMAGE_PROTOCOL_INVALID"))?;
    if value["name"] != name
        || value["subfolder"].as_str().is_some_and(|s| !s.is_empty())
        || value["type"] != "input"
    {
        return Err(error("PRODUCTION_IMAGE_PROTOCOL_INVALID"));
    }
    Ok(name.into())
}
async fn verified(
    data: &Path,
    bytes: Vec<u8>,
    width: u64,
    height: u64,
    active: Arc<AtomicBool>,
) -> Result<Vec<u8>> {
    let out = super::run(
        data,
        "images.verify",
        &json!({"source":"inputs/image.png","width":width,"height":height}),
        &[("inputs/image.png".into(), bytes)],
        Some(active),
    )
    .await?;
    out.files
        .into_iter()
        .find(|(name, _)| name == "image.png")
        .map(|(_, b)| b)
        .ok_or_else(|| error("PRODUCTION_IMAGE_OUTPUT_INVALID"))
}
pub(super) async fn run(
    data: &Path,
    project: &str,
    operation: &str,
    args: &Value,
    inputs: &[(String, Vec<u8>)],
    active: Option<Arc<AtomicBool>>,
) -> Result<Output> {
    let allowed = [
        "prompt",
        "width",
        "height",
        "negativePrompt",
        "seed",
        "steps",
        "source",
        "mask",
        "strength",
    ];
    let object = args
        .as_object()
        .ok_or_else(|| error("PRODUCTION_INVALID_INPUT"))?;
    if ["prompt", "width", "height"]
        .iter()
        .any(|key| !object.contains_key(*key))
        || inputs
            .iter()
            .enumerate()
            .any(|(i, (name, _))| inputs[..i].iter().any(|(previous, _)| previous == name))
    {
        return Err(error("PRODUCTION_INVALID_INPUT"));
    }
    if object.keys().any(|key| !allowed.contains(&key.as_str()))
        || (operation == "images.generate"
            && ["source", "mask", "strength"]
                .iter()
                .any(|key| object.contains_key(*key)))
    {
        return Err(error("PRODUCTION_INVALID_INPUT"));
    }
    let prompt = text(args, "prompt", false)?;
    let negative = text(args, "negativePrompt", true)?;
    let width = integer(args, "width", 512, 64, 2048)?;
    let height = integer(args, "height", 512, 64, 2048)?;
    if width % 8 != 0 || height % 8 != 0 {
        return Err(error("PRODUCTION_INVALID_INPUT"));
    }
    let seed = integer(args, "seed", 0, 0, 9_007_199_254_740_991)?;
    let steps = integer(args, "steps", 20, 1, 50)?;
    let strength = if args.get("strength").is_some() {
        args["strength"]
            .as_f64()
            .filter(|n| n.is_finite() && *n > 0.0 && *n <= 1.0)
            .ok_or_else(|| error("PRODUCTION_INVALID_INPUT"))?
    } else {
        0.75
    };
    let config = read(data, project)?.ok_or_else(|| error("PRODUCTION_IMAGE_SETUP_REQUIRED"))?;
    let base = endpoint(config["url"].as_str().unwrap())?;
    let client = client()?;
    let id = uuid::Uuid::new_v4().to_string();
    let active = active.unwrap_or_else(|| Arc::new(AtomicBool::new(true)));
    let cancelled = async {
        while active.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    let mut guard = Cancel {
        client: client.clone(),
        base: base.clone(),
        id: id.clone(),
        armed: false,
    };
    let work = async {
        catalog(&client, &base, config["checkpoint"].as_str().unwrap()).await?;
        let mut graph = json!({
            "1":{"class_type":"CheckpointLoaderSimple","inputs":{"ckpt_name":config["checkpoint"]}},
            "2":{"class_type":"CLIPTextEncode","inputs":{"text":prompt,"clip":["1",1]}},
            "3":{"class_type":"CLIPTextEncode","inputs":{"text":negative,"clip":["1",1]}},
            "4":{"class_type":"EmptyLatentImage","inputs":{"width":width,"height":height,"batch_size":1}},
            "5":{"class_type":"KSampler","inputs":{"model":["1",0],"positive":["2",0],"negative":["3",0],"latent_image":["4",0],"seed":seed,"steps":steps,"cfg":7.0,"sampler_name":"euler","scheduler":"normal","denoise":1.0}},
            "6":{"class_type":"VAEDecode","inputs":{"samples":["5",0],"vae":["1",2]}},
            "7":{"class_type":"SaveImage","inputs":{"images":["6",0],"filename_prefix":format!("moxcrew-{id}")}}
        });
        if operation == "images.edit" {
            let name = text(args, "source", false)?;
            let reference = inputs
                .iter()
                .find(|(path, _)| path == &name)
                .ok_or_else(|| error("PRODUCTION_INPUT_PATH_INVALID"))?;
            let reference =
                verified(data, reference.1.clone(), width, height, active.clone()).await?;
            let uploaded = upload(
                &client,
                &base,
                &reference,
                &format!("moxcrew-{id}-reference.png"),
            )
            .await?;
            graph["8"] = json!({"class_type":"LoadImage","inputs":{"image":uploaded}});
            graph["4"] =
                json!({"class_type":"VAEEncode","inputs":{"pixels":["8",0],"vae":["1",2]}});
            graph["5"]["inputs"]["denoise"] = json!(strength);
            if let Some(mask) = args.get("mask") {
                if mask
                    .as_object()
                    .is_none_or(|m| m.len() != 1 || !m.contains_key("source"))
                {
                    return Err(error("PRODUCTION_INVALID_INPUT"));
                }
                let name = text(mask, "source", false)?;
                let mask = inputs
                    .iter()
                    .find(|(p, _)| p == &name)
                    .ok_or_else(|| error("PRODUCTION_INPUT_PATH_INVALID"))?;
                let mask = verified(data, mask.1.clone(), width, height, active.clone()).await?;
                let uploaded =
                    upload(&client, &base, &mask, &format!("moxcrew-{id}-mask.png")).await?;
                graph["9"] = json!({"class_type":"LoadImageMask","inputs":{"image":uploaded,"channel":"red"}});
                graph["4"] = json!({"class_type":"VAEEncodeForInpaint","inputs":{"pixels":["8",0],"vae":["1",2],"mask":["9",0],"grow_mask_by":0}});
            }
        }
        guard.armed = true;
        let accepted = request(
            &client,
            &base,
            Method::POST,
            "prompt",
            Some(json!({"prompt_id":id,"client_id":id,"prompt":graph})),
        )
        .await?;
        if accepted["prompt_id"] != id
            || accepted.get("error").is_some()
            || accepted["node_errors"]
                .as_object()
                .is_some_and(|m| !m.is_empty())
        {
            return Err(error("PRODUCTION_IMAGE_PROTOCOL_INVALID"));
        }
        loop {
            let history =
                request(&client, &base, Method::GET, &format!("history/{id}"), None).await?;
            if let Some(result) = history.get(&id) {
                if result["status"]["status_str"] == "error" {
                    return Err(error("PRODUCTION_IMAGE_GENERATION_FAILED"));
                }
                if result["status"]["completed"] == true {
                    if result["status"]["status_str"] != "success" {
                        return Err(error("PRODUCTION_IMAGE_GENERATION_FAILED"));
                    }
                    let images = result["outputs"]["7"]["images"]
                        .as_array()
                        .filter(|a| a.len() == 1)
                        .ok_or_else(|| error("PRODUCTION_IMAGE_PROTOCOL_INVALID"))?;
                    let image = &images[0];
                    let filename = image["filename"]
                        .as_str()
                        .ok_or_else(|| error("PRODUCTION_IMAGE_PROTOCOL_INVALID"))?;
                    if filename.len() > 180
                        || !filename.starts_with(&format!("moxcrew-{id}_"))
                        || !filename.ends_with(".png")
                        || filename.contains(['/', '\\'])
                        || image["type"] != "output"
                        || image["subfolder"] != ""
                    {
                        return Err(error("PRODUCTION_IMAGE_PROTOCOL_INVALID"));
                    }
                    let mut url = base.join("view").unwrap();
                    url.query_pairs_mut()
                        .append_pair("filename", filename)
                        .append_pair("type", "output")
                        .append_pair("subfolder", "");
                    let image = bytes(&client, Method::GET, url, None, LIMIT).await?;
                    let image = verified(data, image, width, height, active.clone()).await?;
                    guard.armed = false;
                    return Ok(Output {
                        value: json!({"kind":"image","engine":"comfyui","width":width,"height":height,"seed":seed,"verified":true}),
                        files: vec![("image.png".into(), image)],
                    });
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    };
    let result = tokio::select! {
        _=cancelled => Err(error("PRODUCTION_CANCELLED")),
        result=tokio::time::timeout(Duration::from_secs(300),work) => result.unwrap_or_else(|_| Err(error("PRODUCTION_TIMEOUT"))),
    };
    if guard.armed {
        let _ = request(
            &client,
            &base,
            Method::POST,
            &format!("api/jobs/{id}/cancel"),
            Some(json!({})),
        )
        .await;
        guard.armed = false;
    }
    result
}
