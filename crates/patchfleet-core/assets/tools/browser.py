"""Fixed browser inspection/drafting. Isolated profile, pinned public GETs, no submit."""
import base64
import csv
import hashlib
import html
import http.client
import ipaddress
import json
import os
from pathlib import Path
import select
import signal
import socket
import ssl
import sys
import time
import urllib.parse
from datetime import datetime, timezone

MAX_BODY = 4 * 1024 * 1024
MAX_TOTAL = 12 * 1024 * 1024
ROOT = Path.cwd().resolve()


def require(ok, code="BROWSER_INVALID_INPUT"):
    if not ok:
        raise ValueError(code)


def exact(value, required, optional=()):
    require(isinstance(value, dict) and set(value) == set(required) | set(optional)
            and all(key in value for key in required))


def public_address(value):
    address = ipaddress.ip_address(value)
    if getattr(address, "ipv4_mapped", None):
        address = address.ipv4_mapped
    reserved = (("192.0.0.0/24", "192.88.99.0/24") if address.version == 4 else
                ("2001::/23", "2001:db8::/32", "2002::/16", "3fff::/20"))
    return address.is_global and not address.is_multicast and not any(address in ipaddress.ip_network(net) for net in reserved)


def public_url(raw):
    require(isinstance(raw, str) and 0 < len(raw) <= 4096)
    url = urllib.parse.urlsplit(raw)
    require(url.scheme in ("http", "https") and url.hostname and not url.username
            and not url.password and url.port in (None, 443 if url.scheme == "https" else 80),
            "BROWSER_URL_DENIED")
    host = url.hostname.lower()
    require(not host.endswith((".local", ".localhost")) and host != "localhost", "BROWSER_PRIVATE_ADDRESS_DENIED")
    try:
        require(public_address(host), "BROWSER_PRIVATE_ADDRESS_DENIED")
    except ValueError as error:
        if str(error).startswith("BROWSER_"):
            raise
        require("." in host, "BROWSER_PRIVATE_ADDRESS_DENIED")
    return url._replace(fragment="")


def public_get(raw, method="GET"):
    url = public_url(raw)
    require(method in ("GET", "HEAD"), "BROWSER_WRITE_DENIED")
    port = url.port or (443 if url.scheme == "https" else 80)
    records = socket.getaddrinfo(url.hostname, port, type=socket.SOCK_STREAM)
    addresses = list(dict.fromkeys(row[4][0] for row in records))
    require(0 < len(addresses) <= 16 and all(public_address(ip) for ip in addresses), "BROWSER_PRIVATE_ADDRESS_DENIED")
    connection = http.client.HTTPConnection(url.hostname, port, timeout=4)
    stream = socket.create_connection((addresses[0], port), timeout=4)
    try:
        if url.scheme == "https":
            stream = ssl.create_default_context().wrap_socket(stream, server_hostname=url.hostname)
        connection.sock = stream
        path = urllib.parse.urlunsplit(("", "", url.path or "/", url.query, ""))
        connection.request(method, path, headers={"User-Agent": "Moxcrew/0.12 source reader", "Accept-Encoding": "identity"})
        response = connection.getresponse()
        size = response.getheader("Content-Length")
        require(size is None or int(size) <= MAX_BODY, "BROWSER_SOURCE_LIMIT")
        body = response.read(MAX_BODY + 1)
        require(len(body) <= MAX_BODY, "BROWSER_SOURCE_LIMIT")
        headers = [{"name": key, "value": value} for key, value in response.getheaders()
                   if key.lower() in ("content-type", "location", "content-security-policy", "x-content-type-options")
                   and len(value) <= 8192]
        return response.status, headers, body
    finally:
        connection.close()
        stream.close()


