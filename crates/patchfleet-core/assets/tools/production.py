"""Moxcrew's bounded, typed local production worker. Python standard library only."""
import csv
import io
import itertools
import json
import math
from pathlib import Path
import re
import runpy
import statistics
import subprocess
import tempfile
import sys
import xml.etree.ElementTree as ET
from xml.sax.saxutils import escape
import zipfile

ROOT = Path.cwd().resolve()
OUT = ROOT / "outputs"
MAX_FILE = 25 * 1024 * 1024
NS = {"a": "http://schemas.openxmlformats.org/drawingml/2006/main",
      "p": "http://schemas.openxmlformats.org/presentationml/2006/main",
      "w": "http://schemas.openxmlformats.org/wordprocessingml/2006/main"}
REL = "http://schemas.openxmlformats.org/package/2006/relationships"
OFFICE = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
CONTENT = "http://schemas.openxmlformats.org/package/2006/content-types"


def require(ok, code="PRODUCTION_INVALID_INPUT"):
    if not ok:
        raise ValueError(code)


def exact(obj, required=(), optional=()):
    require(isinstance(obj, dict) and set(required) <= set(obj)
            and set(obj) <= set(required) | set(optional))


def text(value, limit=10000):
    require(isinstance(value, str) and len(value) <= limit and "\0" not in value)
    require(all(ord(c) >= 32 or c in "\n\r\t" for c in value))
    return value


def number(value, low, high):
    require(type(value) in (int, float) and math.isfinite(value) and low <= value <= high)
    return value


def source(name):
    text(name, 1024)
    path = Path(name)
    require(not path.is_absolute() and path.parts and path.parts[0] == "inputs"
            and all(p not in ("..", ".") for p in path.parts))
    resolved = (ROOT / path).resolve()
    require(resolved.is_relative_to(ROOT / "inputs") and resolved.is_file()
            and resolved.stat().st_size <= MAX_FILE)
    return resolved


def xml(data):
    require(len(data) <= 8 * 1024 * 1024 and b"<!DOCTYPE" not in data.upper()
            and b"<!ENTITY" not in data.upper(), "PRODUCTION_UNSAFE_DOCUMENT")
    return ET.fromstring(data)


def read_zip(path):
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        require(len(infos) <= 4096 and sum(i.file_size for i in infos) <= 64 * 1024 * 1024,
                "PRODUCTION_DOCUMENT_LIMIT")
        require(len({i.filename for i in infos}) == len(infos))
        require(all(not i.filename.startswith("/") and ".." not in Path(i.filename).parts
                    and not i.flag_bits & 1 for i in infos), "PRODUCTION_UNSAFE_DOCUMENT")
        result = {i.filename: archive.read(i) for i in infos}
        for name, data in result.items():
            if name.endswith((".xml", ".rels")):
                node = xml(data)
                require(not any(n.attrib.get("TargetMode") == "External" for n in node.iter()),
                        "PRODUCTION_EXTERNAL_REFERENCE")
        return result


