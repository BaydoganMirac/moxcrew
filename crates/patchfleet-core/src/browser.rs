//! Fixed read/inspect/form-draft operations in a disposable external browser.
use crate::{production, store, Error, Result};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

const SCRIPT: &str = include_str!("../assets/tools/browser.py");
const LIMIT: usize = 25 * 1024 * 1024;
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) async fn run(
    data: &Path,
    operation: &str,
    arguments: &Value,
    inputs: &[(String, Vec<u8>)],
    active: Option<Arc<AtomicBool>>,
) -> Result<production::Output> {
    if !matches!(
        operation,
        "browser.inspect" | "browser.draft" | "browser.research"
    ) {
        return Err(Error("BROWSER_OPERATION_UNSUPPORTED".into()));
    }
    if inputs.len() > 12 || inputs.iter().map(|(_, b)| b.len()).sum::<usize>() > LIMIT {
        return Err(Error("PRODUCTION_INPUT_LIMIT".into()));
    }
    let tools = production::paths(data)?;
    let python = tools["python"]
        .as_str()
        .ok_or_else(|| Error("PRODUCTION_PYTHON_REQUIRED".into()))?;
    if tools["chrome"].as_str().is_none() {
        return Err(Error("BROWSER_CHROME_REQUIRED".into()));
    }
    let parent = std::fs::canonicalize(data)?.join("browser-scratch");
    store::private_directory(&parent)?;
    let scratch = Scratch(parent.join(uuid::Uuid::new_v4().to_string()));
    for p in [
        &scratch.0,
        &scratch.0.join("inputs"),
        &scratch.0.join("outputs"),
    ] {
        store::private_directory(p)?;
    }
    let mut args = arguments.clone();
    let mut mapping = Vec::new();
    for (index, (name, bytes)) in inputs.iter().enumerate() {
        if mapping.iter().any(|(old, _)| old == name) {
            return Err(Error("PRODUCTION_INPUT_PATH_INVALID".into()));
        }
        let extension = Path::new(name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        if extension.len() > 12 || !extension.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err(Error("PRODUCTION_INPUT_PATH_INVALID".into()));
        }
        let target = format!("inputs/source-{index}.{extension}");
        store::atomic_write(&scratch.0.join(&target), bytes)?;
        mapping.push((name.clone(), target));
    }
    production::remap_sources(&mut args, &mapping);
    store::atomic_write(&scratch.0.join("browser.py"), SCRIPT.as_bytes())?;
    let request =
        serde_json::to_vec(&json!({"operation":operation,"arguments":args,"tools":tools}))?;
    let mut command = tokio::process::Command::new(python);
    command
        .args(["-I", "-B", "browser.py"])
        .current_dir(&scratch.0)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let bytes = crate::providers::typed_process(
        command,
        request,
        active.unwrap_or_else(|| Arc::new(AtomicBool::new(true))),
        Duration::from_secs(45),
        2 * 1024 * 1024,
    )
    .await
    .map_err(|code| {
        Error(
            match code.as_str() {
                "EXTRACTION_CANCELLED" => "PRODUCTION_CANCELLED",
                "EXTRACTION_TIMEOUT" => "BROWSER_TIMEOUT",
                _ => "BROWSER_FAILED",
            }
            .into(),
        )
    })?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|_| Error("BROWSER_OUTPUT_INVALID".into()))?;
    if let Some(code) = value["error"].as_str() {
        return Err(Error(
            if code.len() < 80 && code.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
                code
            } else {
                "BROWSER_FAILED"
            }
            .into(),
        ));
    }
    let mut files = Vec::new();
    let mut total = 0;
    for entry in std::fs::read_dir(scratch.0.join("outputs"))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        let bytes = crate::conversation::copied_bytes(
            &scratch.0,
            &PathBuf::from("outputs").join(&name),
            (LIMIT - total) as u64,
        )?;
        total += bytes.len();
        files.push((name, bytes));
        if files.len() > 16 {
            return Err(Error("PRODUCTION_OUTPUT_LIMIT".into()));
        }
    }
    if files.is_empty() {
        return Err(Error("BROWSER_OUTPUT_INVALID".into()));
    }
    Ok(production::Output { value, files })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_policy_denies_private_transition_and_non_read_requests() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = production::paths(tmp.path()).unwrap();
        let python = tools["python"]
            .as_str()
            .expect("Python is required by production fixture tests");
        store::atomic_write(&tmp.path().join("browser.py"), SCRIPT.as_bytes()).unwrap();
        let code = r#"import os,runpy,time
b=runpy.run_path('browser.py',run_name='policy_test')
for ip in ('127.0.0.1','10.0.0.1','100.64.0.1','192.168.1.1','192.0.0.8','192.88.99.1','::1','::ffff:127.0.0.1','2001:db8::1','2002:7f00:1::','3fff::1'):
 assert not b['public_address'](ip),ip
for ip in ('1.1.1.1','8.8.8.8','2606:4700:4700::1111'):
 assert b['public_address'](ip),ip
for url in ('file:///etc/passwd','http://localhost/','https://internal/','http://127.0.0.1/','https://user:pass@example.com/','http://example.com:8080/'):
 try:b['public_url'](url)
 except ValueError:pass
 else:raise AssertionError(url)