class Browser:
    def __init__(self, executable, network):
        require(os.name == "posix", "BROWSER_PLATFORM_UNSUPPORTED")
        self.network = network
        self.deadline = time.monotonic() + 35
        self.next_id = 0
        self.buffer = bytearray()
        self.total = 0
        self.requests = 0
        self.sources = []
        self.session = None
        self.attached = {}
        profile = ROOT / "profile"
        profile.mkdir(mode=0o700)
        incoming, self.writer = os.pipe()
        self.reader, outgoing = os.pipe()
        self.pid = os.fork()
        if self.pid == 0:
            try:
                # Duplicate away from CDP's fixed descriptors before assigning them.
                import fcntl
                a = fcntl.fcntl(incoming, fcntl.F_DUPFD, 10)
                b = fcntl.fcntl(outgoing, fcntl.F_DUPFD, 20)
                os.dup2(a, 3, inheritable=True)
                os.dup2(b, 4, inheritable=True)
                null = os.open(os.devnull, os.O_RDWR)
                for fd in (0, 1, 2):
                    os.dup2(null, fd)
                os.closerange(5, 65536)
                argv = [executable, "--headless=new", "--remote-debugging-pipe",
                        "--user-data-dir=" + str(profile), "--no-first-run", "--no-default-browser-check",
                        "--disable-background-networking", "--disable-component-update", "--disable-sync",
                        "--disable-extensions", "--disable-client-side-phishing-detection", "--no-proxy-server",
                        "--host-resolver-rules=MAP * ~NOTFOUND", "--disable-quic", "--disable-breakpad",
                        "--window-size=1200,800", "about:blank"]
                os.execv(executable, argv)
            except BaseException:
                os._exit(127)
        os.close(incoming)
        os.close(outgoing)

    def send(self, method, params=None, session=True):
        self.next_id += 1
        message = {"id": self.next_id, "method": method, "params": params or {}}
        if session and self.session:
            message["sessionId"] = self.session
        payload = json.dumps(message, ensure_ascii=True).encode() + b"\0"
        require(len(payload) <= 2 * 1024 * 1024, "BROWSER_INPUT_LIMIT")
        while payload:
            count = os.write(self.writer, payload)
            payload = payload[count:]
        return self.next_id

    def message(self):
        while b"\0" not in self.buffer:
            remain = self.deadline - time.monotonic()
            require(remain > 0, "BROWSER_TIMEOUT")
            ready, _, _ = select.select([self.reader], [], [], min(remain, 1))
            if not ready:
                continue
            data = os.read(self.reader, 65536)
            require(data, "BROWSER_DISCONNECTED")
            self.buffer.extend(data)
            require(len(self.buffer) <= 8 * 1024 * 1024, "BROWSER_OUTPUT_LIMIT")
        raw, _, rest = self.buffer.partition(b"\0")
        self.buffer = bytearray(rest)
        return json.loads(raw)

    def call(self, method, params=None, session=True):
        target = self.send(method, params, session)
        while True:
            message = self.message()
            if message.get("method") == "Target.attachedToTarget":
                attached = message["params"]
                self.attached[attached["targetInfo"]["targetId"]] = attached["sessionId"]
            elif message.get("method") == "Fetch.requestPaused" and message.get("sessionId") == self.session:
                self.fetch(message["params"])
            elif message.get("method") == "Fetch.authRequired" and message.get("sessionId") == self.session:
                self.send("Fetch.continueWithAuth", {"requestId": message["params"]["requestId"], "authChallengeResponse": {"response": "CancelAuth"}})
            elif message.get("method") == "Page.javascriptDialogOpening" and message.get("sessionId") == self.session:
                self.send("Page.handleJavaScriptDialog", {"accept": False})
            if message.get("id") == target:
                require("error" not in message, "BROWSER_COMMAND_FAILED")
                return message.get("result", {})

    def fetch(self, params):
        request = params["request"]
        try:
            require(self.network and self.requests < 64, "BROWSER_NETWORK_DENIED")
            self.requests += 1
            status, headers, body = public_get(request["url"], request["method"])
            self.total += len(body)
            require(self.total <= MAX_TOTAL, "BROWSER_SOURCE_LIMIT")
            self.sources.append({"url": request["url"], "sha256": hashlib.sha256(body).hexdigest(), "bytes": len(body)})
            self.send("Fetch.fulfillRequest", {"requestId": params["requestId"], "responseCode": status,
                      "responseHeaders": headers, "body": base64.b64encode(body).decode()})
        except (ValueError, OSError, http.client.HTTPException):
            self.send("Fetch.failRequest", {"requestId": params["requestId"], "errorReason": "BlockedByClient"})

    def evaluate(self, expression):
        response = self.call("Runtime.evaluate", {"expression": expression, "returnByValue": True, "awaitPromise": False, "timeout": 2000})
        require("exceptionDetails" not in response, "BROWSER_DOCUMENT_UNSUPPORTED")
        return response.get("result", {}).get("value")

    def close(self):
        for descriptor in (self.reader, self.writer):
            try:
                os.close(descriptor)
            except OSError:
                pass
        try:
            os.kill(self.pid, signal.SIGTERM)
            until = time.monotonic() + 2
            while time.monotonic() < until:
                if os.waitpid(self.pid, os.WNOHANG)[0]:
                    return
                time.sleep(.02)
            os.kill(self.pid, signal.SIGKILL)
            os.waitpid(self.pid, 0)
        except (OSError, ChildProcessError):
            pass


