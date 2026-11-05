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
        "browser.inspect" | "browser.draft" | "browser.research" | "images.campaign"
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
    #[test]
    fn branded_campaign_worker_checks_formats_contrast_dimensions_and_hashes() {
        let tmp = tempfile::tempdir().unwrap();
        let tools = production::paths(tmp.path()).unwrap();
        let python = tools["python"]
            .as_str()
            .expect("Python is required by production fixture tests");
        store::atomic_write(&tmp.path().join("browser.py"), SCRIPT.as_bytes()).unwrap();
        let code = r#"import base64,hashlib,json,runpy,struct
from pathlib import Path
b=runpy.run_path('browser.py',run_name='campaign_test')
def png(width,height):return b'\x89PNG\r\n\x1a\n'+b'\x00\x00\x00\rIHDR'+struct.pack('>II',width,height)
Path('inputs').mkdir();Path('inputs/hero.png').write_bytes(png(64,64))
class Peer:
 def __init__(self,*args):self.attached={};self.session=None;self.width=1200;self.height=800;self.sources=[];self.requests=0;self.deadline=10**20
 def call(self,method,args=None,session=True):
  args=args or {}
  if method=='Target.createTarget':self.attached['page']='session';return {'targetId':'page'}
  if method=='Emulation.setDeviceMetricsOverride':self.width=args['width'];self.height=args['height']
  if method=='Page.getFrameTree':return {'frameTree':{'frame':{'id':'frame'}}}
  if method=='Page.captureScreenshot':
   return {'data':base64.b64encode(png(self.width,self.height)).decode()}
  return {}
 def evaluate(self,expression):
  if expression=='document.readyState':return 'complete'
  if 'img[data-campaign-source]' in expression:return {'count':1,'ready':True}
  return [{'id':'title','x':40,'y':40,'r':self.width-40,'b':self.height-40}]
 def close(self):pass
b['image_campaign'].__globals__['Browser']=Peer
args={'title':'Ship the work, not the busywork','eyebrow':'MOXCREW CAMPAIGN','subtitle':'One focused local crew turns approved copy into a consistent launch system.','callToAction':'Start local','formats':['square','landscape','portrait','thumbnail'],'brand':{'name':'Moxcrew','primary':'2854F4','accent':'FF7A2F','background':'F8F7F3','text':'15223D','muted':'56627A'},'hero':{'source':'inputs/hero.png','alt':'Checked campaign source'}}
result=b['run']({'operation':'images.campaign','arguments':args,'tools':{'chrome':'/fake/chrome'}})
assert result['kind']=='branded_campaign_assets' and result['assets']==4 and result['humanReviewRequired']
out=Path('outputs'); files=sorted(path.name for path in out.iterdir())
assert len(files)==11 and files[0].startswith('campaign-')
manifest=json.loads((out/'campaign-manifest.json').read_text())
assert manifest['checks']['networkEnabled'] is False and manifest['checks']['pageScriptsEnabled'] is False
assert manifest['inputs']['hero']['sha256']==hashlib.sha256(png(64,64)).hexdigest()
assert manifest['inputs']['hero']['alt']=='Checked campaign source'
assert [item['format'] for item in manifest['artifacts']]==args['formats']
for item in manifest['artifacts']:
 for kind in ('svg','png'):
  payload=(out/item[kind]['file']).read_bytes()
  assert len(payload)==item[kind]['bytes'] and hashlib.sha256(payload).hexdigest()==item[kind]['sha256']
assert hashlib.sha256((out/'campaign-manifest.json').read_bytes()).hexdigest()==result['manifestSha256']
bad=dict(args);bad['formats']=['square','square']
try:b['run']({'operation':'images.campaign','arguments':bad,'tools':{'chrome':'/fake/chrome'}})
except ValueError as error:assert str(error)=='BROWSER_CAMPAIGN_FORMAT_INVALID'
else:raise AssertionError('duplicate format accepted')
bad=dict(args);bad['brand']=dict(args['brand'],text='F8F7F3')
try:b['run']({'operation':'images.campaign','arguments':bad,'tools':{'chrome':'/fake/chrome'}})
except ValueError as error:assert str(error)=='BROWSER_CAMPAIGN_CONTRAST_INVALID'
else:raise AssertionError('low contrast accepted')
"#;
        let result = std::process::Command::new(python)
            .args(["-I", "-B", "-c", code])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(production::OPERATIONS.contains(&"images.campaign"));
        let production_tool = crate::project_studio::tool_schema()
            .into_iter()
            .find(|tool| tool["name"] == "production_run")
            .unwrap();
        assert!(production_tool["description"]
            .as_str()
            .unwrap()
            .contains("images.campaign"));
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