def write_zip(name, parts):
    path = OUT / name
    with zipfile.ZipFile(path, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for key, data in parts.items():
            archive.writestr(key, data)
    read_zip(path)
    return path


def rels(entries):
    return f'<Relationships xmlns="{REL}">' + "".join(
        f'<Relationship Id="rId{i}" Type="{OFFICE}/{kind}" Target="{escape(target)}"/>'
        for i, kind, target in entries) + '</Relationships>'


def content_types(entries):
    return f'<Types xmlns="{CONTENT}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>' + ''.join(
        f'<Override PartName="/{name}" ContentType="application/vnd.openxmlformats-officedocument.{kind}+xml"/>'
        for name, kind in entries) + '</Types>'


def slide_xml(title, bullets):
    def shape(ident, y, h, lines, size):
        paras = ''.join(f'<a:p><a:pPr/><a:r><a:rPr lang="en-US" sz="{size}"/><a:t>{escape(line)}</a:t></a:r></a:p>' for line in lines)
        return f'<p:sp><p:nvSpPr><p:cNvPr id="{ident}" name="Text {ident}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="700000" y="{y}"/><a:ext cx="10800000" cy="{h}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/></p:spPr><p:txBody><a:bodyPr wrap="square"/><a:lstStyle/>{paras}</p:txBody></p:sp>'
    group = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
    return f'<p:sld xmlns:a="{NS["a"]}" xmlns:p="{NS["p"]}"><p:cSld><p:spTree>{group}{shape(2,500000,1000000,[title],3200)}{shape(3,1700000,4500000,bullets,2200)}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>'


def presentations(args, revise):
    if revise:
        exact(args, ("source", "sectionId", "title", "bullets"))
        parts = read_zip(source(args["source"]))
        match = re.fullmatch(r"slide:([1-9][0-9]?)", text(args["sectionId"], 20))
        require(match is not None)
        name = f'ppt/slides/slide{int(match[1])}.xml'
        require(name in parts, "PRODUCTION_SECTION_NOT_FOUND")
        slides = [args]
    else:
        exact(args, ("slides",))
        slides = args["slides"]
        require(isinstance(slides, list) and 1 <= len(slides) <= 60)
        parts = {"_rels/.rels": rels([(1, "officeDocument", "ppt/presentation.xml")])}
    for slide in slides:
        exact(slide, ("title", "bullets"), ("source", "sectionId"))
        title = text(slide["title"], 160)
        bullets = slide["bullets"]
        require(isinstance(bullets, list) and len(bullets) <= 8)
        for bullet in bullets:
            text(bullet, 300)
        target = name if revise else f'ppt/slides/slide{len([n for n in parts if re.fullmatch(r"ppt/slides/slide[0-9]+.xml", n)]) + 1}.xml'
        parts[target] = slide_xml(title, bullets)
    if not revise:
        entries = [("ppt/presentation.xml", "presentationml.presentation.main")]
        entries += [(f'ppt/slides/slide{i}.xml', 'presentationml.slide') for i in range(1, len(slides)+1)]
        parts["[Content_Types].xml"] = content_types(entries)
        ids = ''.join(f'<p:sldId id="{255+i}" r:id="rId{i}"/>' for i in range(1,len(slides)+1))
        parts["ppt/presentation.xml"] = f'<p:presentation xmlns:p="{NS["p"]}" xmlns:r="{OFFICE}"><p:sldIdLst>{ids}</p:sldIdLst><p:sldSz cx="12192000" cy="6858000" type="screen16x9"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>'
        parts["ppt/_rels/presentation.xml.rels"] = rels([(i,"slide",f'slides/slide{i}.xml') for i in range(1,len(slides)+1)])
    write_zip("presentation.pptx", parts)


def documents(args, revise):
    if revise:
        exact(args, ("source", "sectionId", "text"))
        parts = read_zip(source(args["source"]))
        node = xml(parts["word/document.xml"])
        paragraphs = node.findall('.//w:body/w:p', NS)
        match = re.fullmatch(r"paragraph:([1-9][0-9]{0,3})", text(args["sectionId"],30))
        require(match is not None and int(match[1]) <= len(paragraphs), "PRODUCTION_SECTION_NOT_FOUND")
        paragraph = paragraphs[int(match[1])-1]
        for child in list(paragraph):
            if child.tag != f'{{{NS["w"]}}}pPr': paragraph.remove(child)
        run = ET.SubElement(paragraph, f'{{{NS["w"]}}}r')
        ET.SubElement(run, f'{{{NS["w"]}}}t').text = text(args["text"])
        parts["word/document.xml"] = ET.tostring(node,encoding="utf-8",xml_declaration=True)
    else:
        exact(args, ("paragraphs",), ("title",))
        paragraphs = args["paragraphs"]
        require(isinstance(paragraphs,list) and 1 <= len(paragraphs) <= 500)
        lines = ([text(args["title"],160)] if args.get("title") else []) + [text(p) for p in paragraphs]
        body = ''.join(f'<w:p><w:r><w:t xml:space="preserve">{escape(p)}</w:t></w:r></w:p>' for p in lines)
        parts = {"[Content_Types].xml":content_types([("word/document.xml","wordprocessingml.document.main")]),
                 "_rels/.rels":rels([(1,"officeDocument","word/document.xml")]),
                 "word/document.xml":f'<w:document xmlns:w="{NS["w"]}"><w:body>{body}<w:sectPr><w:pgSz w="11906" h="16838"/><w:pgMar top="1440" right="1440" bottom="1440" left="1440"/></w:sectPr></w:body></w:document>'}
    write_zip("document.docx", parts)


def data_operation(args, operation):
    exact(args, ("source",), ("deduplicate", "numericColumns"))
    data = source(args["source"]).read_text(encoding="utf-8-sig")
    rows = list(itertools.islice(csv.reader(io.StringIO(data)), 100002))
    require(1 <= len(rows) <= 100001 and 1 <= len(rows[0]) <= 100)
    require(sum(len(row) for row in rows)<=200000,"PRODUCTION_DATA_LIMIT")
    require(all(len(row)==len(rows[0]) for row in rows),"PRODUCTION_CSV_SHAPE_INVALID")
    for row in rows:
        for cell in row: text(cell,20000)
    cleaned = [[cell.strip() for cell in row] for row in rows]
    if args.get("deduplicate",False):
        require(type(args["deduplicate"]) is bool)
        seen=set();cleaned=cleaned[:1]+[row for row in cleaned[1:] if not (tuple(row) in seen or seen.add(tuple(row)))]
    if operation == 'data.xlsx':
        def column(n):
            label=''
            while n: n,r=divmod(n-1,26);label=chr(65+r)+label
            return label
        body=''.join(f'<row r="{i}">'+''.join(f'<c r="{column(j)}{i}" t="inlineStr"><is><t xml:space="preserve">{escape(cell)}</t></is></c>' for j,cell in enumerate(row,1))+'</row>' for i,row in enumerate(cleaned,1))
        write_zip('data.xlsx',{'[Content_Types].xml':content_types([('xl/workbook.xml','spreadsheetml.sheet.main'),('xl/worksheets/sheet1.xml','spreadsheetml.worksheet')]),'_rels/.rels':rels([(1,'officeDocument','xl/workbook.xml')]),'xl/workbook.xml':f'<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{OFFICE}"><sheets><sheet name="Data" sheetId="1" r:id="rId1"/></sheets></workbook>','xl/_rels/workbook.xml.rels':rels([(1,'worksheet','worksheets/sheet1.xml')]),'xl/worksheets/sheet1.xml':f'<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{body}</sheetData></worksheet>'})
    else:
        with (OUT/'data.csv').open('x',newline='',encoding='utf-8') as f: csv.writer(f).writerows(cleaned)
    stats={"inputRows":len(rows)-1,"outputRows":len(cleaned)-1,"removedDuplicates":len(rows)-len(cleaned),"columns":{}}
    columns=args.get('numericColumns',[])
    require(isinstance(columns,list) and len(columns)<=len(rows[0]))
    for name in columns:
        require(name in cleaned[0]);index=cleaned[0].index(name);values=[];missing=0
        for row in cleaned[1:]:
            if not row[index]: missing+=1;continue
            try: value=float(row[index])
            except ValueError: raise ValueError('PRODUCTION_NON_NUMERIC_VALUE')
            require(math.isfinite(value));values.append(value)
        stats['columns'][name]={"count":len(values),"missing":missing,"sum":math.fsum(values),"mean":statistics.mean(values) if values else None,"min":min(values) if values else None,"max":max(values) if values else None}
    (OUT/'analysis.json').write_text(json.dumps(stats,ensure_ascii=False,allow_nan=False),encoding='utf-8')


def run_tool(tools, name, args):
    executable=tools.get(name)
    require(isinstance(executable,str) and Path(executable).is_absolute(),'PRODUCTION_TOOL_UNAVAILABLE')
    with tempfile.TemporaryFile() as output:
        def limits():
            import resource
            resource.setrlimit(resource.RLIMIT_FSIZE,(MAX_FILE,MAX_FILE))
        result=subprocess.run([executable,*args],stdout=output,stderr=subprocess.DEVNULL,
                              timeout=90,preexec_fn=limits if sys.platform!='win32' else None)
        require(result.returncode==0,'PRODUCTION_TOOL_FAILED')
        output.seek(0);data=output.read(1024*1024+1)
        require(len(data)<=1024*1024,'PRODUCTION_OUTPUT_LIMIT')
        return data


def input_format(path):
    formats={'.mp4':'mov','.mov':'mov','.m4a':'mov','.wav':'wav','.mp3':'mp3',
             '.mkv':'matroska','.webm':'matroska','.png':'png_pipe','.jpg':'jpeg_pipe',
             '.jpeg':'jpeg_pipe','.webp':'webp_pipe'}
    demuxer=formats.get(path.suffix.lower())
    require(demuxer is not None,'PRODUCTION_MEDIA_FORMAT_UNSUPPORTED')
    # Never autodetect a playlist or follow a movie's external data references.
    return ['-protocol_whitelist','file,pipe','-format_whitelist',','.join(sorted(set(formats.values()))),
            '-f',demuxer]+(['-enable_drefs','0','-use_absolute_path','0'] if demuxer=='mov' else [])


def media_info(tools,path):
    raw=run_tool(tools,'ffprobe',['-v','error',*input_format(path),'-show_entries','format=duration:stream=codec_type,width,height,sample_rate,channels','-of','json',str(path)])
    return json.loads(raw)


def media(args,operation,tools):
    common=['-nostdin','-hide_banner','-loglevel','error','-n','-protocol_whitelist','file,pipe']
    if operation=='images.resize':
        exact(args,('source','width','height'))
        w=int(number(args['width'],1,4096));h=int(number(args['height'],1,4096))
        run_tool(tools,'ffmpeg',common+input_format(source(args['source']))+['-i',str(source(args['source'])),'-vf',f'scale={w}:{h}:force_original_aspect_ratio=decrease','-frames:v','1',str(OUT/'image.png')])
    elif operation=='audio.cleanup':
        exact(args,('source',))
        input_path=source(args['source']);before=media_info(tools,input_path)
        duration=float(before.get('format',{}).get('duration',0))
        require(0<duration<=1200,'PRODUCTION_DURATION_LIMIT')
        require(any(s.get('codec_type')=='audio' for s in before.get('streams',[])), 'PRODUCTION_AUDIO_REQUIRED')
        run_tool(tools,'ffmpeg',common+input_format(input_path)+['-i',str(input_path),'-vn','-af','highpass=f=80,afftdn,loudnorm=I=-16:TP=-1.5:LRA=11','-ar','24000','-ac','1','-c:a','pcm_s16le',str(OUT/'audio.wav')])
    elif operation=='video.edit':
        exact(args,('clips',),('width','height','captions'))
        require(isinstance(args['clips'],list) and 1<=len(args['clips'])<=12)
        w=int(number(args.get('width',1280),16,3840));h=int(number(args.get('height',720),16,2160));require(w%2==0 and h%2==0)
        pieces=[];duration=0;audio_shape=None
        for i,clip in enumerate(args['clips']):
            exact(clip,('source','start','end'))
            start=number(clip['start'],0,86400);end=number(clip['end'],start+0.02,start+1200);duration+=end-start
            require(duration<=1200)
            input_path=source(clip['source']);before=media_info(tools,input_path)
            require(any(s.get('codec_type')=='video' for s in before.get('streams',[])), 'PRODUCTION_VIDEO_REQUIRED')
            require(end<=float(before.get('format',{}).get('duration',0))+0.02,'PRODUCTION_CLIP_RANGE_INVALID')
            has_audio=any(s.get('codec_type')=='audio' for s in before.get('streams',[]))
            if audio_shape is None: audio_shape=has_audio
            require(audio_shape==has_audio,'PRODUCTION_MIXED_AUDIO_UNSUPPORTED')
            name=f'part-{i}.mp4';pieces.append(name)
            run_tool(tools,'ffmpeg',common+input_format(input_path)+['-ss',str(start),'-i',str(input_path),'-t',str(end-start),'-vf',f'scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,fps=30','-c:v','mpeg4','-q:v','3','-c:a','aac','-ar','48000','-ac','2',str(ROOT/name)])
        (ROOT/'concat.txt').write_text(''.join(f"file '{name}'\n" for name in pieces))
        run_tool(tools,'ffmpeg',common+['-f','concat','-safe','1','-i',str(ROOT/'concat.txt'),'-c','copy',str(OUT/'video.mp4')])
        if 'captions' in args:
            captions=args['captions'];require(isinstance(captions,list) and len(captions)<=1000)
            def stamp(v):
                ms=round(v*1000);return f'{ms//3600000:02}:{ms//60000%60:02}:{ms//1000%60:02},{ms%1000:03}'
            lines=[];last=0
            for i,cap in enumerate(captions,1):
                exact(cap,('start','end','text'));start=number(cap['start'],last,duration);end=number(cap['end'],start+0.001,duration);last=end
                lines.append(f"{i}\n{stamp(start)} --> {stamp(end)}\n{text(cap['text'],500)}\n")
            (OUT/'captions.srt').write_text('\n'.join(lines),encoding='utf-8')
        (OUT/'edit-recipe.json').write_text(json.dumps(args,ensure_ascii=False),encoding='utf-8')
    else: raise ValueError('PRODUCTION_OPERATION_UNSUPPORTED')
    result=next(p for p in OUT.iterdir() if p.suffix in ('.png','.wav','.mp4'))
    info=media_info(tools,result)
    require(info.get('streams'),'PRODUCTION_OUTPUT_INVALID')
    if operation in ('audio.cleanup','video.edit'):
        actual=float(info.get('format',{}).get('duration',0))
        require(abs(actual-duration)<max(0.2,duration*0.005),'PRODUCTION_DURATION_MISMATCH')
    (OUT/'validation.json').write_text(json.dumps(info),encoding='utf-8')


def inspect(args,tools):
    exact(args,('source',))
    path=source(args['source']);suffix=path.suffix.lower()
    if suffix in ('.md','.txt','.csv','.json','.srt'):
        content=path.read_text(encoding='utf-8-sig')
        return {'kind':'text','text':content[:150000],'truncated':len(content)>150000}
    if suffix in ('.pptx','.docx'):
        parts=read_zip(path);pages=[]
        if suffix=='.pptx':
            names=sorted((n for n in parts if re.fullmatch(r'ppt/slides/slide[0-9]+.xml',n)),key=lambda n:int(re.search(r'slide([0-9]+)',n)[1]))
            require(1<=len(names)<=60)
            for name in names:
                lines=[n.text or '' for n in xml(parts[name]).findall('.//a:t',NS)]
                pages.append({'id':f'slide:{re.search(r"slide([0-9]+)",name)[1]}','title':lines[0] if lines else '', 'text':'\n'.join(lines[1:])[:20000]})
        else:
            paragraphs=xml(parts['word/document.xml']).findall('.//w:body/w:p',NS)
            for i,node in enumerate(paragraphs[:500],1):
                pages.append({'id':f'paragraph:{i}','title':f'Paragraph {i}','text':''.join(n.text or '' for n in node.findall('.//w:t',NS))[:20000]})
        return {'kind':'pages','pages':pages,'fidelity':'text_outline'}
    if suffix in ('.mp4','.mov','.mkv','.wav','.mp3','.m4a','.webm'):
        info=media_info(tools,path)
        preview={'kind':'media','metadata':info,'durationSeconds':float(info.get('format',{}).get('duration',0))}
        if any(s.get('codec_type')=='video' for s in info.get('streams',[])):
            run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(path),'-i',str(path),'-frames:v','1','-vf','scale=800:-2',str(OUT/'preview.png')])
        return preview
    if suffix=='.pdf':
        content=run_tool(tools,'pdftotext',['-f','1','-l','100','-enc','UTF-8','-layout',str(path),'-']).decode('utf-8')
        return {'kind':'text','text':content[:150000],'fidelity':'text_extraction','truncated':len(content)>150000}
    raise ValueError('PRODUCTION_PREVIEW_UNSUPPORTED')