def prepare_browser(browser):
    browser.call("Target.setAutoAttach", {"autoAttach": True, "waitForDebuggerOnStart": True, "flatten": True}, False)
    target = browser.call("Target.createTarget", {"url": "about:blank"}, False)["targetId"]
    while target not in browser.attached:
        message = browser.message()
        if message.get("method") == "Target.attachedToTarget":
            attached = message["params"]
            browser.attached[attached["targetInfo"]["targetId"]] = attached["sessionId"]
    browser.session = browser.attached[target]
    browser.call("Page.enable")
    browser.call("Runtime.enable")
    browser.call("Emulation.setScriptExecutionDisabled", {"value": True})
    browser.call("Network.enable")
    browser.call("Network.setBlockedURLs", {"urls": ["file:*", "filesystem:*", "ftp:*", "ws:*", "wss:*"]})
    browser.call("Fetch.enable", {"patterns": [{"urlPattern": "*", "requestStage": "Request"}], "handleAuthRequests": True})
    browser.call("Browser.setDownloadBehavior", {"behavior": "deny"}, False)
    browser.call("Emulation.setDeviceMetricsOverride", {"width": 1200, "height": 800, "deviceScaleFactor": 1, "mobile": False})
    browser.call("Emulation.setEmulatedMedia", {"features": [{"name": "prefers-color-scheme", "value": "light"}]})
    browser.call("Runtime.runIfWaitingForDebugger")


def wait_for_page(browser):
    until = min(browser.deadline, time.monotonic() + 8)
    while time.monotonic() < until:
        if browser.evaluate("document.readyState") == "complete":
            return
        time.sleep(.05)
    require(False, "BROWSER_TIMEOUT")