try:b['public_get']('https://example.com/','POST')
except ValueError:pass
else:raise AssertionError('POST accepted')
# A browser-level paused attachment must be resumed through its original session.
class Peer:
 def __init__(self,*args):self.attached={};self.session=None;self.sources=[];self.url='https://example.com/';self.deadline=time.monotonic()+35
 def call(self,method,args=None,session=True):
  if method=='Target.createTarget':self.attached['page']='original';return {'targetId':'page'}
  assert method!='Target.attachToTarget','duplicate debugger leaves navigation paused'
  if method=='Runtime.runIfWaitingForDebugger':assert self.session=='original'
  if method=='Page.navigate':self.url=args['url'];self.sources.append({'url':self.url,'sha256':'a'*64,'bytes':128})
  if method=='Page.captureScreenshot':return {'data':'iVBORw0KGgo='}
  return {}
 def evaluate(self,expression):
  if expression=='document.readyState':return 'complete'
  return {'title':'Source','url':self.url,'text':'Actual source','links':[],'fields':[]}
 def close(self):pass
b['run'].__globals__['Browser']=Peer
result=b['run']({'operation':'browser.inspect','arguments':{'url':'https://example.com/'},'tools':{'chrome':os.environ['MOXCREW_TEST_CHROME']}})
assert result['requests']==1 and not result['submitted']
research=b['run']({'operation':'browser.research','arguments':{
 'title':'Public research','question':'What does the page establish?',
 'sources':[{'url':'https://example.com/','label':'Public source'}],
 'claims':[
  {'claim':'The page contains source text.','classification':'fact','evidence':[{'source':'https://example.com/','quote':'Actual source'}],'note':None},
  {'claim':'This may indicate availability.','classification':'inference','evidence':[{'source':'https://example.com/','quote':'Actual source'}],'note':'The page does not state availability directly.'},
  {'claim':'Publication date is known.','classification':'unknown','evidence':[],'note':'The page does not provide a publication date.'}
 ]},'tools':{'chrome':os.environ['MOXCREW_TEST_CHROME']}})
assert research['kind']=='web_research' and research['summary']=={'sources':1,'claims':3,'facts':1,'inferences':1,'unknowns':1}
report=(b['ROOT']/'outputs'/'research-report.md').read_text()
assert '## Claim 1 · Fact' in report and 'accessed ' in report and 'response `'+('a'*64)+'`' in report
rows=(b['ROOT']/'outputs'/'sources.csv').read_text()
assert rows.startswith('claim,source_url,accessed_at,evidence_location') and 'https://example.com/' in rows
evidence=__import__('json').loads((b['ROOT']/'outputs'/'research-evidence.json').read_text())
assert evidence['claims'][0]['evidence'][0]['line']==1 and evidence['sources'][0]['responseSha256']=='a'*64
try:
 b['run']({'operation':'browser.research','arguments':{'title':'Bad','question':'Bad evidence?',
  'sources':[{'url':'https://example.com/','label':'Public source'}],
  'claims':[{'claim':'Invented','classification':'fact','evidence':[{'source':'https://example.com/','quote':'Fabricated quote'}],'note':None}]},
  'tools':{'chrome':os.environ['MOXCREW_TEST_CHROME']}})
except ValueError as error:assert str(error)=='BROWSER_RESEARCH_EVIDENCE_MISMATCH'
else:raise AssertionError('fabricated browser evidence accepted')
try:
 b['run']({'operation':'browser.research','arguments':{'title':'Bad','question':'Bad class?',
  'sources':[{'url':'https://example.com/','label':'Public source'}],
  'claims':[{'claim':'Invalid class','classification':'verified','evidence':[{'source':'https://example.com/','quote':'Actual source'}],'note':None}]},
  'tools':{'chrome':os.environ['MOXCREW_TEST_CHROME']}})
except ValueError as error:assert str(error)=='BROWSER_RESEARCH_CLASSIFICATION_INVALID'
else:raise AssertionError('invalid browser classification accepted')
try:
 b['run']({'operation':'browser.research','arguments':{'title':'Bad','question':'Unused source?',
  'sources':[{'url':'https://example.com/','label':'First'},{'url':'https://example.org/','label':'Second'}],
  'claims':[{'claim':'Only first used','classification':'fact','evidence':[{'source':'https://example.com/','quote':'Actual source'}],'note':None}]},
  'tools':{'chrome':os.environ['MOXCREW_TEST_CHROME']}})
except ValueError as error:assert str(error)=='BROWSER_RESEARCH_UNUSED_SOURCE'
else:raise AssertionError('unused browser source accepted')
"#;
        let result = std::process::Command::new(python)
            .args(["-I", "-B", "-c", code])
            .env("MOXCREW_TEST_CHROME", tmp.path().join("chrome"))
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(production::OPERATIONS.contains(&"browser.research"));
        let schema = crate::project_studio::tool_schema();
        let production_tool = schema
            .iter()
            .find(|tool| tool["name"] == "production_run")
            .unwrap();
        assert!(production_tool["description"]
            .as_str()
            .unwrap()
            .contains("browser.research"));
    }
    #[tokio::test]
    async fn unknown_browser_operations_never_start_a_process() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            run(tmp.path(), "browser.execute_js", &json!({}), &[], None)
                .await
                .err()
                .unwrap()
                .0,
            "BROWSER_OPERATION_UNSUPPORTED"
        );
        assert!(!tmp.path().join("browser-scratch").exists());
    }
}