def main(request):
    exact(request,('operation','arguments','tools'))
    OUT.mkdir(exist_ok=True)
    operation=request['operation'];args=request['arguments'];tools=request['tools']
    if operation in ('images.verify','images.batch','images.mask','resources.ocr','audio.transcribe','audio.podcast','data.chart','data.scenario','data.extract','documents.brief','documents.template','documents.render','video.frame','video.package','interactive.create'):
        extras=runpy.run_path(str(ROOT/'production_capabilities.py'),init_globals=globals())
        result=extras['execute'](operation,args,tools)
        files=[{'name':p.name,'bytes':p.stat().st_size} for p in sorted(OUT.iterdir()) if p.is_file()]
        require(files and len(files)<=16 and sum(f['bytes'] for f in files)<=MAX_FILE,'PRODUCTION_OUTPUT_LIMIT')
        result.update(files=files,verified=True)
        return result
    if operation=='inspect': return inspect(args,tools)
    if operation=='resources.extract':
        result=inspect(args,tools)
        require(result.get('kind') in ('pages','text'),'PRODUCTION_PREVIEW_UNSUPPORTED')
        (OUT/'outline.json').write_text(json.dumps(result,ensure_ascii=False),encoding='utf-8')
        (OUT/'extracted.txt').write_text(result.get('text','\n\n'.join(p['title']+'\n'+p['text'] for p in result.get('pages',[]))),encoding='utf-8')
        return {'files':[{'name':p.name,'bytes':p.stat().st_size} for p in OUT.iterdir()],'verified':True}
    if operation in ('presentations.create','presentations.revise'): presentations(args,operation=='presentations.revise')
    elif operation in ('documents.create','documents.revise'): documents(args,operation=='documents.revise')
    elif operation in ('data.clean','data.analyze','data.xlsx'): data_operation(args,operation)
    elif operation in ('images.resize','video.edit','audio.cleanup'): media(args,operation,tools)
    else: raise ValueError('PRODUCTION_OPERATION_UNSUPPORTED')
    files=[{'name':p.name,'bytes':p.stat().st_size} for p in sorted(OUT.iterdir()) if p.is_file()]
    require(files and len(files)<=16 and sum(f['bytes'] for f in files)<=MAX_FILE,'PRODUCTION_OUTPUT_LIMIT')
    return {'files':files,'verified':True}


if __name__=='__main__':
    try:
        raw=sys.stdin.buffer.read(512*1024+1);require(len(raw)<=512*1024)
        print(json.dumps(main(json.loads(raw)),ensure_ascii=False,allow_nan=False))
    except Exception as error:
        code=str(error) if isinstance(error,ValueError) and str(error).startswith('PRODUCTION_') else 'PRODUCTION_FAILED'
        print(json.dumps({'error':code}))