def web_research(args, tools):
    exact(args, ("title", "question", "sources", "claims"))
    title = args["title"]; question = args["question"]
    require(isinstance(title, str) and 0 < len(title.strip()) <= 160)
    require(isinstance(question, str) and 0 < len(question.strip()) <= 2000)
    requested = args["sources"]
    require(isinstance(requested, list) and 1 <= len(requested) <= 4)
    seen_urls = set(); seen_labels = set(); checked_sources = []
    for index, item in enumerate(requested, 1):
        exact(item, ("url", "label")); url = item["url"]; label = item["label"]
        public_url(url)
        require(isinstance(label, str) and 0 < len(label.strip()) <= 200
                and "/" not in label and "\\" not in label)
        require(url not in seen_urls and label not in seen_labels, "BROWSER_DUPLICATE_SOURCE")
        seen_urls.add(url); seen_labels.add(label)
        checked_sources.append({"id": f"source-{index:02}", "url": url, "label": label.strip()})

    executable = tools.get("chrome")
    require(isinstance(executable, str) and Path(executable).is_absolute(), "BROWSER_CHROME_REQUIRED")
    browser = Browser(executable, True)
    try:
        prepare_browser(browser)
        pages = []
        for source_record in checked_sources:
            start = len(browser.sources)
            browser.call("Page.navigate", {"url": source_record["url"]})
            wait_for_page(browser)
            document = browser.evaluate("(() => ({title:document.title.slice(0,200),url:location.href,text:(document.body?.innerText||'').slice(0,64000)}))()")
            require(isinstance(document, dict) and isinstance(document.get("text"), str)
                    and document["text"].strip(), "BROWSER_OUTPUT_INVALID")
            public_url(document["url"])
            page_requests = browser.sources[start:]
            response = next((item for item in reversed(page_requests) if item["url"] == document["url"]), None)
            require(response is not None, "BROWSER_SOURCE_UNAVAILABLE")
            observed = datetime.now(timezone.utc).isoformat()
            page = dict(source_record)
            page.update({"requestedUrl": source_record["url"], "url": document["url"],
                         "pageTitle": document.get("title", ""), "accessedAt": observed,
                         "responseSha256": response["sha256"],
                         "textSha256": hashlib.sha256(document["text"].encode()).hexdigest(),
                         "textBytes": len(document["text"].encode()), "text": document["text"],
                         "possiblyTruncated": len(document["text"]) == 64000})
            pages.append(page)

        by_url = {item["requestedUrl"]: item for item in pages}
        claims = args["claims"]
        require(isinstance(claims, list) and 1 <= len(claims) <= 100, "BROWSER_RESEARCH_CLAIM_LIMIT")
        checked_claims = []; seen_claims = set(); used_sources = set(); total_evidence = 0
        for claim_index, item in enumerate(claims, 1):
            exact(item, ("claim", "classification", "evidence", "note"))
            statement = item["claim"]; classification = item["classification"]
            require(isinstance(statement, str) and 0 < len(statement.strip()) <= 2000
                    and statement not in seen_claims, "BROWSER_RESEARCH_CLAIM_INVALID")
            require(classification in ("fact", "inference", "unknown"), "BROWSER_RESEARCH_CLASSIFICATION_INVALID")
            seen_claims.add(statement); evidence = item["evidence"]; note = item["note"]
            require(isinstance(evidence, list) and len(evidence) <= 8, "BROWSER_RESEARCH_EVIDENCE_LIMIT")
            if classification == "unknown":
                require(not evidence and isinstance(note, str) and 0 < len(note.strip()) <= 1000,
                        "BROWSER_RESEARCH_UNKNOWN_INVALID")
                checked_evidence = []; checked_note = note.strip()
            else:
                require(evidence, "BROWSER_RESEARCH_EVIDENCE_REQUIRED")
                if classification == "fact":
                    require(note is None, "BROWSER_RESEARCH_NOTE_INVALID"); checked_note = None
                else:
                    require(isinstance(note, str) and 0 < len(note.strip()) <= 1000,
                            "BROWSER_RESEARCH_NOTE_INVALID"); checked_note = note.strip()
                checked_evidence = []; seen_evidence = set()
                for entry in evidence:
                    exact(entry, ("source", "quote")); source_url = entry["source"]
                    require(source_url in by_url, "BROWSER_RESEARCH_SOURCE_INVALID")
                    quote = entry["quote"]
                    require(isinstance(quote, str) and 0 < len(quote.strip()) <= 4000,
                            "BROWSER_RESEARCH_EVIDENCE_REQUIRED")
                    page = by_url[source_url]; start = page["text"].find(quote)
                    require(start >= 0, "BROWSER_RESEARCH_EVIDENCE_MISMATCH")
                    pair = (page["id"], quote)
                    require(pair not in seen_evidence, "BROWSER_RESEARCH_EVIDENCE_DUPLICATE")
                    seen_evidence.add(pair); used_sources.add(source_url); total_evidence += 1
                    require(total_evidence <= 500, "BROWSER_RESEARCH_EVIDENCE_LIMIT")
                    byte_start = len(page["text"][:start].encode())
                    checked_evidence.append({"sourceId": page["id"], "sourceLabel": page["label"],
                                             "sourceUrl": page["url"], "accessedAt": page["accessedAt"],
                                             "quote": quote, "line": page["text"].count("\n", 0, start) + 1,
                                             "startByte": byte_start,
                                             "endByte": byte_start + len(quote.encode())})
            checked_claims.append({"index": claim_index, "claim": statement.strip(),
                                   "classification": classification, "evidence": checked_evidence,
                                   "note": checked_note})
        require(used_sources == set(by_url), "BROWSER_RESEARCH_UNUSED_SOURCE")

        public_sources = [{key: page[key] for key in ("id", "label", "requestedUrl", "url", "pageTitle",
                          "accessedAt", "responseSha256", "textSha256", "textBytes", "possiblyTruncated")}
                          for page in pages]
        result = {"kind": "web_research", "title": title.strip(), "question": question.strip(),
                  "sources": public_sources, "claims": checked_claims,
                  "summary": {"sources": len(public_sources), "claims": len(checked_claims),
                              "facts": sum(item["classification"] == "fact" for item in checked_claims),
                              "inferences": sum(item["classification"] == "inference" for item in checked_claims),
                              "unknowns": sum(item["classification"] == "unknown" for item in checked_claims)},
                  "file": "research-report.md"}
        markdown = lambda value: ''.join(('\\' + character) if character in '\\`*_{}[]()#+-.!|>' else character
                                         for character in html.escape(str(value), quote=True).replace('\r', ' ').replace('\n', ' '))
        report = ['# ' + markdown(result["title"]), '', '**Research question:** ' + markdown(result["question"]), '',
                  '> Classifications are reviewable labels, not automatic truth scores. Quotes, access times and response hashes are verified against this run.', '',
                  '## Retrieved sources', '']
        report.extend(f'- **{markdown(source["id"])} · {markdown(source["label"])}** · {markdown(source["url"])} · accessed {markdown(source["accessedAt"])} · response `{source["responseSha256"]}` · text `{source["textSha256"]}`'
                      for source in public_sources)
        for claim in checked_claims:
            report.extend(('', f'## Claim {claim["index"]} · {claim["classification"].title()}', '', markdown(claim["claim"])))
            if claim["note"] is not None:
                report.extend(('', '**Review note:** ' + markdown(claim["note"])))
            for evidence_item in claim["evidence"]:
                report.extend(('', f'**{markdown(evidence_item["sourceId"])} · {markdown(evidence_item["sourceLabel"])} · accessed {markdown(evidence_item["accessedAt"])} · line {evidence_item["line"]} · bytes {evidence_item["startByte"]}–{evidence_item["endByte"]}**',
                               '', f'> {markdown(evidence_item["quote"])}'))
        out = ROOT / "outputs"; out.mkdir(exist_ok=True)
        (out / "research-report.md").write_text('\n'.join(report) + '\n', encoding="utf-8")
        csv_cell = lambda value: "'" + value if str(value).lstrip().startswith(("=", "+", "-", "@")) else value
        with (out / "sources.csv").open("w", encoding="utf-8", newline="") as handle:
            writer = csv.writer(handle); writer.writerow(("claim", "source_url", "accessed_at", "evidence_location"))
            for claim in checked_claims:
                if not claim["evidence"]:
                    writer.writerow((csv_cell(claim["claim"]), "", "", "unknown: " + claim["note"]))
                for evidence_item in claim["evidence"]:
                    writer.writerow((csv_cell(claim["claim"]), evidence_item["sourceUrl"], evidence_item["accessedAt"],
                                     f'line {evidence_item["line"]}; bytes {evidence_item["startByte"]}-{evidence_item["endByte"]}'))
        (out / "research-evidence.json").write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
        return result
    finally:
        browser.close()


def run(request):
    operation, args, tools = request["operation"], request["arguments"], request["tools"]
    if operation == "browser.research":
        return web_research(args, tools)
    require(operation in ("browser.inspect", "browser.draft") and isinstance(args, dict))
    require(set(args) <= {"url", "source", "fields"} and (("url" in args) != ("source" in args)))
    fields = args.get("fields", [])
    require(isinstance(fields, list) and len(fields) <= 20 and (operation != "browser.draft" or len(fields) > 0))
    require(operation != "browser.inspect" or not fields)
    for field in fields:
        require(isinstance(field, dict) and set(field) == {"selector", "value"}
                and isinstance(field["selector"], str) and 0 < len(field["selector"]) <= 512
                and isinstance(field["value"], str) and len(field["value"]) <= 4096
                and "\0" not in field["selector"] + field["value"])
    source = None
    if "url" in args:
        public_url(args["url"])
    else:
        path = Path(args["source"])
        require(not path.is_absolute() and path.parts and path.parts[0] == "inputs" and ".." not in path.parts)
        path = (ROOT / path).resolve()
        require(path.is_relative_to(ROOT / "inputs") and path.is_file() and path.stat().st_size <= 1024 * 1024)
        source = path.read_text(encoding="utf-8")
    executable = tools.get("chrome")
    require(isinstance(executable, str) and Path(executable).is_absolute(), "BROWSER_CHROME_REQUIRED")
    browser = Browser(executable, source is None)
    try:
        prepare_browser(browser)
        if source is None:
            browser.call("Page.navigate", {"url": args["url"]})
        else:
            frame = browser.call("Page.getFrameTree")["frameTree"]["frame"]["id"]
            browser.call("Page.setDocumentContent", {"frameId": frame, "html": source})
        browser.call("Emulation.setDefaultBackgroundColorOverride", {"color": {"r": 255, "g": 255, "b": 255, "a": 1}})
        wait_for_page(browser)
        if fields:
            result = browser.evaluate("(() => { const fields=" + json.dumps(fields, ensure_ascii=True) + "; const done=[]; for(const f of fields){ const e=document.querySelector(f.selector); if(!e || !['INPUT','TEXTAREA','SELECT'].includes(e.tagName) || e.disabled || ['password','file','hidden','submit','button','reset','image'].includes(e.type)) return {error:'BROWSER_FIELD_DENIED'}; const r=e.getBoundingClientRect(); if(!r.width||!r.height) return {error:'BROWSER_FIELD_DENIED'}; if(e.type==='checkbox'||e.type==='radio')e.checked=f.value==='true'; else {e.value=f.value;if(e.value!==f.value)return {error:'BROWSER_FIELD_VALUE_INVALID'};} done.push({selector:f.selector,value:e.type==='checkbox'||e.type==='radio'?String(e.checked):e.value}); } return {fields:done}; })()")
            require(isinstance(result, dict) and "error" not in result, (result or {}).get("error", "BROWSER_FIELD_DENIED"))
        # Fixed inspection code only; no caller-provided JavaScript is executed.
        document = browser.evaluate("(() => ({title:document.title.slice(0,200),url:location.href,text:(document.body?.innerText||'').slice(0,64000),links:Array.from(document.querySelectorAll('a[href]')).slice(0,100).map(a=>({label:(a.innerText||'').slice(0,160),url:a.href})),fields:Array.from(document.querySelectorAll('input,textarea,select')).filter(e=>!['password','file','hidden'].includes(e.type)).slice(0,100).map(e=>({name:e.name,type:e.type||e.tagName.toLowerCase(),value:(e.type==='checkbox'||e.type==='radio')?String(e.checked):String(e.value).slice(0,4096)}))}))()")
        require(isinstance(document, dict), "BROWSER_OUTPUT_INVALID")
        if source is None:
            public_url(document["url"])
            require(browser.sources, "BROWSER_SOURCE_UNAVAILABLE")
        else:
            document["url"] = "frozen-resource"
        document["operation"] = operation
        document["observedAt"] = datetime.now(timezone.utc).isoformat()
        document["sources"] = browser.sources
        document["submitted"] = False
        document["pageScriptsEnabled"] = False
        screenshot = browser.call("Page.captureScreenshot", {"format": "png", "captureBeyondViewport": False})
        pixels = base64.b64decode(screenshot["data"], validate=True)
        require(pixels.startswith(b"\x89PNG\r\n\x1a\n") and len(pixels) <= 4 * 1024 * 1024, "BROWSER_OUTPUT_LIMIT")
        out = ROOT / "outputs"
        out.mkdir(exist_ok=True)
        (out / "browser.png").write_bytes(pixels)
        (out / "browser.json").write_text(json.dumps(document, ensure_ascii=False, indent=2), encoding="utf-8")
        (out / "browser-source.txt").write_text(document["text"], encoding="utf-8")
        content = "<h1>" + html.escape(document["title"]) + "</h1><pre>" + html.escape(document["text"]) + "</pre>"
        content += "<h2>Form draft — not submitted</h2><pre>" + html.escape(json.dumps(document["fields"], ensure_ascii=False, indent=2)) + "</pre>"
        (out / "browser.html").write_text('<!doctype html><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src \'none\'; connect-src \'none\'; base-uri \'none\'; form-action \'none\'"><title>Browser source</title>' + content, encoding="utf-8")
        return {"kind": "browser", "title": document["title"], "url": document["url"], "submitted": False, "requests": len(browser.sources)}
    finally:
        browser.close()


if __name__ == "__main__":
    try:
        request = json.loads(sys.stdin.buffer.read(2 * 1024 * 1024 + 1))
        result = run(request)
    except (ValueError, OSError, KeyError, TypeError, http.client.HTTPException, json.JSONDecodeError) as error:
        code = str(error)
        result = {"error": code if code.startswith("BROWSER_") and code.replace("_", "").isupper() else "BROWSER_FAILED"}
    print(json.dumps(result, ensure_ascii=False))
