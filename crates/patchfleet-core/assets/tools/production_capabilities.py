"""Fixed production capabilities loaded only from the bundled worker's private scratch."""
import base64
import binascii
import csv
import datetime
import difflib
import hashlib
import html
import io
import itertools
import math
import struct
import textwrap
import urllib.parse
import zlib


def file_sha256(path):
    digest=hashlib.sha256()
    with path.open('rb') as handle:
        for block in iter(lambda:handle.read(1024*1024),b''):
            digest.update(block)
    return digest.hexdigest()


def png(data, expected=None):
    require(8 <= len(data) <= MAX_FILE and data[:8] == b'\x89PNG\r\n\x1a\n', 'PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
    offset=8; chunks=[]; compressed=bytearray(); dimensions=None; ended=False; palette=0; transparent=False; idat=False; after_idat=False
    while offset < len(data):
        require(offset+12 <= len(data), 'PRODUCTION_IMAGE_INVALID')
        size=struct.unpack('>I',data[offset:offset+4])[0]; kind=data[offset+4:offset+8]
        require(size <= MAX_FILE and offset+size+12 <= len(data), 'PRODUCTION_IMAGE_INVALID')
        body=data[offset+8:offset+8+size]; checksum=struct.unpack('>I',data[offset+8+size:offset+12+size])[0]
        require(binascii.crc32(kind+body)&0xffffffff == checksum, 'PRODUCTION_IMAGE_INVALID')
        require(all(65<=c<=90 or 97<=c<=122 for c in kind) and kind[2]&32==0, 'PRODUCTION_IMAGE_INVALID')
        require(not ended and kind not in (b'acTL',b'fcTL',b'fdAT'), 'PRODUCTION_IMAGE_INVALID')
        if idat and kind!=b'IDAT': after_idat=True
        if kind==b'IHDR':
            require(dimensions is None and offset==8 and size==13, 'PRODUCTION_IMAGE_INVALID')
            w,h,depth,color,compression,filtering,interlace=struct.unpack('>IIBBBBB',body)
            require(0<w<=4096 and 0<h<=4096 and depth==8 and color in (0,2,3,4,6)
                    and compression==filtering==interlace==0, 'PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
            if expected: require((w,h)==expected, 'PRODUCTION_IMAGE_DIMENSIONS_MISMATCH')
            dimensions=(w,h); channels={0:1,2:3,3:1,4:2,6:4}[color]
        elif kind==b'IDAT':
            require(dimensions is not None and not after_idat and (color!=3 or palette), 'PRODUCTION_IMAGE_INVALID')
            compressed.extend(body);idat=True
        elif kind==b'PLTE':
            require(dimensions is not None and not idat and not palette and color in (2,3,6) and 0<size<=768 and size%3==0, 'PRODUCTION_IMAGE_INVALID');palette=size//3
        elif kind==b'tRNS':
            require(dimensions is not None and not idat and not transparent and
                    ((color==0 and size==2) or (color==2 and size==6) or (color==3 and 0<size<=palette)), 'PRODUCTION_IMAGE_INVALID')
            transparent=True
        elif kind==b'IEND':
            require(size==0 and offset+12==len(data), 'PRODUCTION_IMAGE_INVALID'); ended=True
        elif kind[0]&32==0:
            raise ValueError('PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
        if kind in (b'IHDR',b'PLTE',b'tRNS',b'IDAT',b'IEND'):
            chunks.append(data[offset:offset+size+12])
        offset+=size+12
    require(ended and dimensions is not None and compressed and (color!=3 or palette), 'PRODUCTION_IMAGE_INVALID')
    expected_size=(w*channels+1)*h; decoder=zlib.decompressobj()
    decoded=decoder.decompress(bytes(compressed),expected_size+1)
    require(len(decoded)==expected_size and decoder.eof and not decoder.unused_data and not decoder.unconsumed_tail,
            'PRODUCTION_IMAGE_INVALID')
    require(all(decoded[row*(w*channels+1)]<=4 for row in range(h)), 'PRODUCTION_IMAGE_INVALID')
    return b'\x89PNG\r\n\x1a\n'+b''.join(chunks),w,h


def make_mask(args, tools):
    exact(args,('source','region')); region=args['region']; exact(region,('x','y','width','height'))
    x=number(region['x'],0,1);y=number(region['y'],0,1)
    rw=number(region['width'],0.000001,1);rh=number(region['height'],0.000001,1)
    require(x+rw<=1.000000001 and y+rh<=1.000000001)
    path=source(args['source'])
    if path.suffix.lower()=='.png': _,w,h=png(path.read_bytes())
    else:
        require(path.suffix.lower() in ('.jpg','.jpeg','.webp'),'PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
        info=media_info(tools,path); streams=info.get('streams',[])
        require(len(streams)==1 and streams[0].get('codec_type')=='video','PRODUCTION_IMAGE_INVALID')
        w=int(number(streams[0]['width'],1,4096));h=int(number(streams[0]['height'],1,4096))
    left=int(x*w);top=int(y*h);right=min(w,math.ceil((x+rw)*w));bottom=min(h,math.ceil((y+rh)*h))
    raw=b''.join(b'\0'+(b'\0'*left+b'\xff'*(right-left)+b'\0'*(w-right) if top<=row<bottom else b'\0'*w) for row in range(h))
    def chunk(kind,body): return struct.pack('>I',len(body))+kind+body+struct.pack('>I',binascii.crc32(kind+body)&0xffffffff)
    result=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,0,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')
    png(result,(w,h));(OUT/'mask.png').write_bytes(result)
    return {'kind':'mask','width':w,'height':h,'region':region,'file':'mask.png'}


def batch_images(args, tools):
    exact(args,('sources','width','height'))
    sources=args['sources'];require(isinstance(sources,list) and 2<=len(sources)<=12)
    width=int(number(args['width'],1,4096));height=int(number(args['height'],1,4096))
    require(isinstance(tools.get('ffmpeg'),str),'PRODUCTION_TOOL_UNAVAILABLE')
    inputs=[]
    for item in sources:
        exact(item,('source',));inputs.append(source(item['source']))
    common=['-nostdin','-hide_banner','-loglevel','error','-n','-protocol_whitelist','file,pipe']
    items=[]
    for index,path in enumerate(inputs,1):
        output=OUT/f'image-{index:02}.png'
        try:
            run_tool(tools,'ffmpeg',common+input_format(path)+['-i',str(path),'-vf',f'scale={width}:{height}:force_original_aspect_ratio=decrease','-frames:v','1',str(output)])
            checked,w,h=png(output.read_bytes());output.write_bytes(checked)
            items.append({'index':index,'status':'converted','file':output.name,'width':w,'height':h})
        except (OSError,ValueError):
            output.unlink(missing_ok=True)
            items.append({'index':index,'status':'failed','reasonCode':'PRODUCTION_IMAGE_CONVERSION_FAILED'})
    value={'kind':'image_batch','maxWidth':width,'maxHeight':height,
           'successCount':sum(item['status']=='converted' for item in items),
           'failedCount':sum(item['status']=='failed' for item in items),'items':items}
    (OUT/'batch.json').write_text(json.dumps(value,ensure_ascii=False),encoding='utf-8')
    return value


def stamp(value):
    ms=round(value*1000)
    return f'{ms//3600000:02}:{ms//60000%60:02}:{ms//1000%60:02},{ms%1000:03}'


def podcast(args,tools):
    exact(args,('title','clips'))
    title=text(args['title'],160).strip();require(title)
    clips=args['clips'];require(isinstance(clips,list) and 1<=len(clips)<=12)
    common=['-nostdin','-hide_banner','-loglevel','error','-n','-protocol_whitelist','file,pipe']
    chapters=[];pieces=[];position=0.0
    for index,clip in enumerate(clips,1):
        exact(clip,('source','start','end','title'))
        chapter=text(clip['title'],160).strip();require(chapter)
        start=number(clip['start'],0,86400);end=number(clip['end'],start+0.02,start+1200)
        path=source(clip['source']);info=media_info(tools,path)
        require(any(stream.get('codec_type')=='audio' for stream in info.get('streams',[])),'PRODUCTION_AUDIO_REQUIRED')
        require(end<=float(info.get('format',{}).get('duration',0))+0.02,'PRODUCTION_CLIP_RANGE_INVALID')
        duration=end-start;require(position+duration<=1200,'PRODUCTION_DURATION_LIMIT')
        name=f'podcast-part-{index:02}.wav';pieces.append(name)
        run_tool(tools,'ffmpeg',common+input_format(path)+['-ss',str(start),'-i',str(path),'-t',str(duration),'-map','0:a:0','-vn','-ar','48000','-ac','2','-c:a','pcm_s16le',str(ROOT/name)])
        chapters.append({'index':index,'title':chapter,'start':round(position,3),'end':round(position+duration,3)})
        position+=duration
    (ROOT/'podcast-concat.txt').write_text(''.join(f"file '{name}'\n" for name in pieces),encoding='utf-8')
    output=OUT/'podcast.m4a'
    run_tool(tools,'ffmpeg',common+['-f','concat','-safe','1','-i',str(ROOT/'podcast-concat.txt'),'-map','0:a:0','-vn','-c:a','aac','-b:a','192k','-map_metadata','-1',str(output)])
    validation=media_info(tools,output);streams=validation.get('streams',[])
    require(any(stream.get('codec_type')=='audio' for stream in streams) and not any(stream.get('codec_type')=='video' for stream in streams),'PRODUCTION_OUTPUT_INVALID')
    actual=float(validation.get('format',{}).get('duration',0))
    require(abs(actual-position)<max(0.2,position*0.005),'PRODUCTION_DURATION_MISMATCH')
    package={'kind':'podcast','title':title,'file':output.name,'durationSeconds':actual,
             'chapterCount':len(chapters),'chapters':chapters}
    (OUT/'chapters.json').write_text(json.dumps({'title':title,'durationSeconds':actual,'chapters':chapters},ensure_ascii=False),encoding='utf-8')
    (OUT/'validation.json').write_text(json.dumps(validation,ensure_ascii=False),encoding='utf-8')
    return package


def speech(args,tools):
    exact(args,('title','text','language'))
    title=text(args['title'],160)
    script=text(args['text'],2000)
    language=text(args['language'],5)
    require(title and title==title.strip() and script.strip()
            and re.fullmatch(r'[a-z]{2}_[A-Z]{2}',language) is not None)
    require(isinstance(tools.get('piper'),str)
            and Path(tools['piper']).is_absolute(),'PRODUCTION_SPEECH_ENGINE_REQUIRED')
    model_value=tools.get('piperModel')
    require(isinstance(model_value,str) and Path(model_value).is_absolute(),
            'PRODUCTION_SPEECH_MODEL_REQUIRED')
    model=Path(model_value)
    require(model.is_file() and model.suffix.lower()=='.onnx'
            and 4<=model.stat().st_size<=2*1024*1024*1024,
            'PRODUCTION_SPEECH_MODEL_INVALID')
    config=Path(str(model)+'.json')
    require(config.is_file() and 2<=config.stat().st_size<=1024*1024,
            'PRODUCTION_SPEECH_CONFIG_INVALID')
    try:
        configuration=json.loads(config.read_text(encoding='utf-8'))
        configured_language=configuration['language']['code']
    except (OSError,UnicodeError,json.JSONDecodeError,KeyError,TypeError):
        raise ValueError('PRODUCTION_SPEECH_CONFIG_INVALID')
    require(isinstance(configured_language,str)
            and re.fullmatch(r'[a-z]{2}_[A-Z]{2}',configured_language) is not None,
            'PRODUCTION_SPEECH_CONFIG_INVALID')
    require(configured_language==language,'PRODUCTION_SPEECH_LANGUAGE_MISMATCH')
    model_digest=file_sha256(model)
    config_digest=file_sha256(config)
    input_file=ROOT/'speech-input.txt'
    input_file.write_text(script,encoding='utf-8')
    raw=ROOT/'speech-raw.wav'
    try:
        run_tool(tools,'piper',['--model',str(model),'--config',str(config),
                 '--input-file',str(input_file),'--output-file',str(raw)])
    except ValueError as error:
        if str(error)=='PRODUCTION_TOOL_FAILED':
            raise ValueError('PRODUCTION_SPEECH_SYNTHESIS_FAILED')
        raise
    require(raw.is_file() and 44<=raw.stat().st_size<=MAX_FILE,
            'PRODUCTION_SPEECH_OUTPUT_INVALID')
    raw_info=media_info(tools,raw)
    raw_streams=raw_info.get('streams',[])
    require(len(raw_streams)==1 and raw_streams[0].get('codec_type')=='audio',
            'PRODUCTION_SPEECH_OUTPUT_INVALID')
    duration=number(float(raw_info.get('format',{}).get('duration',0)),0.02,240)
    target=-18
    measured=audio_loudness(tools,raw,target)
    normalize=(f'loudnorm=I={target}:TP=-1.5:LRA=11:'
               f'measured_I={measured["input_i"]}:measured_TP={measured["input_tp"]}:'
               f'measured_LRA={measured["input_lra"]}:measured_thresh={measured["input_thresh"]}:'
               f'offset={measured["target_offset"]}:linear=true:print_format=summary')
    output=OUT/'speech.wav'
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',
             *input_format(raw),'-i',str(raw),'-vn','-af',normalize,'-ar','48000',
             '-ac','1','-c:a','pcm_s16le',str(output)])
    checked=media_info(tools,output)
    streams=checked.get('streams',[])
    actual=number(float(checked.get('format',{}).get('duration',0)),0.02,240)
    require(len(streams)==1 and streams[0].get('codec_type')=='audio'
            and streams[0].get('codec_name')=='pcm_s16le'
            and streams[0].get('sample_rate')=='48000'
            and streams[0].get('channels')==1,
            'PRODUCTION_SPEECH_OUTPUT_INVALID')
    require(abs(actual-duration)<max(0.2,duration*.005),'PRODUCTION_DURATION_MISMATCH')
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error',
             *input_format(output),'-i',str(output),'-map','0:a:0','-f','null','-'])
    after=audio_loudness(tools,output,target)
    target_difference=abs(after['input_i']-target)
    require(target_difference<=1,'PRODUCTION_AUDIO_LEVEL_MISMATCH')
    require(file_sha256(model)==model_digest and file_sha256(config)==config_digest,
            'PRODUCTION_SPEECH_MODEL_CHANGED')
    content={'schemaVersion':1,'title':title,'text':script,'language':language}
    content_file=OUT/'speech-content.json'
    content_file.write_text(json.dumps(content,ensure_ascii=False,indent=2),encoding='utf-8')
    safe_title=''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                       for character in title.replace('\r',' ').replace('\n',' '))
    summary=OUT/'summary.md'
    summary.write_text(
        f'# {safe_title}\n\n'
        f'- Result: `speech.wav` · {actual:.3f} seconds · 48 kHz mono PCM WAV\n'
        f'- Script: {len(script)} characters · locale `{language}`\n'
        f'- Checks: voice configuration locale matched; output reopened; full audio decoded; integrated loudness within 1 LU of {target} LUFS\n'
        '- Human review required: pronunciation, meaning, voice suitability, listening quality and voice-model licence\n'
        '- Not performed: voice cloning, translation, model download, remote generation or publication\n',
        encoding='utf-8')
    artifacts={path.name:file_sha256(path) for path in (output,content_file,summary)}
    manifest={'schemaVersion':1,'kind':'local-speech','engine':'Piper',
              'title':title,'language':language,
              'script':{'characters':len(script),'sha256':hashlib.sha256(script.encode('utf-8')).hexdigest()},
              'voice':{'modelSha256':model_digest,'configSha256':config_digest,
                       'configuredLanguage':configured_language},
              'output':{'file':output.name,'sha256':artifacts[output.name],
                        'durationSeconds':actual,'codec':streams[0].get('codec_name'),
                        'sampleRate':streams[0].get('sample_rate'),'channels':streams[0].get('channels'),
                        'integratedLufs':after['input_i'],'targetLufs':target,
                        'targetDifferenceLu':target_difference},
              'artifacts':artifacts,
              'verification':{'scriptPreserved':True,'configurationLanguageMatched':True,
                              'outputReopened':True,'fullAudioDecoded':True,
                              'loudnessTargetMatched':True,'pronunciationReviewed':False,
                              'meaningReviewed':False,'voiceSuitabilityReviewed':False,
                              'modelLicenceReviewed':False}}
    manifest_file=OUT/'speech-manifest.json'
    manifest_file.write_text(json.dumps(manifest,ensure_ascii=False,indent=2,allow_nan=False),encoding='utf-8')
    return {'kind':'local_speech','file':output.name,'language':language,
            'durationSeconds':actual,'manifestSha256':file_sha256(manifest_file),
            'humanReviewRequired':True}


def data_chart(args,tools):
    exact(args,('source','title','categoryColumn','valueColumn','aggregation'))
    title=text(args['title'],160).strip();category=text(args['categoryColumn'],160).strip()
    value=text(args['valueColumn'],160).strip();aggregation=args['aggregation']
    require(title and category and value and category!=value and aggregation in ('sum','average'))
    path=source(args['source']);require(path.suffix.lower()=='.csv','PRODUCTION_DATA_FORMAT_UNSUPPORTED')
    rows=list(itertools.islice(csv.reader(io.StringIO(path.read_text(encoding='utf-8-sig'))),100002))
    require(2<=len(rows)<=100001 and 1<=len(rows[0])<=100,'PRODUCTION_DATA_LIMIT')
    require(sum(len(row) for row in rows)<=200000,'PRODUCTION_DATA_LIMIT')
    require(all(len(row)==len(rows[0]) for row in rows),'PRODUCTION_CSV_SHAPE_INVALID')
    cleaned=[[text(cell,20000).strip() for cell in row] for row in rows]
    header=cleaned[0];require(len(set(header))==len(header) and category in header and value in header)
    category_index=header.index(category);value_index=header.index(value);groups={}
    for row in cleaned[1:]:
        label=row[category_index];require(label)
        try: number_value=float(row[value_index])
        except ValueError: raise ValueError('PRODUCTION_NON_NUMERIC_VALUE')
        require(math.isfinite(number_value) and -1e12<=number_value<=1e12,'PRODUCTION_NON_NUMERIC_VALUE')
        groups.setdefault(label,[]).append(number_value);require(len(groups)<=40,'PRODUCTION_CHART_LIMIT')
    require(groups)
    series=[{'label':label,'value':math.fsum(values) if aggregation=='sum' else math.fsum(values)/len(values),'rows':len(values)}
            for label,values in groups.items()]
    values=[item['value'] for item in series];low=min(0,min(values));high=max(0,max(values));span=high-low or 1
    width=1200;height=max(360,150+len(series)*48);left=320;right=80;top=105;plot=width-left-right
    position=lambda item:left+(item-low)/span*plot;zero=position(0)
    escape=lambda value:html.escape(str(value),quote=True)
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                                  for character in escape(value).replace('\r',' ').replace('\n',' '))
    elements=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
              '<rect width="100%" height="100%" fill="#f8f7f3"/>',f'<title id="title">{escape(title)}</title>',
              f'<desc id="desc">{escape(aggregation)} of {escape(value)} grouped by {escape(category)}</desc>',
              f'<text x="48" y="55" font-family="system-ui,sans-serif" font-size="30" font-weight="700" fill="#15223d">{escape(title)}</text>',
              f'<line x1="{zero:.2f}" y1="{top-20}" x2="{zero:.2f}" y2="{height-45}" stroke="#526079" stroke-width="2"/>']
    for index,item in enumerate(series):
        y=top+index*48;end=position(item['value']);x=min(zero,end);bar=max(2,abs(end-zero));colour='#2854f4' if item['value']>=0 else '#ff7a2f'
        display=format(item['value'],'.8g')
        elements.extend([f'<text x="48" y="{y+22}" font-family="system-ui,sans-serif" font-size="18" fill="#15223d">{escape(item["label"])}</text>',
                         f'<rect x="{x:.2f}" y="{y}" width="{bar:.2f}" height="28" rx="6" fill="{colour}"/>',
                         f'<text x="{min(width-70,max(left+8,end+10)):.2f}" y="{y+21}" font-family="ui-monospace,monospace" font-size="16" fill="#15223d">{escape(display)}</text>'])
    elements.append('</svg>')
    (OUT/'chart.svg').write_text(''.join(elements),encoding='utf-8')
    result={'kind':'data_chart','title':title,'categoryColumn':category,'valueColumn':value,
            'aggregation':aggregation,'rowsAnalyzed':len(cleaned)-1,'categoryCount':len(series),'series':series,'file':'chart.svg'}
    (OUT/'chart-data.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    report=['# '+markdown(title),'',f'- Source rows checked: {len(cleaned)-1}',f'- Grouping: {markdown(category)}',f'- Value: {aggregation} of {markdown(value)}',f'- Categories: {len(series)}','','| Category | Value | Rows |','|---|---:|---:|']
    report.extend(f'| {markdown(item["label"])} | {format(item["value"],".8g")} | {item["rows"]} |' for item in series)
    (OUT/'report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    return result


def data_scenario(args,tools):
    exact(args,('source','title','itemColumn','baseColumn','upsideColumn','downsideColumn','currency'))
    title=text(args['title'],160).strip();item=text(args['itemColumn'],160).strip()
    base=text(args['baseColumn'],160).strip();upside=text(args['upsideColumn'],160).strip()
    downside=text(args['downsideColumn'],160).strip();currency=text(args['currency'],3).strip()
    columns=(item,base,upside,downside)
    require(title and all(columns) and len(set(columns))==4 and re.fullmatch('[A-Z]{3}',currency) is not None)
    path=source(args['source']);require(path.suffix.lower()=='.csv','PRODUCTION_DATA_FORMAT_UNSUPPORTED')
    rows=list(itertools.islice(csv.reader(io.StringIO(path.read_text(encoding='utf-8-sig'))),5002))
    require(2<=len(rows)<=5001 and 1<=len(rows[0])<=100,'PRODUCTION_DATA_LIMIT')
    require(sum(len(row) for row in rows)<=200000,'PRODUCTION_DATA_LIMIT')
    require(all(len(row)==len(rows[0]) for row in rows),'PRODUCTION_CSV_SHAPE_INVALID')
    cleaned=[[text(cell,20000).strip() for cell in row] for row in rows]
    header=cleaned[0];require(all(header) and len(set(header))==len(header) and all(name in header for name in columns))
    indexes=[header.index(name) for name in columns];items=[];labels=set()
    for row in cleaned[1:]:
        label=row[indexes[0]];require(label and label not in labels,'PRODUCTION_SCENARIO_ITEM_INVALID');labels.add(label)
        values=[]
        for index in indexes[1:]:
            try: value=float(row[index])
            except ValueError: raise ValueError('PRODUCTION_NON_NUMERIC_VALUE')
            require(math.isfinite(value) and -1e12<=value<=1e12,'PRODUCTION_NON_NUMERIC_VALUE');values.append(value)
        items.append({'label':label,'base':values[0],'upside':values[1],'downside':values[2]})
    require(items)
    totals={name:math.fsum(entry[name] for entry in items) for name in ('base','upside','downside')}
    inline=lambda reference,value,style='':f'<c r="{reference}" t="inlineStr"{style}><is><t xml:space="preserve">{escape(value)}</t></is></c>'
    numeric=lambda reference,value,style='':f'<c r="{reference}"{style}><v>{format(value,".17g")}</v></c>'
    input_rows=['<row r="1">'+inline('A1','Item',' s="1"')+inline('B1',f'Base ({currency})',' s="1"')+inline('C1',f'Upside ({currency})',' s="1"')+inline('D1',f'Downside ({currency})',' s="1"')+'</row>']
    for row_number,entry in enumerate(items,2):
        input_rows.append(f'<row r="{row_number}">'+inline(f'A{row_number}',entry['label'])+numeric(f'B{row_number}',entry['base'],' s="2"')+numeric(f'C{row_number}',entry['upside'],' s="2"')+numeric(f'D{row_number}',entry['downside'],' s="2"')+'</row>')
    last_input=len(items)+1
    inputs=f'<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:D{last_input}"/><sheetViews><sheetView workbookViewId="0"><pane ySplit="1" topLeftCell="A2" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><cols><col min="1" max="1" width="32" customWidth="1"/><col min="2" max="4" width="18" customWidth="1"/></cols><sheetData>{"".join(input_rows)}</sheetData><autoFilter ref="A1:D{last_input}"/></worksheet>'
    model_rows=['<row r="1" ht="28" customHeight="1">'+inline('A1',title,' s="1"')+'</row>',
                '<row r="2">'+inline('A2','Scenario',' s="1"')+inline('B2','Base')+'</row>',
                '<row r="4">'+inline('A4','Item',' s="1"')+inline('B4',f'Value ({currency})',' s="1"')+'</row>']
    for index,entry in enumerate(items):
        row_number=index+5;input_row=index+2
        formula=f'IF($B$2="Base",Inputs!B{input_row},IF($B$2="Upside",Inputs!C{input_row},Inputs!D{input_row}))'
        model_rows.append(f'<row r="{row_number}">'+inline(f'A{row_number}',entry['label'])+f'<c r="B{row_number}" s="2"><f ca="1">{formula}</f><v>{format(entry["base"],".17g")}</v></c></row>')
    total_row=len(items)+5;first_value=5;last_value=total_row-1
    model_rows.append(f'<row r="{total_row}">'+inline(f'A{total_row}','Total',' s="1"')+f'<c r="B{total_row}" s="3"><f ca="1">SUM(B{first_value}:B{last_value})</f><v>{format(totals["base"],".17g")}</v></c></row>')
    model=f'<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:B{total_row}"/><sheetViews><sheetView workbookViewId="0"><pane ySplit="4" topLeftCell="A5" activePane="bottomLeft" state="frozen"/></sheetView></sheetViews><cols><col min="1" max="1" width="32" customWidth="1"/><col min="2" max="2" width="20" customWidth="1"/></cols><sheetData>{"".join(model_rows)}</sheetData><mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells><dataValidations count="1"><dataValidation type="list" allowBlank="0" showErrorMessage="1" errorTitle="Choose a scenario" error="Use Base, Upside or Downside." sqref="B2"><formula1>"Base,Upside,Downside"</formula1></dataValidation></dataValidations></worksheet>'
    styles='''<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="1"><numFmt numFmtId="164" formatCode="#,#0.00"/></numFmts><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><color rgb="FFFFFFFF"/><sz val="11"/><name val="Calibri"/></font></fonts><fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF2854F4"/><bgColor indexed="64"/></patternFill></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="4"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1"/><xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="1"/><xf numFmtId="164" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="1" applyFill="1" applyNumberFormat="1"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles><dxfs count="0"/><tableStyles count="0" defaultTableStyle="TableStyleMedium2" defaultPivotStyle="PivotStyleLight16"/></styleSheet>'''
    workbook=f'<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="{OFFICE}"><bookViews><workbookView activeTab="0"/></bookViews><sheets><sheet name="Scenario Model" sheetId="1" r:id="rId2"/><sheet name="Inputs" sheetId="2" r:id="rId1"/></sheets><calcPr calcId="191029" calcMode="auto" fullCalcOnLoad="1" forceFullCalc="1"/></workbook>'
    package={'[Content_Types].xml':content_types([('xl/workbook.xml','spreadsheetml.sheet.main'),('xl/worksheets/sheet1.xml','spreadsheetml.worksheet'),('xl/worksheets/sheet2.xml','spreadsheetml.worksheet'),('xl/styles.xml','spreadsheetml.styles')]),'_rels/.rels':rels([(1,'officeDocument','xl/workbook.xml')]),'xl/workbook.xml':workbook,'xl/_rels/workbook.xml.rels':rels([(1,'worksheet','worksheets/sheet1.xml'),(2,'worksheet','worksheets/sheet2.xml'),(3,'styles','styles.xml')]),'xl/worksheets/sheet1.xml':inputs,'xl/worksheets/sheet2.xml':model,'xl/styles.xml':styles}
    workbook_path=write_zip('scenario.xlsx',package);checked=read_zip(workbook_path)
    sheet=xml(checked['xl/worksheets/sheet2.xml']);ns={'s':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
    formulas=[node.text or '' for node in sheet.findall('.//s:f',ns)]
    expected=[f'IF($B$2="Base",Inputs!B{index},IF($B$2="Upside",Inputs!C{index},Inputs!D{index}))' for index in range(2,len(items)+2)]
    validation=sheet.find('.//s:dataValidation/s:formula1',ns)
    workbook_node=xml(checked['xl/workbook.xml']);calculation=workbook_node.find('.//s:calcPr',ns)
    require(formulas[:-1]==expected and formulas[-1]==f'SUM(B{first_value}:B{last_value})' and validation is not None and validation.text=='"Base,Upside,Downside"' and calculation is not None and calculation.attrib.get('calcMode')=='auto' and calculation.attrib.get('fullCalcOnLoad')=='1' and calculation.attrib.get('forceFullCalc')=='1','PRODUCTION_OUTPUT_INVALID')
    result={'kind':'data_scenario','title':title,'itemColumn':item,'scenarioColumns':{'base':base,'upside':upside,'downside':downside},'currency':currency,'rowCount':len(items),'formulaCells':len(formulas),'recalculation':'automatic','scenarios':[{'name':name.title(),'total':totals[name]} for name in ('base','upside','downside')],'items':items,'file':'scenario.xlsx'}
    (OUT/'scenario-data.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'',f'- Rows checked: {len(items)}',f'- Currency: {currency}',f'- Item column: {markdown(item)}','- Workbook selector: Base / Upside / Downside','',f'| Scenario | Total ({currency}) |','|---|---:|']
    report.extend(f'| {name.title()} | {format(totals[name],".8g")} |' for name in ('base','upside','downside'))
    (OUT/'report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    return result


def extracted_sources(sources,tools):
    require(isinstance(sources,list) and 1<=len(sources)<=8)
    prepared=[];seen_paths=set()
    for index,item in enumerate(sources,1):
        exact(item,('source','label'));path=source(item['source']);label=text(item['label'],200).strip()
        require(label and '/' not in label and '\\' not in label and str(path) not in seen_paths);seen_paths.add(str(path));suffix=path.suffix.lower()
        raw=path.read_bytes();source_hash=hashlib.sha256(raw).hexdigest()
        if suffix in ('.txt','.md','.csv','.json','.html','.htm','.srt'):
            require(len(raw)<=1024*1024,'PRODUCTION_DATA_LIMIT')
            try: content=raw.decode('utf-8-sig')
            except UnicodeDecodeError: raise ValueError('PRODUCTION_DATA_FORMAT_UNSUPPORTED')
        elif suffix=='.pdf':
            raw_text=run_tool(tools,'pdftotext',['-layout',str(path),'-'])
            try: content=raw_text.decode('utf-8')
            except UnicodeDecodeError: raise ValueError('PRODUCTION_DATA_FORMAT_UNSUPPORTED')
        elif suffix in ('.png','.jpg','.jpeg','.webp'):
            raw_text=run_tool(tools,'tesseract',[str(path),'stdout','--psm','3'])
            try: content=raw_text.decode('utf-8')
            except UnicodeDecodeError: raise ValueError('PRODUCTION_DATA_FORMAT_UNSUPPORTED')
        else: raise ValueError('PRODUCTION_DATA_FORMAT_UNSUPPORTED')
        content=content.replace('\r\n','\n').replace('\r','\n');text(content,1024*1024)
        require(content.strip(),'PRODUCTION_EXTRACTION_EMPTY_SOURCE')
        prepared.append({'path':str(path),'id':f'source-{index:02}','label':label,'sourceSha256':source_hash,
                         'textSha256':hashlib.sha256(content.encode()).hexdigest(),'content':content})
    return prepared


def source_changes(args,tools):
    exact(args,('title','before','after'))
    title=text(args['title'],160).strip();require(title)
    snapshots=[]
    for key in ('before','after'):
        item=args[key];exact(item,('source','label','capturedAt'))
        label=text(item['label'],200).strip();captured=text(item['capturedAt'],10)
        require(label and '/' not in label and '\\' not in label)
        try: captured_date=datetime.date.fromisoformat(captured)
        except ValueError: raise ValueError('PRODUCTION_SOURCE_DATE_INVALID')
        snapshots.append({'source':item['source'],'label':label,'capturedAt':captured,'date':captured_date})
    require(snapshots[1]['date']>snapshots[0]['date'],'PRODUCTION_SOURCE_DATE_ORDER_INVALID')
    prepared=extracted_sources([{'source':item['source'],'label':item['label']} for item in snapshots],tools)

    def units(record):
        result=[]
        for line_number,value in enumerate(record['content'].splitlines(),1):
            normalized=re.sub(r'\s+',' ',value).strip()
            if normalized:
                text(value,4000);result.append({'line':line_number,'text':value.strip(),'normalized':normalized})
        require(1<=len(result)<=2000 and sum(len(item['text']) for item in result)<=200000,
                'PRODUCTION_SOURCE_CHANGE_LIMIT')
        return result

    before_units=units(prepared[0]);after_units=units(prepared[1])
    matcher=difflib.SequenceMatcher(None,[item['normalized'] for item in before_units],
                                    [item['normalized'] for item in after_units],autojunk=False)
    changes=[];unchanged=0
    def append(kind,before=None,after=None):
        require(len(changes)<1000,'PRODUCTION_SOURCE_CHANGE_LIMIT')
        changes.append({'index':len(changes)+1,'type':kind,
                        'beforeLine':before['line'] if before else None,
                        'afterLine':after['line'] if after else None,
                        'before':before['text'] if before else None,
                        'after':after['text'] if after else None})
    for tag,i1,i2,j1,j2 in matcher.get_opcodes():
        if tag=='equal':
            unchanged+=i2-i1
        elif tag=='delete':
            for item in before_units[i1:i2]: append('removed',before=item)
        elif tag=='insert':
            for item in after_units[j1:j2]: append('added',after=item)
        else:
            before_block=before_units[i1:i2];after_block=after_units[j1:j2]
            paired=min(len(before_block),len(after_block))
            for index in range(paired): append('modified',before_block[index],after_block[index])
            for item in before_block[paired:]: append('removed',before=item)
            for item in after_block[paired:]: append('added',after=item)
    counts={kind:sum(item['type']==kind for item in changes) for kind in ('added','removed','modified')}
    public=[]
    for snapshot,record,items in zip(snapshots,prepared,(before_units,after_units)):
        public.append({'label':snapshot['label'],'capturedAt':snapshot['capturedAt'],
                       'sourceSha256':record['sourceSha256'],'textSha256':record['textSha256'],
                       'meaningfulLineCount':len(items)})
    result={'kind':'source_change_report','title':title,'before':public[0],'after':public[1],
            'hasChanges':bool(changes),
            'ignoredFormattingOnly':not changes and prepared[0]['textSha256']!=prepared[1]['textSha256'],
            'summary':{'added':counts['added'],'removed':counts['removed'],'modified':counts['modified'],
                       'unchanged':unchanged},'changes':changes,'file':'change-report.md'}
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                                  for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'',
            f'- Earlier snapshot: {markdown(public[0]["label"])} · {public[0]["capturedAt"]} · `{public[0]["sourceSha256"]}`',
            f'- Later snapshot: {markdown(public[1]["label"])} · {public[1]["capturedAt"]} · `{public[1]["sourceSha256"]}`','',
            '| Result | Count |','|---|---:|',f'| Added | {counts["added"]} |',
            f'| Removed | {counts["removed"]} |',f'| Modified | {counts["modified"]} |',
            f'| Unchanged | {unchanged} |','']
    if not changes:
        report.append('No meaningful text changes detected.')
        if result['ignoredFormattingOnly']:
            report.extend(('', 'Whitespace or blank-line differences were ignored.'))
    else:
        for item in changes:
            report.extend((f'## {item["type"].title()} {item["index"]}',''))
            if item['before'] is not None:
                report.extend((f'**Before · line {item["beforeLine"]}**',markdown(item['before']),'',))
            if item['after'] is not None:
                report.extend((f'**After · line {item["afterLine"]}**',markdown(item['after']),'',))
    (OUT/'change-report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    (OUT/'change-report.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    return result


def claim_check(args,tools):
    exact(args,('title','sources','claims'))
    title=text(args['title'],160).strip();require(title,'PRODUCTION_TITLE_REQUIRED')
    prepared=extracted_sources(args['sources'],tools)
    source_inputs=[item['source'] for item in args['sources']]
    require(len(source_inputs)==len(set(source_inputs)),'PRODUCTION_DUPLICATE_SOURCE')
    by_input={item['source']:record for item,record in zip(args['sources'],prepared)}
    claims=args['claims'];require(isinstance(claims,list) and 1<=len(claims)<=100,'PRODUCTION_CLAIM_LIMIT')
    checked_claims=[];seen_claims=set();total_evidence=0
    for claim_index,item in enumerate(claims,1):
        exact(item,('claim','checkedSources','evidence','unresolvedReason'))
        statement=text(item['claim'],2000).strip();require(statement and statement not in seen_claims,'PRODUCTION_DUPLICATE_CLAIM')
        seen_claims.add(statement)
        checked_entries=item['checkedSources'];require(isinstance(checked_entries,list) and 1<=len(checked_entries)<=len(prepared),'PRODUCTION_CHECKED_SOURCE_REQUIRED')
        for entry in checked_entries: exact(entry,('source',))
        checked=[entry['source'] for entry in checked_entries]
        require(len(checked)==len(set(checked)) and all(value in by_input for value in checked),'PRODUCTION_CHECKED_SOURCE_INVALID')
        evidence=item['evidence'];require(isinstance(evidence,list) and len(evidence)<=20,'PRODUCTION_CLAIM_LIMIT')
        total_evidence+=len(evidence);require(total_evidence<=500,'PRODUCTION_CLAIM_LIMIT')
        verified=[];stances=[]
        for evidence_index,entry in enumerate(evidence,1):
            exact(entry,('source','stance','quote'))
            source_input=entry['source'];require(source_input in checked and source_input in by_input,'PRODUCTION_EVIDENCE_SOURCE_UNCHECKED')
            stance=entry['stance'];require(stance in ('supports','disputes','context'),'PRODUCTION_EVIDENCE_STANCE_INVALID')
            quote=text(entry['quote'],4000);require(quote.strip(),'PRODUCTION_EVIDENCE_QUOTE_REQUIRED')
            record=by_input[source_input];start=record['content'].find(quote)
            require(start>=0,'PRODUCTION_EVIDENCE_QUOTE_MISMATCH')
            byte_start=len(record['content'][:start].encode('utf-8'))
            byte_end=byte_start+len(quote.encode('utf-8'))
            line=record['content'].count('\n',0,start)+1
            verified.append({'index':evidence_index,'sourceId':record['id'],'sourceLabel':record['label'],
                             'stance':stance,'quote':quote,'line':line,
                             'startByte':byte_start,'endByte':byte_end})
            stances.append(stance)
        status='disputed' if 'disputes' in stances else ('supported' if 'supports' in stances else 'unresolved')
        reason=item['unresolvedReason']
        if status=='unresolved':
            reason=text(reason,1000).strip() if isinstance(reason,str) else ''
            require(reason,'PRODUCTION_UNRESOLVED_REASON_REQUIRED')
        else:
            require(reason is None,'PRODUCTION_CLAIM_STATUS_CONFLICT')
        checked_claims.append({'index':claim_index,'claim':statement,'status':status,
                               'checkedSourceIds':[by_input[value]['id'] for value in checked],
                               'unresolvedReason':reason if status=='unresolved' else None,
                               'evidence':verified,
                               'evidenceCounts':{stance:sum(value==stance for value in stances)
                                                 for stance in ('supports','disputes','context')}})
    public_sources=[{'id':record['id'],'label':record['label'],'sourceSha256':record['sourceSha256'],
                     'textSha256':record['textSha256']} for record in prepared]
    counts={status:sum(item['status']==status for item in checked_claims)
            for status in ('supported','disputed','unresolved')}
    result={'kind':'claim_check_report','title':title,'sources':public_sources,'summary':counts,
            'claims':checked_claims,'file':'claim-check-report.md'}
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                                  for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'',
            '> Statuses are evidence classifications, not automatic truth scores.','',
            '| Status | Claims |','|---|---:|',f'| Supported | {counts["supported"]} |',
            f'| Disputed | {counts["disputed"]} |',f'| Unresolved | {counts["unresolved"]} |','',
            '## Inspected sources','']
    report.extend(f'- **{markdown(record["id"])} · {markdown(record["label"])}** · source `{record["sourceSha256"]}` · text `{record["textSha256"]}`'
                  for record in public_sources)
    for item in checked_claims:
        report.extend(('',f'## Claim {item["index"]} · {item["status"].title()}','',markdown(item['claim']),'',
                       '**Sources checked:** '+', '.join(markdown(value) for value in item['checkedSourceIds'])))
        if item['unresolvedReason'] is not None:
            report.extend(('', '**Unresolved because:** '+markdown(item['unresolvedReason'])))
        for evidence in item['evidence']:
            report.extend(('',f'### Evidence {evidence["index"]} · {evidence["stance"].title()}','',
                           f'{markdown(evidence["sourceId"])} · {markdown(evidence["sourceLabel"])} · line {evidence["line"]} · bytes {evidence["startByte"]}–{evidence["endByte"]}',
                           '',f'> {markdown(evidence["quote"])}'))
    (OUT/'claim-check-report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    (OUT/'claim-check.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    return result


def research_compare(args,tools):
    exact(args,('title','question','sources','criteria','options','recommendation'))
    title=text(args['title'],160).strip();question=text(args['question'],2000).strip()
    require(title,'PRODUCTION_TITLE_REQUIRED');require(question,'PRODUCTION_COMPARISON_QUESTION_REQUIRED')
    prepared=extracted_sources(args['sources'],tools)
    by_input={item['source']:record for item,record in zip(args['sources'],prepared)}

    criteria=args['criteria'];require(isinstance(criteria,list) and 1<=len(criteria)<=12,'PRODUCTION_COMPARISON_CRITERIA_LIMIT')
    checked_criteria=[];criterion_keys=[];seen_labels=set()
    for index,item in enumerate(criteria,1):
        exact(item,('key','label'));key=text(item['key'],32);label=text(item['label'],160).strip()
        require(re.fullmatch(r'[a-z][a-z0-9_]{0,31}',key) is not None and key not in criterion_keys,
                'PRODUCTION_COMPARISON_CRITERIA_INVALID')
        require(label and label not in seen_labels,'PRODUCTION_COMPARISON_CRITERIA_INVALID')
        criterion_keys.append(key);seen_labels.add(label)
        checked_criteria.append({'index':index,'key':key,'label':label})

    used_sources=set();total_evidence=0
    def verified(entry):
        nonlocal total_evidence
        exact(entry,('source','quote'));source_input=entry['source']
        require(source_input in by_input,'PRODUCTION_COMPARISON_SOURCE_INVALID')
        quote=text(entry['quote'],4000);require(quote.strip(),'PRODUCTION_COMPARISON_EVIDENCE_REQUIRED')
        record=by_input[source_input];start=record['content'].find(quote)
        require(start>=0,'PRODUCTION_COMPARISON_EVIDENCE_MISMATCH')
        total_evidence+=1;require(total_evidence<=500,'PRODUCTION_COMPARISON_LIMIT')
        used_sources.add(source_input);byte_start=len(record['content'][:start].encode('utf-8'))
        return {'sourceId':record['id'],'sourceLabel':record['label'],'quote':quote,
                'line':record['content'].count('\n',0,start)+1,'startByte':byte_start,
                'endByte':byte_start+len(quote.encode('utf-8'))}

    options=args['options'];require(isinstance(options,list) and 2<=len(options)<=12,'PRODUCTION_COMPARISON_OPTION_LIMIT')
    checked_options=[];option_names=set();expected_keys=set(criterion_keys)
    for option_index,item in enumerate(options,1):
        exact(item,('option','cells'));name=text(item['option'],240).strip()
        require(name and name not in option_names,'PRODUCTION_COMPARISON_OPTION_INVALID');option_names.add(name)
        cells=item['cells'];require(isinstance(cells,dict) and set(cells)==expected_keys,
                                    'PRODUCTION_COMPARISON_CELL_SET_MISMATCH')
        checked_cells={}
        for key in criterion_keys:
            cell=cells[key];exact(cell,('status','assessment','evidence'));status=cell['status']
            require(status in ('confirmed','uncertain','missing'),'PRODUCTION_COMPARISON_STATUS_INVALID')
            evidence=cell['evidence'];require(isinstance(evidence,list) and len(evidence)<=8,
                                               'PRODUCTION_COMPARISON_EVIDENCE_REQUIRED')
            if status=='missing':
                require(cell['assessment'] is None and not evidence,'PRODUCTION_COMPARISON_MISSING_CONFLICT')
                assessment=None;checked_evidence=[]
            else:
                assessment=text(cell['assessment'],2000).strip() if isinstance(cell['assessment'],str) else ''
                require(assessment and evidence,'PRODUCTION_COMPARISON_EVIDENCE_REQUIRED')
                checked_evidence=[];pairs=set()
                for entry in evidence:
                    value=verified(entry);pair=(value['sourceId'],value['quote'])
                    require(pair not in pairs,'PRODUCTION_COMPARISON_EVIDENCE_DUPLICATE');pairs.add(pair)
                    checked_evidence.append(value)
            checked_cells[key]={'status':status,'assessment':assessment,'evidence':checked_evidence}
        checked_options.append({'index':option_index,'option':name,'cells':checked_cells})
    require(used_sources==set(by_input),'PRODUCTION_COMPARISON_UNUSED_SOURCE')

    recommendation=args['recommendation'];exact(recommendation,('option','reason','basedOnCriteria','uncertainty'))
    selected=recommendation['option'];reason=text(recommendation['reason'],2000).strip()
    uncertainty=text(recommendation['uncertainty'],2000).strip();basis=recommendation['basedOnCriteria']
    require(reason and uncertainty and reason!=uncertainty,'PRODUCTION_COMPARISON_RECOMMENDATION_INVALID')
    require(isinstance(basis,list) and all(isinstance(key,str) for key in basis)
            and len(basis)==len(set(basis)) and all(key in expected_keys for key in basis),
            'PRODUCTION_COMPARISON_RECOMMENDATION_INVALID')
    if selected is None:
        require(not basis,'PRODUCTION_COMPARISON_RECOMMENDATION_INVALID')
    else:
        selected=text(selected,240).strip();require(selected in option_names and basis,
                                                   'PRODUCTION_COMPARISON_RECOMMENDATION_INVALID')
        selected_cells=next(item['cells'] for item in checked_options if item['option']==selected)
        require(all(selected_cells[key]['status']!='missing' for key in basis),
                'PRODUCTION_COMPARISON_RECOMMENDATION_UNSUPPORTED')
    checked_recommendation={'option':selected,'reason':reason,'basedOnCriteria':basis,'uncertainty':uncertainty}

    public_sources=[{key:item[key] for key in ('id','label','sourceSha256','textSha256')} for item in prepared]
    status_counts={status:sum(cell['status']==status for option in checked_options for cell in option['cells'].values())
                   for status in ('confirmed','uncertain','missing')}
    result={'kind':'research_comparison','title':title,'question':question,'sources':public_sources,
            'summary':{'sources':len(prepared),'criteria':len(checked_criteria),'options':len(checked_options),
                       **status_counts},'criteria':checked_criteria,'options':checked_options,
            'recommendation':checked_recommendation,'file':'comparison-report.md'}
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                                  for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'','**Decision question:** '+markdown(question),'',
            '> This table applies the same owner criteria to every option. Evidence integrity is checked; the recommendation remains a reviewable judgment, not an automatic truth score.','',
            '## Inspected sources','']
    report.extend(f'- **{markdown(record["id"])} · {markdown(record["label"])}** · source `{record["sourceSha256"]}` · text `{record["textSha256"]}`'
                  for record in public_sources)
    report.extend(('', '## Comparison','',
                   '| Owner criterion | '+' | '.join(markdown(item['option']) for item in checked_options)+' |',
                   '|---|'+'|'.join('---' for _ in checked_options)+'|'))
    for criterion in checked_criteria:
        values=[]
        for option in checked_options:
            cell=option['cells'][criterion['key']]
            values.append(markdown(cell['status'].title()+(' · '+cell['assessment'] if cell['assessment'] else '')))
        report.append('| '+markdown(criterion['label'])+' | '+' | '.join(values)+' |')
    for option in checked_options:
        report.extend(('',f'### Evidence · {markdown(option["option"])}'))
        for criterion in checked_criteria:
            cell=option['cells'][criterion['key']]
            report.extend(('',f'**{markdown(criterion["label"])} · {markdown(cell["status"].title())}**'))
            if cell['assessment'] is not None: report.extend(('',markdown(cell['assessment'])))
            if not cell['evidence']: report.extend(('', 'No source evidence supplied; this cell remains missing.'))
            for evidence in cell['evidence']:
                report.extend(('',f'{markdown(evidence["sourceId"])} · {markdown(evidence["sourceLabel"])} · line {evidence["line"]} · bytes {evidence["startByte"]}–{evidence["endByte"]}',
                               '',f'> {markdown(evidence["quote"])}'))
    criterion_labels={item['key']:item['label'] for item in checked_criteria}
    report.extend(('', '## Recommendation','',
                   markdown(selected) if selected is not None else 'No option recommended.',
                   '',markdown(reason),'',
                   '**Based on owner criteria:** '+(', '.join(markdown(criterion_labels[key]) for key in basis) if basis else 'None'),'',
                   '## Uncertainty','',markdown(uncertainty)))
    (OUT/'comparison-report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    (OUT/'comparison-evidence.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    return result


def research_synthesis(args,tools):
    exact(args,('title','focus','sources','findings','disagreements'))
    title=text(args['title'],160).strip();focus=text(args['focus'],2000).strip()
    require(title,'PRODUCTION_TITLE_REQUIRED');require(focus,'PRODUCTION_SYNTHESIS_FOCUS_REQUIRED')
    require(isinstance(args['sources'],list) and 2<=len(args['sources'])<=8,'PRODUCTION_SYNTHESIS_SOURCE_LIMIT')
    prepared=extracted_sources(args['sources'],tools)
    by_input={item['source']:record for item,record in zip(args['sources'],prepared)}
    used_sources=set();total_evidence=0

    def verified(entry):
        nonlocal total_evidence
        exact(entry,('source','quote'));source_input=entry['source']
        require(source_input in by_input,'PRODUCTION_SYNTHESIS_SOURCE_INVALID')
        quote=text(entry['quote'],4000);require(quote.strip(),'PRODUCTION_SYNTHESIS_EVIDENCE_REQUIRED')
        record=by_input[source_input];start=record['content'].find(quote)
        require(start>=0,'PRODUCTION_SYNTHESIS_EVIDENCE_MISMATCH')
        total_evidence+=1;require(total_evidence<=500,'PRODUCTION_SYNTHESIS_LIMIT')
        used_sources.add(source_input);byte_start=len(record['content'][:start].encode('utf-8'))
        return record,{'sourceId':record['id'],'sourceLabel':record['label'],'quote':quote,
                       'line':record['content'].count('\n',0,start)+1,'startByte':byte_start,
                       'endByte':byte_start+len(quote.encode('utf-8'))}

    findings=args['findings'];require(isinstance(findings,list) and 1<=len(findings)<=100,'PRODUCTION_SYNTHESIS_LIMIT')
    checked_findings=[];seen_findings=set();cross_source=0
    for index,item in enumerate(findings,1):
        exact(item,('finding','evidence'));finding=text(item['finding'],2000).strip()
        require(finding and finding not in seen_findings,'PRODUCTION_SYNTHESIS_FINDING_INVALID');seen_findings.add(finding)
        evidence=item['evidence'];require(isinstance(evidence,list) and 1<=len(evidence)<=8,'PRODUCTION_SYNTHESIS_EVIDENCE_REQUIRED')
        checked=[];pairs=set();source_ids=set()
        for entry in evidence:
            record,value=verified(entry);pair=(record['id'],value['quote'])
            require(pair not in pairs,'PRODUCTION_SYNTHESIS_EVIDENCE_DUPLICATE');pairs.add(pair)
            source_ids.add(record['id']);checked.append(value)
        if len(source_ids)>=2: cross_source+=1
        checked_findings.append({'index':index,'finding':finding,'sourceIds':sorted(source_ids),'evidence':checked})
    require(cross_source>=1,'PRODUCTION_SYNTHESIS_CROSS_SOURCE_REQUIRED')

    disagreements=args['disagreements'];require(isinstance(disagreements,list) and len(disagreements)<=50,'PRODUCTION_SYNTHESIS_LIMIT')
    checked_disagreements=[];seen_topics=set()
    for index,item in enumerate(disagreements,1):
        exact(item,('topic','positions'));topic=text(item['topic'],1000).strip()
        require(topic and topic not in seen_topics,'PRODUCTION_SYNTHESIS_DISAGREEMENT_INVALID');seen_topics.add(topic)
        positions=item['positions'];require(isinstance(positions,list) and 2<=len(positions)<=8,'PRODUCTION_SYNTHESIS_DISAGREEMENT_INVALID')
        checked=[];position_sources=set()
        for position_index,position in enumerate(positions,1):
            exact(position,('source','position','quote'));summary=text(position['position'],2000).strip()
            require(summary,'PRODUCTION_SYNTHESIS_DISAGREEMENT_INVALID')
            record,value=verified({'source':position['source'],'quote':position['quote']})
            require(record['id'] not in position_sources,'PRODUCTION_SYNTHESIS_DISAGREEMENT_SOURCE_DUPLICATE')
            position_sources.add(record['id']);value.update({'index':position_index,'position':summary});checked.append(value)
        checked_disagreements.append({'index':index,'topic':topic,'positions':checked,'status':'unresolved'})
    require(used_sources==set(by_input),'PRODUCTION_SYNTHESIS_UNUSED_SOURCE')
    public_sources=[{key:item[key] for key in ('id','label','sourceSha256','textSha256')} for item in prepared]
    result={'kind':'research_synthesis','title':title,'focus':focus,'sources':public_sources,
            'summary':{'sources':len(prepared),'findings':len(checked_findings),
                       'crossSourceFindings':cross_source,'disagreements':len(checked_disagreements)},
            'findings':checked_findings,'disagreements':checked_disagreements,'file':'synthesis-report.md'}
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                                  for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'','**Focus:** '+markdown(focus),'',
            '> Findings are attributed synthesis; disagreement remains unresolved unless the sources themselves resolve it.','',
            '## Inspected sources','']
    report.extend(f'- **{markdown(record["id"])} · {markdown(record["label"])}** · source `{record["sourceSha256"]}` · text `{record["textSha256"]}`'
                  for record in public_sources)
    report.extend(('', '## Findings'))
    for item in checked_findings:
        report.extend(('',f'### Finding {item["index"]}','',markdown(item['finding'])))
        for evidence_index,evidence in enumerate(item['evidence'],1):
            report.extend(('',f'**Evidence {evidence_index} · {markdown(evidence["sourceId"])} · {markdown(evidence["sourceLabel"])} · line {evidence["line"]} · bytes {evidence["startByte"]}–{evidence["endByte"]}**','',f'> {markdown(evidence["quote"])}'))
    report.extend(('', '## Disagreements'))
    if not checked_disagreements:
        report.extend(('', 'No source disagreement was supplied. This is not evidence of consensus.'))
    for item in checked_disagreements:
        report.extend(('',f'### Disagreement {item["index"]}','',markdown(item['topic'])))
        for position in item['positions']:
            report.extend(('',f'**Position {position["index"]} · {markdown(position["sourceId"])} · {markdown(position["sourceLabel"])} · line {position["line"]}**','',
                           markdown(position['position']),'',f'> {markdown(position["quote"])}'))
        report.extend(('', '**Status:** Unresolved source disagreement.'))
    (OUT/'synthesis-report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    (OUT/'synthesis-evidence.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    return result


def data_extract(args,tools):
    exact(args,('title','sources','columns','rows'))
    title=text(args['title'],160).strip();require(title)
    prepared=extracted_sources(args['sources'],tools)
    columns=args['columns'];require(isinstance(columns,list) and 1<=len(columns)<=12)
    checked_columns=[];keys=set()
    for column in columns:
        exact(column,('key','label'));key=text(column['key'],32).strip();label=text(column['label'],80).strip()
        require(re.fullmatch('[a-z][a-z0-9_]{0,31}',key) is not None and label and key not in keys)
        keys.add(key);checked_columns.append({'key':key,'label':label})
    rows=args['rows'];require(isinstance(rows,list) and 1<=len(rows)<=500 and len(rows)*len(columns)<=6000)
    checked_rows=[];counts={'confirmed':0,'uncertain':0,'missing':0}
    for row_index,row in enumerate(rows,1):
        exact(row,('source','cells'));path=source(row['source'])
        record=next((item for item in prepared if item['path']==str(path)),None);require(record is not None)
        cells=row['cells'];require(isinstance(cells,dict) and set(cells)==keys)
        checked={}
        for column in checked_columns:
            key=column['key'];cell=cells[key];exact(cell,('status','value','evidence'))
            status=cell['status'];require(isinstance(status,str) and status in counts)
            if status=='missing':
                require(cell['value'] is None and cell['evidence'] is None);value=None;evidence=None;line=None
            else:
                value=text(cell['value'],2000).strip();evidence=text(cell['evidence'],4000).strip()
                require(value and evidence and value in evidence,'PRODUCTION_EXTRACTION_EVIDENCE_INVALID')
                offset=record['content'].find(evidence);require(offset>=0,'PRODUCTION_EXTRACTION_EVIDENCE_INVALID')
                line=record['content'].count('\n',0,offset)+1
            counts[status]+=1;checked[key]={'value':value,'status':status,'evidence':evidence,'line':line}
        checked_rows.append({'row':row_index,'sourceId':record['id'],'sourceLabel':record['label'],'cells':checked})
    public_sources=[{key:item[key] for key in ('id','label','sourceSha256','textSha256')} for item in prepared]
    result={'kind':'data_extraction','title':title,'columns':checked_columns,'sources':public_sources,
            'rowCount':len(checked_rows),'cellCount':sum(counts.values()),'statusCounts':counts,'rows':checked_rows,
            'file':'extracted-table.csv'}
    csv_cell=lambda value:("'"+str(value)) if str(value).startswith(('=','+','-','@')) else value
    with (OUT/'extracted-table.csv').open('x',newline='',encoding='utf-8') as output:
        writer=csv.writer(output);header=['Source']
        for column in checked_columns: header.extend((csv_cell(column['label']),csv_cell(column['label']+' status'),csv_cell(column['label']+' evidence line')))
        writer.writerow(header)
        for row in checked_rows:
            values=[csv_cell(row['sourceLabel'])]
            for column in checked_columns:
                cell=row['cells'][column['key']];values.extend((csv_cell(cell['value']) if cell['value'] else '',cell['status'],cell['line'] or ''))
            writer.writerow(values)
    (OUT/'extraction.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'',f'- Sources checked: {len(prepared)}',f'- Rows extracted: {len(checked_rows)}',
            f'- Confirmed cells: {counts["confirmed"]}',f'- Uncertain cells: {counts["uncertain"]}',f'- Missing cells: {counts["missing"]}',
            '','| Source | Confirmed | Uncertain | Missing |','|---|---:|---:|---:|']
    for item in prepared:
        source_rows=[row for row in checked_rows if row['sourceId']==item['id']]
        source_counts={status:sum(cell['status']==status for row in source_rows for cell in row['cells'].values()) for status in counts}
        report.append(f'| {markdown(item["label"])} | {source_counts["confirmed"]} | {source_counts["uncertain"]} | {source_counts["missing"]} |')
    (OUT/'report.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    return result


def documents_brief(args,tools):
    exact(args,('title','sources','findings','questions','actions'))
    title=text(args['title'],160).strip();require(title)
    prepared=extracted_sources(args['sources'],tools)
    by_path={item['path']:item for item in prepared}
    def evidence(item):
        path=source(item['source']);record=by_path.get(str(path));require(record is not None)
        quote=text(item['evidence'],4000).strip();require(quote,'PRODUCTION_BRIEF_EVIDENCE_INVALID')
        offset=record['content'].find(quote);require(offset>=0,'PRODUCTION_BRIEF_EVIDENCE_INVALID')
        return record,quote,record['content'].count('\n',0,offset)+1
    findings=args['findings'];require(isinstance(findings,list) and 1<=len(findings)<=100)
    checked_findings=[]
    for item in findings:
        exact(item,('source','summary','evidence'));summary=text(item['summary'],1000).strip();require(summary)
        record,quote,line=evidence(item)
        checked_findings.append({'sourceId':record['id'],'sourceLabel':record['label'],'summary':summary,'evidence':quote,'line':line})
    questions=args['questions'];require(isinstance(questions,list) and len(questions)<=50)
    checked_questions=[]
    for item in questions:
        exact(item,('question','reason'));question=text(item['question'],500).strip();reason=text(item['reason'],1000).strip()
        require(question and reason);checked_questions.append({'question':question,'reason':reason})
    actions=args['actions'];require(isinstance(actions,list) and 1<=len(actions)<=100)
    checked_actions=[]
    for item in actions:
        exact(item,('source','task','evidence','owner','due'));task=text(item['task'],1000).strip();require(task)
        owner=item['owner'];require(owner is None or bool(text(owner,160).strip()))
        due=item['due']
        if due is not None:
            due=text(due,10);require(re.fullmatch(r'[0-9]{4}-[0-9]{2}-[0-9]{2}',due) is not None)
            try: datetime.date.fromisoformat(due)
            except ValueError: raise ValueError('PRODUCTION_BRIEF_DATE_INVALID')
        record,quote,line=evidence(item)
        checked_actions.append({'sourceId':record['id'],'sourceLabel':record['label'],'task':task,
                                'owner':owner.strip() if owner is not None else None,'due':due,
                                'evidence':quote,'line':line,'status':'open'})
    public_sources=[{key:item[key] for key in ('id','label','sourceSha256','textSha256')} for item in prepared]
    result={'kind':'attributed_brief','title':title,'sources':public_sources,'findings':checked_findings,
            'questions':checked_questions,'actions':checked_actions,
            'counts':{'sources':len(prepared),'findings':len(checked_findings),'questions':len(checked_questions),'actions':len(checked_actions)},
            'file':'document.docx'}
    paragraphs=['BRIEF']
    for index,item in enumerate(checked_findings,1):
        paragraphs.extend((f'Finding {index} · {item["sourceLabel"]}',item['summary'],f'Evidence · line {item["line"]} · {item["evidence"]}'))
    paragraphs.append('OPEN QUESTIONS')
    if checked_questions:
        for index,item in enumerate(checked_questions,1): paragraphs.extend((f'Question {index} · {item["question"]}',f'Why unresolved · {item["reason"]}'))
    else: paragraphs.append('No open questions identified.')
    paragraphs.append('ACTION PLAN')
    for index,item in enumerate(checked_actions,1):
        paragraphs.extend((f'Action {index} · {item["task"]}',f'Owner · {item["owner"] or "Unassigned"}',
                           f'Due · {item["due"] or "Not set"}',f'Source · {item["sourceLabel"]}',
                           f'Evidence · line {item["line"]} · {item["evidence"]}'))
    require(len(paragraphs)<=500,'PRODUCTION_BRIEF_LIMIT')
    documents({'title':title,'paragraphs':paragraphs},False)
    (OUT/'brief.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    return result


def documents_rewrite(args,tools):
    exact(args,('source','mode','target','preserve','segments'))
    path=source(args['source']);suffix=path.suffix.lower()
    if suffix=='.docx':
        parts=read_zip(path);node=xml(parts['word/document.xml'])
        units=[''.join(item.text or '' for item in paragraph.findall('.//w:t',NS))
               for paragraph in node.findall('.//w:body/w:p',NS)]
    elif suffix in ('.txt','.md'):
        try: content=path.read_bytes().decode('utf-8-sig')
        except UnicodeDecodeError: raise ValueError('PRODUCTION_REWRITE_FORMAT_UNSUPPORTED')
        content=content.replace('\r\n','\n').replace('\r','\n');units=content.split('\n')
    else: raise ValueError('PRODUCTION_REWRITE_FORMAT_UNSUPPORTED')
    units=[value for value in units if value.strip()]
    require(1<=len(units)<=200 and sum(len(value) for value in units)<=100000,'PRODUCTION_REWRITE_SOURCE_LIMIT')
    for value in units: text(value,4000)
    mode=args['mode'];require(isinstance(mode,str) and mode in ('translate','rewrite'))
    target=text(args['target'],240).strip();require(target and '\n' not in target and '\r' not in target)
    preserve=args['preserve'];require(isinstance(preserve,list) and len(preserve)<=30)
    checked_preserve=[]
    source_text='\n'.join(units)
    for value in preserve:
        value=text(value,120).strip()
        require(value and '\n' not in value and '\r' not in value and value in source_text and value not in checked_preserve)
        checked_preserve.append(value)
    segments=args['segments'];require(isinstance(segments,list) and len(segments)==len(units))
    checked=[]
    for expected,item in zip(units,segments):
        exact(item,('source','result'));original=text(item['source'],4000);result=text(item['result'],4000).strip()
        require(original==expected,'PRODUCTION_REWRITE_SOURCE_MISMATCH')
        require(result and '\n' not in result and '\r' not in result)
        checked.append({'index':len(checked)+1,'source':original,'result':result})
    require(any(item['source']!=item['result'] for item in checked),'PRODUCTION_REWRITE_NO_CHANGE')
    result_text='\n'.join(item['result'] for item in checked)
    fact_pattern=re.compile(r'https?://[^\s<>()\[\]{}]+|[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|(?<![\w])(?:[+-]?\d(?:[\d.,:/-]*\d)?(?:\s?(?:%|[A-Z]{3}))?)(?![\w])')
    automatic=[]
    for match in fact_pattern.finditer(source_text):
        value=match[0].rstrip('.,;:!?') if match[0].startswith(('http://','https://')) else match[0]
        if value and value not in automatic: automatic.append(value)
    protected=[]
    for value in automatic+checked_preserve:
        if value not in protected: protected.append(value)
    require(all(value in result_text for value in protected),'PRODUCTION_REWRITE_FACT_CHANGED')
    title=('Translation' if mode=='translate' else 'Rewrite')+' · '+target
    documents({'title':title,'paragraphs':[item['result'] for item in checked]},False)
    audit={'kind':'document_rewrite','mode':mode,'target':target,
           'sourceSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'sourceFormat':suffix[1:],
           'segmentCount':len(checked),'automaticFacts':automatic,'preservedTerms':checked_preserve,
           'segments':checked,'file':'document.docx'}
    (OUT/'rewrite-audit.json').write_text(json.dumps(audit,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    markdown=lambda value:''.join(('\\'+character) if character in '\\`*_{}[]()#+-.!|>' else character
                                  for character in html.escape(str(value),quote=True).replace('\r',' ').replace('\n',' '))
    report=['# '+markdown(title),'',f'- Source SHA-256: `{audit["sourceSha256"]}`',
            f'- Segments: {len(checked)}',f'- Protected facts/terms: {len(protected)}','']
    for item in checked:
        report.extend((f'## Segment {item["index"]}','','**Source**',markdown(item['source']),'',
                       '**Result**',markdown(item['result']),''))
    (OUT/'rewrite-comparison.md').write_text('\n'.join(report)+'\n',encoding='utf-8')
    return audit


def documents_template(args,tools):
    exact(args,('source','fields'))
    path=source(args['source']);require(path.suffix.lower()=='.docx','PRODUCTION_TEMPLATE_FORMAT_UNSUPPORTED')
    parts=read_zip(path);names=[name for name in parts if name=='word/document.xml' or re.fullmatch(r'word/(header|footer)[0-9]+\.xml',name)]
    require('word/document.xml' in names)
    pattern=re.compile(r'\{\{([a-z][a-z0-9_]{0,31})\}\}')
    documents={};placeholders=set()
    for name in names:
        node=xml(parts[name]);documents[name]=node
        for paragraph in node.findall('.//w:p',NS):
            visible=''.join(item.text or '' for item in paragraph.findall('.//w:t',NS))
            found=list(pattern.finditer(visible));placeholders.update(match[1] for match in found)
            remainder=pattern.sub('',visible)
            require('{{' not in remainder and '}}' not in remainder,'PRODUCTION_TEMPLATE_PLACEHOLDER_INVALID')
    require(1<=len(placeholders)<=100,'PRODUCTION_TEMPLATE_PLACEHOLDER_INVALID')
    fields=args['fields'];require(isinstance(fields,list) and 1<=len(fields)<=100)
    replacements={};checked=[]
    for field in fields:
        exact(field,('key','status','value'));key=text(field['key'],32);status=field['status']
        require(re.fullmatch(r'[a-z][a-z0-9_]{0,31}',key) is not None and key not in replacements)
        require(isinstance(status,str) and status in ('provided','missing'))
        if status=='missing':
            require(field['value'] is None);value=None;replacement=f'[MISSING: {key}]'
        else:
            value=text(field['value'],4000);require(value.strip() and '\n' not in value and '\r' not in value and '{{' not in value and '}}' not in value)
            replacement=value
        replacements[key]=replacement;checked.append({'key':key,'status':status,'value':value})
    require(set(replacements)==placeholders,'PRODUCTION_TEMPLATE_FIELDS_MISMATCH')
    for name,node in documents.items():
        for paragraph in node.findall('.//w:p',NS):
            items=paragraph.findall('.//w:t',NS);original=[item.text or '' for item in items];visible=''.join(original)
            matches=list(pattern.finditer(visible))
            if not matches: continue
            spans=[];offset=0
            for index,value in enumerate(original): spans.append((offset,offset+len(value),index));offset+=len(value)
            rendered=['' for _ in items]
            def copy(start,end):
                for left,right,index in spans:
                    if right<=start or left>=end: continue
                    rendered[index]+=visible[max(start,left):min(end,right)]
            cursor=0
            for match in matches:
                copy(cursor,match.start());index=next(index for left,right,index in spans if left<=match.start()<right)
                rendered[index]+=replacements[match[1]];cursor=match.end()
            copy(cursor,len(visible))
            for item,value in zip(items,rendered):
                item.text=value;item.set('{http://www.w3.org/XML/1998/namespace}space','preserve')
        parts[name]=ET.tostring(node,encoding='utf-8',xml_declaration=True)
    write_zip('filled-template.docx',parts)
    result={'kind':'filled_template','templateSha256':hashlib.sha256(path.read_bytes()).hexdigest(),
            'fieldCount':len(checked),'providedCount':sum(item['status']=='provided' for item in checked),
            'missingCount':sum(item['status']=='missing' for item in checked),'fields':checked,'file':'filled-template.docx'}
    (OUT/'template-fill.json').write_text(json.dumps(result,ensure_ascii=False,allow_nan=False),encoding='utf-8')
    return result


def captions_text(captions,duration,webvtt=False):
    require(isinstance(captions,list) and len(captions)<=1000)
    last=0;lines=[]
    for i,cap in enumerate(captions,1):
        exact(cap,('start','end','text'));start=number(cap['start'],last,duration);end=number(cap['end'],start+0.001,duration);last=end
        caption=text(cap['text'],500)
        # Plain SRT text only: do not allow ASS override tags or SRT/HTML formatting.
        caption=caption.replace('{','(').replace('}',')').replace('<','‹').replace('>','›').replace('\\','／')
        caption='\n'.join(line for line in caption.replace('\r','').split('\n') if line.strip())
        begin=stamp(start).replace(',','.') if webvtt else stamp(start)
        finish=stamp(end).replace(',','.') if webvtt else stamp(end)
        lines.append(f'{i}\n{begin} --> {finish}\n{caption}\n')
    body='\n'.join(lines)
    return 'WEBVTT\n\n'+body if webvtt else body


def captions_file(captions,duration,path):
    body=captions_text(captions,duration)
    path.write_text(body,encoding='utf-8')
    return body


def vtt_file(captions,duration,path):
    body=captions_text(captions,duration,True)
    path.write_text(body,encoding='utf-8')
    return body


def pdf_page_count(path,tools):
    require(path.suffix.lower()=='.pdf' and path.read_bytes()[:5]==b'%PDF-','PRODUCTION_PDF_FORMAT_UNSUPPORTED')
    raw=run_tool(tools,'pdfinfo',[str(path)]).decode('utf-8')
    match=re.search(r'(?m)^Pages:\s+([0-9]+)\s*$',raw)
    require(match is not None,'PRODUCTION_PDF_INFO_INVALID')
    encrypted=re.search(r'(?mi)^Encrypted:\s+([^\r\n]+)',raw)
    javascript=re.search(r'(?mi)^JavaScript:\s+([^\r\n]+)',raw)
    form=re.search(r'(?mi)^Form:\s+([^\r\n]+)',raw)
    require(encrypted is not None and encrypted[1].strip().lower()=='no','PRODUCTION_PDF_UNSAFE')
    require(javascript is None or javascript[1].strip().lower()=='no','PRODUCTION_PDF_UNSAFE')
    require(form is None or form[1].strip().lower()=='none','PRODUCTION_PDF_UNSAFE')
    pages=int(match[1]);require(1<=pages<=300,'PRODUCTION_PDF_PAGE_LIMIT')
    return pages


def pdf_text_pages(path,pages,tools):
    raw=run_tool(tools,'pdftotext',['-f','1','-l',str(pages),'-enc','UTF-8','-layout',str(path),'-']).decode('utf-8')
    values=raw.split('\f')
    while len(values)>pages and not values[-1].strip(): values.pop()
    require(len(values)<=pages,'PRODUCTION_PDF_TEXT_INVALID')
    values.extend(['']*(pages-len(values)))
    return [value.replace('\r\n','\n').replace('\r','\n') for value in values]


def pdf_page_png(path,page,prefix,tools):
    run_tool(tools,'pdftoppm',['-f',str(page),'-l',str(page),'-singlefile','-scale-to','1280','-png',str(path),str(prefix)])
    image=prefix.with_suffix('.png');require(image.is_file(),'PRODUCTION_PDF_RENDER_FAILED')
    checked,w,h=png(image.read_bytes());image.write_bytes(checked)
    return checked,w,h


def pdf_extract(args,tools):
    exact(args,('source','mode'),('language',));mode=args['mode']
    require(mode in ('text','ocr','auto'),'PRODUCTION_PDF_MODE_INVALID')
    language=text(args.get('language','eng'),40)
    require(re.fullmatch(r'[a-zA-Z_]{2,20}(\+[a-zA-Z_]{2,20})?',language) is not None)
    path=source(args['source']);pages=pdf_page_count(path,tools)
    require(pages<=(100 if mode=='text' else 12),'PRODUCTION_PDF_PAGE_LIMIT')
    extracted=pdf_text_pages(path,pages,tools) if mode!='ocr' else ['']*pages
    page_records=[];used_ocr=0
    for index,value in enumerate(extracted,1):
        method='text'
        if mode=='ocr' or (mode=='auto' and len(value.strip())<20):
            require(tools.get('pdftoppm') and tools.get('tesseract'),'PRODUCTION_OCR_ENGINE_REQUIRED')
            raster=ROOT/f'pdf-ocr-{index:03}'
            pdf_page_png(path,index,raster,tools)
            value=run_tool(tools,'tesseract',[str(raster.with_suffix('.png')),'stdout','-l',language,'--psm','3']).decode('utf-8')
            method='ocr';used_ocr+=1
        text(value,150000);extracted[index-1]=value
        page_records.append({'page':index,'method':method,'characters':len(value),
                             'textSha256':hashlib.sha256(value.encode()).hexdigest()})
    require(any(value.strip() for value in extracted),'PRODUCTION_PDF_TEXT_EMPTY')
    combined=''.join(f'[Page {index}]\n{value.rstrip()}\n\n' for index,value in enumerate(extracted,1))
    text(combined,150000)
    (OUT/'extracted.txt').write_text(combined,encoding='utf-8')
    result={'kind':'pdf_extraction','mode':mode,'language':language if used_ocr else None,
            'sourceSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'pageCount':pages,
            'textPageCount':pages-used_ocr,'ocrPageCount':used_ocr,'pages':page_records,
            'text':combined,'file':'extracted.txt'}
    (OUT/'extraction.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    return result


def pdf_merge(args,tools):
    exact(args,('sources',));requested=args['sources']
    require(isinstance(requested,list) and 2<=len(requested)<=12,'PRODUCTION_PDF_SOURCE_LIMIT')
    records=[];paths=[];total=0
    for index,item in enumerate(requested,1):
        exact(item,('source',));path=source(item['source']);pages=pdf_page_count(path,tools)
        require(pages<=100 and total+pages<=300,'PRODUCTION_PDF_PAGE_LIMIT')
        records.append({'index':index,'sourceSha256':hashlib.sha256(path.read_bytes()).hexdigest(),
                        'pages':pages,'startPage':total+1,'endPage':total+pages})
        paths.append(path);total+=pages
    output=OUT/'merged.pdf'
    run_tool(tools,'pdfunite',[*[str(path) for path in paths],str(output)])
    require(output.is_file() and output.stat().st_size<=MAX_FILE and output.read_bytes()[:5]==b'%PDF-',
            'PRODUCTION_PDF_MERGE_FAILED')
    require(pdf_page_count(output,tools)==total,'PRODUCTION_PDF_MERGE_PAGE_MISMATCH')
    merged_text=pdf_text_pages(output,total,tools)
    for path,record in zip(paths,records):
        source_text=pdf_text_pages(path,record['pages'],tools)
        merged_slice=merged_text[record['startPage']-1:record['endPage']]
        require(source_text==merged_slice,'PRODUCTION_PDF_MERGE_ORDER_MISMATCH')
        if not any(value.strip() for value in source_text):
            source_png,_,_=pdf_page_png(path,1,ROOT/f'pdf-source-{record["index"]:02}',tools)
            merged_png,_,_=pdf_page_png(output,record['startPage'],ROOT/f'pdf-merged-{record["index"]:02}',tools)
            require(source_png==merged_png,'PRODUCTION_PDF_MERGE_ORDER_MISMATCH')
            record['orderProof']='render'
            record['firstPageRenderSha256']=hashlib.sha256(source_png).hexdigest()
            if record['pages']>1:
                source_last,_,_=pdf_page_png(path,record['pages'],ROOT/f'pdf-source-last-{record["index"]:02}',tools)
                merged_last,_,_=pdf_page_png(output,record['endPage'],ROOT/f'pdf-merged-last-{record["index"]:02}',tools)
                require(source_last==merged_last,'PRODUCTION_PDF_MERGE_ORDER_MISMATCH')
                record['lastPageRenderSha256']=hashlib.sha256(source_last).hexdigest()
        else:
            record['orderProof']='text'
            record['textSha256']=hashlib.sha256('\f'.join(source_text).encode()).hexdigest()
    first,w1,h1=pdf_page_png(output,1,ROOT/'pdf-preview-first',tools)
    last,w2,h2=pdf_page_png(output,total,ROOT/'pdf-preview-last',tools)
    (OUT/'preview-first.png').write_bytes(first);(OUT/'preview-last.png').write_bytes(last)
    result={'kind':'pdf_merge','pageCount':total,'sourceCount':len(records),'verifiedOrder':True,
            'sources':records,'mergedSha256':hashlib.sha256(output.read_bytes()).hexdigest(),
            'previews':[{'page':1,'file':'preview-first.png','width':w1,'height':h1},
                        {'page':total,'file':'preview-last.png','width':w2,'height':h2}],
            'file':'merged.pdf'}
    (OUT/'merge-manifest.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
    return result


def ocr(args,tools):
    exact(args,('source',),('language',));language=text(args.get('language','eng'),40)
    require(re.fullmatch(r'[a-zA-Z_]{2,20}(\+[a-zA-Z_]{2,20})?',language) is not None)
    path=source(args['source']);require(path.suffix.lower() in ('.png','.jpg','.jpeg','.webp'),'PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
    # Force a checked single raster before OCR, excluding multipage containers and lists.
    if path.suffix.lower()=='.png':
        checked,_,_=png(path.read_bytes()); path=ROOT/'ocr-input.png';path.write_bytes(checked)
    else:
        checked=ROOT/'ocr-input.png'
        run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(path),'-i',str(path),'-vf','scale=4096:4096:force_original_aspect_ratio=decrease','-frames:v','1',str(checked)])
        png(checked.read_bytes());path=checked
    require(tools.get('tesseract'),'PRODUCTION_OCR_ENGINE_REQUIRED')
    result=run_tool(tools,'tesseract',[str(path),'stdout','-l',language,'--psm','3']).decode('utf-8')
    text(result,150000)
    value={'kind':'ocr','text':result,'language':language,'engine':'tesseract'}
    (OUT/'extracted.txt').write_text(result,encoding='utf-8');(OUT/'ocr.json').write_text(json.dumps(value,ensure_ascii=False),encoding='utf-8')
    return value


def transcribe(args,tools):
    exact(args,('source',),('language',));language=text(args.get('language','auto'),12)
    require(language=='auto' or re.fullmatch(r'[a-z]{2,3}',language) is not None)
    require(tools.get('whisper'),'PRODUCTION_TRANSCRIPTION_ENGINE_REQUIRED')
    model=tools.get('whisperModel');require(isinstance(model,str) and Path(model).is_absolute() and Path(model).is_file(),'PRODUCTION_TRANSCRIPTION_MODEL_REQUIRED')
    path=source(args['source']);source_digest=hashlib.sha256(path.read_bytes()).hexdigest();info=media_info(tools,path)
    duration=number(float(info.get('format',{}).get('duration',0)),0.02,1200)
    stream=next((s for s in info.get('streams',[]) if s.get('codec_type')=='audio'),None)
    require(stream is not None,'PRODUCTION_AUDIO_REQUIRED')
    wav=ROOT/'speech.wav'
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(path),'-i',str(path),'-vn','-ar','16000','-ac','1','-c:a','pcm_s16le',str(wav)])
    normalized=media_info(tools,wav);normalized_duration=float(normalized.get('format',{}).get('duration',0))
    normalized_stream=next((s for s in normalized.get('streams',[]) if s.get('codec_type')=='audio'),None)
    require(normalized_stream is not None and normalized_stream.get('sample_rate')=='16000'
            and normalized_stream.get('channels')==1,'PRODUCTION_TRANSCRIPTION_AUDIO_INVALID')
    require(abs(normalized_duration-duration)<max(0.2,duration*.005),'PRODUCTION_DURATION_MISMATCH')
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error',*input_format(wav),'-i',str(wav),'-map','0:a:0','-f','null','-'])
    target=ROOT/'speech-result'
    run_tool(tools,'whisper',['-m',model,'-f',str(wav),'-l',language,'-oj','-of',str(target),'-np'])
    result=target.with_suffix('.json');require(result.is_file() and result.stat().st_size<=1024*1024,'PRODUCTION_TRANSCRIPTION_INVALID')
    native=json.loads(result.read_text(encoding='utf-8'));items=native.get('transcription')
    require(isinstance(items,list) and len(items)<=1000,'PRODUCTION_TRANSCRIPTION_INVALID')
    segments=[];last=0
    for item in items:
        offsets=item.get('offsets',{});raw_start=offsets.get('from');raw_end=offsets.get('to')
        require(type(raw_start) in (int,float) and type(raw_end) in (int,float)
                and math.isfinite(raw_start) and math.isfinite(raw_end),
                'PRODUCTION_TRANSCRIPTION_INVALID')
        start=raw_start/1000;end=min(raw_end/1000,duration)
        caption=text(item.get('text'),500).strip()
        require(last<=start<=duration and start<end<=duration and raw_end<=duration*1000+50
                and caption,'PRODUCTION_TRANSCRIPTION_INVALID');last=end
        segments.append({'start':start,'end':end,'text':caption})
    result_language=text(native.get('result',{}).get('language',language),12)
    require(re.fullmatch(r'[a-z]{2,3}|auto',result_language) is not None,'PRODUCTION_TRANSCRIPTION_INVALID')
    value={'kind':'transcript','text':'\n'.join(s['text'] for s in segments),'segments':segments,
           'segmentCount':len(segments),'durationSeconds':duration,'requestedLanguage':language,
           'language':result_language,'engine':'whisper.cpp','sourceSha256':source_digest}
    text(value['text'],150000)
    transcript_json=OUT/'transcript.json';transcript_text=OUT/'transcript.txt'
    srt=OUT/'captions.srt';vtt=OUT/'captions.vtt'
    transcript_json.write_text(json.dumps(value,ensure_ascii=False),encoding='utf-8');transcript_text.write_text(value['text'],encoding='utf-8')
    srt_body=captions_file(segments,duration,srt);vtt_body=vtt_file(segments,duration,vtt)
    require(srt.read_text(encoding='utf-8')==srt_body and vtt.read_text(encoding='utf-8')==vtt_body,
            'PRODUCTION_TRANSCRIPTION_OUTPUT_MISMATCH')
    require(hashlib.sha256(path.read_bytes()).hexdigest()==source_digest,'PRODUCTION_SOURCE_CHANGED')
    artifacts={item.name:hashlib.sha256(item.read_bytes()).hexdigest()
               for item in (transcript_json,transcript_text,srt,vtt)}
    manifest={'kind':'transcription','engine':'whisper.cpp','requestedLanguage':language,
              'detectedLanguage':result_language,'source':{'sha256':source_digest,
              'durationSeconds':duration,'codec':stream.get('codec_name'),
              'sampleRate':stream.get('sample_rate'),'channels':stream.get('channels')},
              'normalizedAudio':{'sha256':hashlib.sha256(wav.read_bytes()).hexdigest(),
              'durationSeconds':normalized_duration,'codec':normalized_stream.get('codec_name'),
              'sampleRate':normalized_stream.get('sample_rate'),'channels':normalized_stream.get('channels')},
              'segmentCount':len(segments),'artifacts':artifacts,
              'verification':{'originalPreserved':True,'durationMatched':True,
              'normalizedAudioDecoded':True,'segmentTimelineValid':True,
              'subtitleFilesReopened':True,'textMatched':True,
              'contentAccuracyReviewed':False,'uncertainPassagesReviewed':False}}
    (OUT/'transcript-manifest.json').write_text(
        json.dumps(manifest,ensure_ascii=False,indent=2,allow_nan=False),encoding='utf-8')
    return value


def render(args,tools):
    exact(args,('source',),('pages',));pages=args.get('pages',[1])
    require(isinstance(pages,list) and 1<=len(pages)<=8 and len(set(pages))==len(pages)
            and all(type(p) is int and 1<=p<=100 for p in pages))
    path=source(args['source']);require(path.suffix.lower() in ('.pptx','.docx','.xlsx','.pdf'),'PRODUCTION_RENDER_FORMAT_UNSUPPORTED')
    require(tools.get('pdftoppm'),'PRODUCTION_PAGE_RENDERER_REQUIRED')
    if path.suffix.lower()!='.pdf':
        require(tools.get('libreoffice'),'PRODUCTION_OFFICE_RENDERER_REQUIRED')
        parts=read_zip(path)
        require(not any(any(bad in name.lower() for bad in ('vbaproject','activex/','embeddings/')) for name in parts),'PRODUCTION_UNSAFE_DOCUMENT')
        for name,part in parts.items():
            require(not name.lower().endswith(('.svg','.html','.htm','.js','.bin')),'PRODUCTION_UNSAFE_DOCUMENT')
            if name.endswith(('.xml','.rels')):
                for node in xml(part).iter():
                    local=node.tag.rsplit('}',1)[-1]
                    require(local not in ('instrText','fldSimple','altChunk','ddeLink','oleLink','externalBook'),'PRODUCTION_EXTERNAL_REFERENCE')
                    if local=='f':
                        formula=(node.text or '').upper()
                        require(not any(word in formula for word in ('WEBSERVICE','HYPERLINK','DDE','RTD','[',']')),'PRODUCTION_EXTERNAL_REFERENCE')
        profile=ROOT/'office-profile';(profile/'user').mkdir(parents=True)
        # A new private profile: highest macro security and no external-link updates.
        (profile/'user'/'registrymodifications.xcu').write_text('<?xml version="1.0"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item><item oor:path="/org.openoffice.Office.Common/Load"><prop oor:name="UpdateMode" oor:op="fuse"><value>0</value></prop></item></oor:items>')
        directory=ROOT/'rendered';directory.mkdir()
        run_tool(tools,'libreoffice',['-env:UserInstallation='+profile.as_uri(),'--headless','--nologo','--nodefault','--norestore','--convert-to','pdf','--outdir',str(directory),str(path)])
        pdf=directory/(path.stem+'.pdf');require(pdf.is_file() and pdf.stat().st_size<=MAX_FILE,'PRODUCTION_RENDER_FAILED')
        (OUT/'document.pdf').write_bytes(pdf.read_bytes())
    else: (OUT/'document.pdf').write_bytes(path.read_bytes())
    require((OUT/'document.pdf').read_bytes().startswith(b'%PDF-'),'PRODUCTION_RENDER_FAILED')
    output=[]
    for page in pages:
        prefix=OUT/f'page-{page}'
        run_tool(tools,'pdftoppm',['-f',str(page),'-l',str(page),'-singlefile','-scale-to','1280','-png',str(OUT/'document.pdf'),str(prefix)])
        image=prefix.with_suffix('.png');require(image.is_file(),'PRODUCTION_PAGE_NOT_FOUND')
        checked,w,h=png(image.read_bytes());image.write_bytes(checked)
        output.append({'id':f'page:{page}','page':page,'file':image.name,'width':w,'height':h})
    return {'kind':'rendered_pages','pages':output,'pdf':'document.pdf','fidelity':'rendered'}


def frame(args,tools):
    exact(args,('source','time'));path=source(args['source']);info=media_info(tools,path)
    duration=number(float(info.get('format',{}).get('duration',0)),0.001,86400)
    require(any(s.get('codec_type')=='video' for s in info.get('streams',[])),'PRODUCTION_VIDEO_REQUIRED')
    at=number(args['time'],0,max(0,duration-0.001))
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(path),'-ss',str(at),'-i',str(path),'-frames:v','1','-vf','scale=1280:1280:force_original_aspect_ratio=decrease',str(OUT/'preview.png')])
    require((OUT/'preview.png').is_file(),'PRODUCTION_FRAME_UNAVAILABLE')
    checked,w,h=png((OUT/'preview.png').read_bytes());(OUT/'preview.png').write_bytes(checked)
    return {'kind':'frame','time':at,'durationSeconds':duration,'file':'preview.png','width':w,'height':h}



def caption_cards(captions,tools):
    require(tools.get('libreoffice') and tools.get('pdftoppm'),'PRODUCTION_CAPTION_RENDERER_REQUIRED')
    require(len(captions)<=20 and all(len(c['text'])<=120 for c in captions),'PRODUCTION_CAPTION_RENDERER_LIMIT')
    profile=ROOT/'caption-profile';(profile/'user').mkdir(parents=True)
    (profile/'user'/'registrymodifications.xcu').write_text('<?xml version="1.0"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item></oor:items>')
    cards=[]
    for i,caption in enumerate(captions):
        lines=textwrap.wrap(' '.join(caption['text'].split()),width=40,break_long_words=True,break_on_hyphens=False)
        require(1<=len(lines)<=3,'PRODUCTION_CAPTION_RENDERER_LIMIT')
        paragraphs=''.join('<w:p><w:pPr><w:jc w:val="center"/><w:spacing w:before="0" w:after="0"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:sz w:val="32"/></w:rPr><w:t xml:space="preserve">'+escape(line)+'</w:t></w:r></w:p>' for line in lines)
        parts={'[Content_Types].xml':content_types([('word/document.xml','wordprocessingml.document.main')]),'_rels/.rels':rels([(1,'officeDocument','word/document.xml')]),'word/document.xml':f'<w:document xmlns:w="{NS["w"]}"><w:body>{paragraphs}<w:sectPr><w:pgSz w:w="9600" w:h="1800"/><w:pgMar w:top="160" w:right="320" w:bottom="160" w:left="320" w:header="0" w:footer="0"/></w:sectPr></w:body></w:document>'}
        doc=ROOT/f'caption-{i}.docx'
        with zipfile.ZipFile(doc,'x',compression=zipfile.ZIP_DEFLATED) as archive:
            for name,body in parts.items():archive.writestr(name,body)
        read_zip(doc)
        run_tool(tools,'libreoffice',['-env:UserInstallation='+profile.as_uri(),'--headless','--nologo','--nodefault','--norestore','--convert-to','pdf','--outdir',str(ROOT),str(doc)])
        pdf=doc.with_suffix('.pdf');require(pdf.is_file() and pdf.stat().st_size<=MAX_FILE,'PRODUCTION_CAPTION_RENDER_FAILED')
        prefix=ROOT/f'caption-card-{i}'
        run_tool(tools,'pdftoppm',['-f','1','-l','2','-scale-to-x','640','-scale-to-y','-1','-png',str(pdf),str(prefix)])
        pages=list(ROOT.glob(prefix.name+'-*.png'));require(len(pages)==1,'PRODUCTION_CAPTION_RENDERER_LIMIT')
        checked,w,h=png(pages[0].read_bytes());require(w==640 and 1<=h<=160,'PRODUCTION_CAPTION_RENDER_FAILED')
        pages[0].write_bytes(checked);cards.append(pages[0])
    return cards


def video_title_card(title,tools):
    require(tools.get('libreoffice') and tools.get('pdftoppm'),'PRODUCTION_CAPTION_RENDERER_REQUIRED')
    lines=textwrap.wrap(' '.join(title.split()),width=48,break_long_words=True,break_on_hyphens=False)
    require(1<=len(lines)<=3,'PRODUCTION_VIDEO_TITLE_RANGE_INVALID')
    profile=ROOT/'video-title-profile';(profile/'user').mkdir(parents=True)
    (profile/'user'/'registrymodifications.xcu').write_text('<?xml version="1.0"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item></oor:items>')
    paragraphs=''.join('<w:p><w:pPr><w:jc w:val="center"/><w:spacing w:before="0" w:after="0"/></w:pPr><w:r><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/><w:b/><w:color w:val="FFFFFF"/><w:sz w:val="38"/></w:rPr><w:t xml:space="preserve">'+escape(line)+'</w:t></w:r></w:p>' for line in lines)
    table='<w:tbl><w:tblPr><w:tblW w:w="9600" w:type="dxa"/><w:tblLayout w:type="fixed"/><w:tblBorders><w:top w:val="nil"/><w:left w:val="nil"/><w:bottom w:val="single" w:sz="28" w:color="FF7A2F"/><w:right w:val="nil"/><w:insideH w:val="nil"/><w:insideV w:val="nil"/></w:tblBorders></w:tblPr><w:tblGrid><w:gridCol w:w="9600"/></w:tblGrid><w:tr><w:trPr><w:trHeight w:val="1700" w:hRule="exact"/></w:trPr><w:tc><w:tcPr><w:tcW w:w="9600" w:type="dxa"/><w:shd w:fill="15223D"/><w:vAlign w:val="center"/><w:tcMar><w:left w:w="420" w:type="dxa"/><w:right w:w="420" w:type="dxa"/></w:tcMar></w:tcPr>'+paragraphs+'</w:tc></w:tr></w:tbl>'
    parts={'[Content_Types].xml':content_types([('word/document.xml','wordprocessingml.document.main')]),'_rels/.rels':rels([(1,'officeDocument','word/document.xml')]),'word/document.xml':f'<w:document xmlns:w="{NS["w"]}"><w:body>{table}<w:sectPr><w:pgSz w:w="9600" w:h="1800"/><w:pgMar w:top="0" w:right="0" w:bottom="0" w:left="0" w:header="0" w:footer="0"/></w:sectPr></w:body></w:document>'}
    doc=ROOT/'video-title.docx'
    with zipfile.ZipFile(doc,'x',compression=zipfile.ZIP_DEFLATED) as archive:
        for name,body in parts.items():archive.writestr(name,body)
    read_zip(doc)
    run_tool(tools,'libreoffice',['-env:UserInstallation='+profile.as_uri(),'--headless','--nologo','--nodefault','--norestore','--convert-to','pdf','--outdir',str(ROOT),str(doc)])
    pdf=doc.with_suffix('.pdf');require(pdf.is_file() and pdf.stat().st_size<=MAX_FILE,'PRODUCTION_CAPTION_RENDER_FAILED')
    prefix=ROOT/'video-title-card'
    run_tool(tools,'pdftoppm',['-f','1','-l','2','-scale-to-x','960','-scale-to-y','-1','-png',str(pdf),str(prefix)])
    pages=list(ROOT.glob(prefix.name+'-*.png'));require(len(pages)==1,'PRODUCTION_CAPTION_RENDERER_LIMIT')
    checked,w,h=png(pages[0].read_bytes());require(w==960 and 1<=h<=220,'PRODUCTION_CAPTION_RENDER_FAILED')
    pages[0].write_bytes(checked);return pages[0]


def video_highlights(args,tools):
    exact(args,('source','targetSeconds','clipSeconds','emphasis'))
    emphasis=args['emphasis'];require(emphasis in ('balanced','visual','audio'),'PRODUCTION_VIDEO_EMPHASIS_INVALID')
    path=source(args['source']);require(path.suffix.lower() in ('.mp4','.mov','.mkv','.webm'),'PRODUCTION_VIDEO_FORMAT_UNSUPPORTED')
    info=media_info(tools,path);streams=info.get('streams',[])
    require(any(stream.get('codec_type')=='video' for stream in streams),'PRODUCTION_VIDEO_REQUIRED')
    duration=number(float(info.get('format',{}).get('duration',0)),2,1200)
    clip_seconds=number(args['clipSeconds'],2,min(30,duration))
    target_seconds=number(args['targetSeconds'],clip_seconds,min(180,duration))
    clip_count=round(target_seconds/clip_seconds)
    require(1<=clip_count<=12 and abs(clip_count*clip_seconds-target_seconds)<0.001,'PRODUCTION_VIDEO_HIGHLIGHT_DURATION_INVALID')
    has_audio=any(stream.get('codec_type')=='audio' for stream in streams)
    require(emphasis!='audio' or has_audio,'PRODUCTION_AUDIO_REQUIRED')

    visual=run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error',*input_format(path),'-i',str(path),
        '-an','-vf','fps=1,scale=32:18,format=gray','-pix_fmt','gray','-f','rawvideo','-'])
    frame_size=32*18;require(len(visual)>=frame_size,'PRODUCTION_VIDEO_ANALYSIS_FAILED')
    frames=[visual[offset:offset+frame_size] for offset in range(0,len(visual)-frame_size+1,frame_size)]
    audio=b''
    if has_audio:
        audio=run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error',*input_format(path),'-i',str(path),
            '-vn','-ac','1','-ar','200','-f','s16le','-'])
        require(len(audio)>=2 and len(audio)%2==0,'PRODUCTION_VIDEO_ANALYSIS_FAILED')
    samples=struct.unpack('<'+str(len(audio)//2)+'h',audio) if audio else ()

    stride=max(1.0,clip_seconds/2)
    starts=[];cursor=0.0;last=max(0.0,duration-clip_seconds)
    while cursor<=last+0.001 and len(starts)<240:
        starts.append(round(min(cursor,last),3));cursor+=stride
    if not starts or abs(starts[-1]-last)>0.001: starts.append(round(last,3))
    candidates=[]
    for start in starts:
        end=start+clip_seconds
        first=max(1,int(math.floor(start))+1);last_frame=min(len(frames)-1,int(math.ceil(end)))
        changes=[]
        for index in range(first,last_frame+1):
            previous,current=frames[index-1],frames[index]
            changes.append(sum(abs(a-b) for a,b in zip(previous,current))/(frame_size*255))
        visual_score=sum(changes)/len(changes) if changes else 0.0
        audio_start=max(0,int(start*200));audio_end=min(len(samples),int(end*200))
        window=samples[audio_start:audio_end]
        audio_score=math.sqrt(sum(value*value for value in window)/len(window))/32768 if window else 0.0
        candidates.append({'start':start,'end':round(end,3),'visualScore':visual_score,'audioScore':audio_score})
    require(candidates,'PRODUCTION_VIDEO_ANALYSIS_FAILED')
    def normalized(key,value):
        values=[candidate[key] for candidate in candidates];low=min(values);high=max(values)
        return 0.0 if high-low<1e-12 else (value-low)/(high-low)
    weights={'balanced':(.55,.45),'visual':(1,0),'audio':(0,1)}[emphasis]
    for candidate in candidates:
        candidate['combinedScore']=weights[0]*normalized('visualScore',candidate['visualScore'])+weights[1]*normalized('audioScore',candidate['audioScore'])
    ranked=sorted(candidates,key=lambda item:(-item['combinedScore'],-item['visualScore'],-item['audioScore'],item['start']))
    selected=[]
    for candidate in ranked:
        if all(candidate['end']<=item['start']+0.001 or candidate['start']>=item['end']-0.001 for item in selected):
            selected.append(candidate)
            if len(selected)==clip_count: break
    require(len(selected)==clip_count,'PRODUCTION_VIDEO_HIGHLIGHT_DURATION_INVALID')
    selected.sort(key=lambda item:item['start'])
    for item in selected:
        for key in ('visualScore','audioScore','combinedScore'): item[key]=round(item[key],6)
    clips=[{'source':args['source'],'start':item['start'],'end':item['end']} for item in selected]
    media({'clips':clips,'width':1280,'height':720},'video.edit',tools)
    output=OUT/'highlights.mp4';(OUT/'video.mp4').rename(output)
    (OUT/'edit-recipe.json').rename(OUT/'edit-plan.json')
    output_info=media_info(tools,output);actual=float(output_info.get('format',{}).get('duration',0))
    output_streams=output_info.get('streams',[])
    rendered_video=[stream for stream in output_streams if stream.get('codec_type')=='video']
    require(len(rendered_video)==1 and (rendered_video[0].get('width'),rendered_video[0].get('height'))==(1280,720),'PRODUCTION_OUTPUT_INVALID')
    require(any(stream.get('codec_type')=='audio' for stream in output_streams)==has_audio,'PRODUCTION_OUTPUT_INVALID')
    require(abs(actual-target_seconds)<max(0.2,target_seconds*0.005),'PRODUCTION_DURATION_MISMATCH')
    decode=['-nostdin','-hide_banner','-loglevel','error',*input_format(output),'-i',str(output),'-map','0:v:0']
    if has_audio: decode+=['-map','0:a:0']
    run_tool(tools,'ffmpeg',decode+['-f','null','-'])
    frames_out=[];position=0.0
    for index,item in enumerate(selected,1):
        image=OUT/f'highlight-frame-{index:02}.png';at=position+clip_seconds/2
        run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(output),'-ss',str(at),'-i',str(output),'-frames:v','1',str(image)])
        checked,w,h=png(image.read_bytes(),(1280,720));image.write_bytes(checked)
        frames_out.append({'selection':index,'sourceSeconds':round((item['start']+item['end'])/2,3),'outputSeconds':round(at,3),'file':image.name,'width':w,'height':h})
        position+=clip_seconds
    digest=lambda value:hashlib.sha256(value.read_bytes()).hexdigest()
    manifest={'kind':'video-highlights','emphasis':emphasis,'sourceDurationSeconds':duration,
        'targetSeconds':target_seconds,'clipSeconds':clip_seconds,'sourceHadAudio':has_audio,
        'analysis':{'visualSamplesPerSecond':1,'visualSampleSize':[32,18],'audioSamplesPerSecond':200 if has_audio else 0,
            'candidateCount':len(candidates),'weights':{'visual':weights[0],'audio':weights[1]}},
        'selections':selected,'sourceSha256':digest(path),'outputSha256':digest(output),'output':output_info,'frames':frames_out,
        'verification':{'fullVideoDecoded':True,'durationMatched':True,'dimensionsMatched':True,
            'rangesNonOverlapping':True,'selectionFramesRetained':True,'humanMeaningReviewRequired':True}}
    (OUT/'highlight-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
    return manifest


def compose_video(args,tools):
    exact(args,('source','image','audio','title','titleSeconds','overlay','audioMode','audioLevel'))
    exact(args['image'],('source',));exact(args['audio'],('source',))
    title=text(args['title'],120);title_seconds=number(args['titleSeconds'],0.5,10)
    overlay=args['overlay'];exact(overlay,('start','end','position'))
    position=overlay['position'];require(position in ('top-left','top-right','bottom-left','bottom-right'),'PRODUCTION_VIDEO_POSITION_INVALID')
    mode=args['audioMode'];require(mode in ('mix','replace'),'PRODUCTION_VIDEO_AUDIO_MODE_INVALID')
    level=number(args['audioLevel'],0.05,1)
    video_path=source(args['source']);image_path=source(args['image']['source']);audio_path=source(args['audio']['source'])
    require(video_path.suffix.lower() in ('.mp4','.mov','.mkv','.webm'),'PRODUCTION_VIDEO_FORMAT_UNSUPPORTED')
    require(image_path.suffix.lower() in ('.png','.jpg','.jpeg','.webp'),'PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
    require(audio_path.suffix.lower() in ('.wav','.mp3','.m4a'),'PRODUCTION_AUDIO_FORMAT_UNSUPPORTED')
    video_info=media_info(tools,video_path);image_info=media_info(tools,image_path);audio_info=media_info(tools,audio_path)
    duration=number(float(video_info.get('format',{}).get('duration',0)),0.5,1200)
    video_streams=[stream for stream in video_info.get('streams',[]) if stream.get('codec_type')=='video']
    require(video_streams,'PRODUCTION_VIDEO_REQUIRED')
    image_streams=[stream for stream in image_info.get('streams',[]) if stream.get('codec_type')=='video']
    require(len(image_streams)==1 and 1<=int(image_streams[0].get('width',0))<=4096
            and 1<=int(image_streams[0].get('height',0))<=4096,'PRODUCTION_IMAGE_FORMAT_UNSUPPORTED')
    require(any(stream.get('codec_type')=='audio' for stream in audio_info.get('streams',[])),'PRODUCTION_AUDIO_REQUIRED')
    audio_duration=number(float(audio_info.get('format',{}).get('duration',0)),0.05,3600)
    source_has_audio=any(stream.get('codec_type')=='audio' for stream in video_info.get('streams',[]))
    require(mode!='mix' or source_has_audio,'PRODUCTION_VIDEO_AUDIO_MIX_REQUIRES_SOURCE')
    start=number(overlay['start'],0,max(0,duration-0.05));end=number(overlay['end'],start+0.05,duration)
    require(title_seconds<=duration,'PRODUCTION_VIDEO_TITLE_RANGE_INVALID')
    card=video_title_card(title,tools)
    x,y={'top-left':('48','48'),'top-right':('main_w-overlay_w-48','48'),
         'bottom-left':('48','main_h-overlay_h-48'),
         'bottom-right':('main_w-overlay_w-48','main_h-overlay_h-48')}[position]
    common=['-nostdin','-hide_banner','-loglevel','error','-n']
    command=common+input_format(video_path)+['-i',str(video_path)]+input_format(image_path)+['-i',str(image_path)]
    command+=['-stream_loop','-1']+input_format(audio_path)+['-i',str(audio_path)]+input_format(card)+['-i',str(card)]
    graph=[
        '[0:v:0]scale=1280:720:force_original_aspect_ratio=decrease,pad=1280:720:(ow-iw)/2:(oh-ih)/2:color=0x090D18,fps=30,setsar=1[base]',
        '[1:v:0]scale=260:260:force_original_aspect_ratio=decrease[image]',
        f"[base][image]overlay={x}:{y}:enable='gte(t,{start})*lt(t,{end})'[withimage]",
        '[3:v:0]scale=960:-1[title]',
        f"[withimage][title]overlay=(main_w-overlay_w)/2:48:enable='lt(t,{title_seconds})'[v]",
        f'[2:a:0]volume={level},aresample=48000,aformat=channel_layouts=stereo,atrim=duration={duration},asetpts=PTS-STARTPTS[music]',
    ]
    if mode=='mix':
        graph+=[f'[0:a:0]aresample=48000,aformat=channel_layouts=stereo,apad=pad_dur={duration},atrim=duration={duration},asetpts=PTS-STARTPTS[original]',
                '[original][music]amix=inputs=2:duration=first:dropout_transition=0[a]']
    else: graph+=['[music]anull[a]']
    output=OUT/'composed-video.mp4'
    run_tool(tools,'ffmpeg',command+['-filter_complex',';'.join(graph),'-map','[v]','-map','[a]','-t',str(duration),
        '-c:v','mpeg4','-q:v','3','-pix_fmt','yuv420p','-c:a','aac','-ar','48000','-ac','2','-movflags','+faststart',str(output)])
    info=media_info(tools,output);streams=info.get('streams',[])
    rendered_video=[stream for stream in streams if stream.get('codec_type')=='video']
    require(len(rendered_video)==1 and (rendered_video[0].get('width'),rendered_video[0].get('height'))==(1280,720),'PRODUCTION_OUTPUT_INVALID')
    require(any(stream.get('codec_type')=='audio' for stream in streams),'PRODUCTION_OUTPUT_INVALID')
    actual=float(info.get('format',{}).get('duration',0));require(abs(actual-duration)<max(0.2,duration*0.005),'PRODUCTION_DURATION_MISMATCH')
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error',*input_format(output),'-i',str(output),'-map','0:v:0','-map','0:a:0','-f','null','-'])
    moments=[('start',0),('middle',duration/2),('overlay',(start+end)/2),('end',max(0,duration-0.5))]
    frames=[]
    for label,at in moments:
        image=OUT/f'video-frame-{label}.png'
        run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(output),'-ss',str(at),'-i',str(output),'-frames:v','1',str(image)])
        checked,w,h=png(image.read_bytes(),(1280,720));image.write_bytes(checked)
        frames.append({'label':label,'seconds':at,'file':image.name,'width':w,'height':h})
    digest=lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    manifest={'kind':'composed-video','title':title,'durationSeconds':actual,'outputSha256':digest(output),
        'sourceSha256':digest(video_path),'imageSha256':digest(image_path),'audioSha256':digest(audio_path),
        'titleSeconds':title_seconds,'overlay':{'start':start,'end':end,'position':position},
        'audio':{'mode':mode,'level':level,'sourceHadAudio':source_has_audio,'sourceDurationSeconds':audio_duration},
        'streams':streams,'frames':frames,'verification':{'fullVideoDecoded':True,'durationMatched':True,
        'dimensionsMatched':True,'audioPresent':True,'placementFramesRetained':True,'visualPlacementReviewed':False}}
    (OUT/'composition-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
    return manifest


def package(args,tools):
    exact(args,('clips','aspects','captions','title'),('description','hashtags'))
    aspects=args['aspects'];require(isinstance(aspects,list) and 1<=len(aspects)<=3 and len(set(aspects))==len(aspects)
                                 and all(a in ('portrait','square','landscape') for a in aspects))
    title=text(args['title'],160);description=text(args.get('description',''),10000);tags=args.get('hashtags',[])
    require(isinstance(tags,list) and len(tags)<=20)
    for tag in tags: require(re.fullmatch(r'#[\w]{1,60}',text(tag,64)) is not None)
    require(isinstance(args['clips'],list) and 1<=len(args['clips'])<=12)
    duration=0
    for clip in args['clips']:
        exact(clip,('source','start','end'));start=number(clip['start'],0,86400);end=number(clip['end'],start+0.02,start+1200);duration+=end-start
    require(duration<=1200)
    captions_file(args['captions'],duration,OUT/'captions.srt')
    caption_renderer=None;cards=[]
    if args['captions']:
        filters=run_tool(tools,'ffmpeg',['-hide_banner','-filters']).decode('utf-8',errors='replace')
        caption_renderer='libass' if re.search(r'\bsubtitles\s',filters) is not None else 'office_overlay'
        if caption_renderer=='office_overlay':
            require(re.search(r'\boverlay\s',filters) is not None,'PRODUCTION_CAPTION_RENDERER_REQUIRED')
            cards=caption_cards(args['captions'],tools)
    # Existing checked clip path supplies range validation, safe demuxers and normalized audio.
    media({'clips':args['clips'],'width':1280,'height':720},'video.edit',tools)
    base=ROOT/'package-base.mp4';(OUT/'video.mp4').rename(base)
    for name in ('validation.json','edit-recipe.json'): (OUT/name).unlink(missing_ok=True)
    result=[]
    for aspect in aspects:
        w,h={'portrait':(720,1280),'square':(720,720),'landscape':(1280,720)}[aspect]
        vf=f'scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1'
        if caption_renderer=='libass': vf+=",subtitles=filename=outputs/captions.srt:force_style='FontName=Arial,FontSize=22,Outline=2,MarginV=32'"
        video=OUT/f'{aspect}.mp4'
        encode=['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(base),'-i',str(base)]
        if cards:
            for card in cards: encode+=input_format(card)+['-i',str(card)]
            graph=[f'[0:v]{vf}[v0]']
            for i,caption in enumerate(args['captions']):
                graph.append(f"[v{i}][{i+1}:v]overlay=(main_w-overlay_w)/2:main_h-overlay_h-32:enable='gte(t,{caption['start']})*lt(t,{caption['end']})'[v{i+1}]")
            encode+=['-filter_complex',';'.join(graph),'-map',f'[v{len(cards)}]','-map','0:a?']
        else: encode+=['-vf',vf]
        run_tool(tools,'ffmpeg',encode+['-t',str(duration),'-c:v','mpeg4','-q:v','3','-pix_fmt','yuv420p','-c:a','aac','-movflags','+faststart',str(video)])
        info=media_info(tools,video);streams=[s for s in info.get('streams',[]) if s.get('codec_type')=='video']
        require(len(streams)==1 and (streams[0].get('width'),streams[0].get('height'))==(w,h),'PRODUCTION_OUTPUT_INVALID')
        actual=float(info.get('format',{}).get('duration',0));require(abs(actual-duration)<max(0.2,duration*0.005),'PRODUCTION_DURATION_MISMATCH')
        cover=OUT/f'{aspect}-cover.png'
        run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(video),'-ss',str(min(duration/2,1.0)),'-i',str(video),'-frames:v','1',str(cover)])
        checked,_,_=png(cover.read_bytes(),(w,h));cover.write_bytes(checked)
        result.append({'aspect':aspect,'file':video.name,'cover':cover.name,'width':w,'height':h,'durationSeconds':actual,'captionsBurned':bool(args['captions']),'captionRenderer':caption_renderer})
    (OUT/'content.txt').write_text(title+'\n\n'+description+'\n\n'+' '.join(tags),encoding='utf-8')
    value={'kind':'video_package','title':title,'durationSeconds':duration,'exports':result,'captions':'captions.srt','content':'content.txt'}
    (OUT/'package.json').write_text(json.dumps(value,ensure_ascii=False),encoding='utf-8')
    return value


STYLE='''*{box-sizing:border-box}body{font:16px system-ui,sans-serif;background:#f5f7fb;color:#152333;margin:0;padding:32px}main{max-width:1080px;margin:auto}h1{font-size:32px;margin:0 0 12px}p{color:#506273}section{background:white;border:1px solid #d5dee8;border-radius:16px;padding:24px;margin:20px 0;overflow:auto}input,button{font:inherit;padding:10px 12px;border:1px solid #bccbd8;border-radius:8px;background:white;color:#152333}input:focus,button:focus{outline:3px solid #5296ff;outline-offset:2px}table{border-collapse:collapse;width:100%;margin-top:16px}td,th{text-align:left;padding:12px;border-bottom:1px solid #e3e9ef}th button{border:0;font-weight:bold}label{display:flex;gap:16px;align-items:center;flex-wrap:wrap;margin:12px 0}progress{height:28px;flex:1;min-width:120px;accent-color:#4278df}.metric{font-size:24px;font-weight:bold}output{display:block;margin:20px 0}footer{color:#506273;font-size:13px}@media(max-width:600px){body{padding:16px}section{padding:16px}h1{font-size:26px}}'''
JS='''"use strict";const d=JSON.parse(document.getElementById("data").textContent);const root=document.getElementById("tool");function el(tag,value){const n=document.createElement(tag);if(value!==undefined)n.textContent=String(value);return n}function input(type,label,value){const l=el("label",label),i=el("input");i.type=type;i.value=value;l.append(i);root.append(l);return i}if(d.kind==="table"){const filter=input("search","Filter rows","");const table=el("table"),head=el("thead"),body=el("tbody");table.append(head,body);root.append(table);let rows=d.rows.slice(),direction=1;const tr=el("tr");d.columns.forEach((name,index)=>{const th=el("th"),b=el("button",name);b.type="button";b.addEventListener("click",()=>{rows.sort((a,c)=>String(a[index]).localeCompare(String(c[index]),undefined,{numeric:true})*direction);direction*=-1;draw()});th.append(b);tr.append(th)});head.append(tr);const count=el("output");root.append(count);function draw(){body.replaceChildren();const found=rows.filter(row=>row.some(cell=>String(cell).toLocaleLowerCase().includes(filter.value.toLocaleLowerCase())));found.forEach(row=>{const tr=el("tr");row.forEach(cell=>tr.append(el("td",cell)));body.append(tr)});count.textContent=found.length+" / "+rows.length+" rows"}filter.addEventListener("input",draw);draw()}else if(d.kind==="chart"){const max=Math.max(1,...d.series.map(s=>s.value));d.series.forEach(s=>{const row=el("label",s.label),bar=el("progress"),value=el("span",s.value);bar.max=max;bar.value=s.value;bar.setAttribute("aria-label",s.label);row.append(bar,value);root.append(row)})}else{const budget=input("number","Budget ("+d.currency+")",d.budget);budget.min="0";budget.step="0.01";const amounts=d.items.map(item=>{const i=input("number",item.label,item.amount);i.min="0";i.step="0.01";return i});const total=el("output"),remaining=el("output"),progress=el("progress");total.className="metric";remaining.className="metric";progress.setAttribute("aria-label","Budget used");root.append(total,remaining,progress);function draw(){const cap=Number(budget.value),values=amounts.map(i=>Number(i.value));if(!Number.isFinite(cap)||cap<0||values.some(v=>!Number.isFinite(v)||v<0)){remaining.textContent="Enter valid non-negative amounts";return}const spent=values.reduce((a,b)=>a+b,0);total.textContent="Total: "+spent.toFixed(2)+" "+d.currency;remaining.textContent="Remaining: "+(cap-spent).toFixed(2)+" "+d.currency;progress.max=Math.max(1,cap);progress.value=Math.min(spent,progress.max)}[budget,...amounts].forEach(i=>i.addEventListener("input",draw));draw()}'''

SITE_STYLE='''
:root{color-scheme:dark;--ink:#f7f3e8;--muted:#b8b0a2;--night:#10100f;--panel:#1a1917;--line:#39352e;--acid:#e9ff62;--orange:#ff6948;--blue:#7b8cff}
*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--night);color:var(--ink);font-family:Arial,Helvetica,sans-serif;line-height:1.5;overflow-x:hidden}a{color:inherit}
.skip{position:absolute;left:1rem;top:-5rem;background:var(--acid);color:#111;padding:.75rem 1rem;z-index:20}.skip:focus{top:1rem}.shell{width:min(1180px,calc(100% - 40px));margin:auto}
.nav{display:flex;align-items:center;justify-content:space-between;padding:24px 0;border-bottom:1px solid var(--line)}.mark{display:flex;align-items:center;gap:12px;font-weight:900;letter-spacing:-.03em}.mark::before{content:'';width:22px;height:22px;background:var(--acid);border-radius:50% 50% 8px 50%;transform:rotate(-12deg)}.nav a:last-child{color:var(--muted);font-size:.88rem;text-decoration:none}
.hero{position:relative;min-height:680px;display:grid;grid-template-columns:minmax(0,1.2fr) minmax(260px,.8fr);gap:64px;align-items:center;padding:96px 0 72px}.hero::before{content:'';position:absolute;width:480px;height:480px;right:-190px;top:32px;background:var(--orange);border-radius:50%;opacity:.92;z-index:-1}
.hero>div,.section-head>*{min-width:0}.eyebrow{display:inline-flex;align-items:center;gap:10px;color:var(--acid);font-size:.78rem;font-weight:900;letter-spacing:.16em;text-transform:uppercase}.eyebrow::before{content:'';width:36px;height:2px;background:currentColor}h1{max-width:840px;margin:24px 0;font-size:clamp(3.5rem,9vw,7.6rem);line-height:.84;letter-spacing:-.075em;text-wrap:balance;overflow-wrap:anywhere}.lead{max-width:680px;color:var(--muted);font-size:clamp(1.05rem,2vw,1.35rem);overflow-wrap:anywhere}
.actions{display:flex;align-items:center;gap:18px;flex-wrap:wrap;margin-top:36px}.button{display:inline-flex;align-items:center;gap:14px;background:var(--acid);color:#111;padding:16px 22px;border:2px solid var(--acid);border-radius:999px;text-decoration:none;font-weight:900;box-shadow:7px 7px 0 var(--orange);transition:transform .15s,box-shadow .15s}.button::after{content:'↗';font-size:1.1em}.button:hover{transform:translate(3px,3px);box-shadow:4px 4px 0 var(--orange)}.proof{color:var(--muted);font-size:.82rem}
.totem{min-height:420px;position:relative}.tile{position:absolute;border:1px solid var(--line);background:var(--panel);box-shadow:0 24px 60px #0008}.tile.one{inset:42px 12px 72px 18px;transform:rotate(6deg);background:var(--blue)}.tile.two{inset:78px 46px 26px 0;transform:rotate(-5deg);background:var(--acid)}.tile.three{inset:110px 0 0 70px;display:grid;place-items:center;background:var(--panel);transform:rotate(2deg)}.face{width:148px;height:148px;border:18px solid var(--orange);border-radius:50% 50% 46% 54%;position:relative}.face::before,.face::after{content:'';position:absolute;top:43px;width:16px;height:22px;background:var(--ink);border-radius:50%}.face::before{left:29px}.face::after{right:29px}
.features{padding:70px 0 110px}.section-head{display:grid;grid-template-columns:1fr 1fr;gap:28px;align-items:end;margin-bottom:42px}.kicker{margin:0;color:var(--orange);font-size:.8rem;font-weight:900;letter-spacing:.15em;text-transform:uppercase}.section-head h2{margin:10px 0 0;font-size:clamp(2.3rem,5vw,4.7rem);line-height:.95;letter-spacing:-.055em}.section-head p{margin:0;color:var(--muted);overflow-wrap:anywhere}
.grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));border-top:1px solid var(--line);border-left:1px solid var(--line)}.feature{min-height:220px;padding:34px;border-right:1px solid var(--line);border-bottom:1px solid var(--line);background:linear-gradient(145deg,#1a1917,#11110f)}.number{color:var(--acid);font-size:.76rem;font-weight:900;letter-spacing:.12em}.feature h3{margin:56px 0 12px;font-size:1.55rem;letter-spacing:-.035em}.feature p{margin:0;color:var(--muted);max-width:48ch}
.footer{display:flex;justify-content:space-between;gap:20px;padding:28px 0 42px;border-top:1px solid var(--line);color:var(--muted);font-size:.8rem}a:focus-visible{outline:3px solid var(--acid);outline-offset:5px}
@media(max-width:780px){.shell{width:min(calc(100% - 28px),680px)}.nav{padding:18px 0}.nav a:last-child{display:none}.hero{min-height:auto;grid-template-columns:1fr;gap:48px;padding:70px 0 58px}.hero::before{width:300px;height:300px;right:-230px}.totem{min-height:330px}.section-head{grid-template-columns:1fr}.grid{grid-template-columns:1fr}.feature{min-height:190px}.feature h3{margin-top:34px}.footer{flex-direction:column}h1{font-size:clamp(3rem,16vw,4.7rem);letter-spacing:-.06em}.lead{font-size:1rem}}
@media(prefers-reduced-motion:reduce){html{scroll-behavior:auto}.button{transition:none}}
'''


def static_site(args):
    exact(args,('language','title','eyebrow','headline','subhead','features','cta'))
    language=text(args['language'],2);require(language in ('en','tr'))
    def required(value,limit):
        value=text(value,limit).strip();require(value);return value
    title=required(args['title'],80);eyebrow=required(args['eyebrow'],80)
    headline=required(args['headline'],160);subhead=required(args['subhead'],400)
    features=args['features'];require(isinstance(features,list) and 2<=len(features)<=6)
    checked=[]
    for feature in features:
        exact(feature,('title','body'))
        checked.append({'title':required(feature['title'],80),'body':required(feature['body'],240)})
    cta=args['cta'];exact(cta,('label','href'))
    label=required(cta['label'],60);href=text(cta['href'],2048).strip();require(href)
    if href not in ('#top','#features'):
        require(not any(character.isspace() for character in href))
        parsed=urllib.parse.urlsplit(href)
        try: port=parsed.port
        except ValueError: raise ValueError('PRODUCTION_INVALID_INPUT')
        require(parsed.scheme=='https' and parsed.hostname is not None and parsed.username is None
                and parsed.password is None and '\\' not in href and (port is None or 1<=port<=65535))
    data={'language':language,'title':title,'eyebrow':eyebrow,'headline':headline,
          'subhead':subhead,'features':checked,'cta':{'label':label,'href':href}}
    esc=lambda value: html.escape(value,quote=True)
    cards=''.join('<article class="feature"><span class="number">%02d</span><h3>%s</h3><p>%s</p></article>'%
                  (index,esc(feature['title']),esc(feature['body']))
                  for index,feature in enumerate(checked,1))
    feature_heading='What makes it different' if language=='en' else 'Neyi farklı yapıyor'
    feature_intro=('A clear value story in one fast, accessible page.' if language=='en'
                   else 'Tek, hızlı ve erişilebilir sayfada berrak bir değer hikâyesi.')
    local_note=('Editable local output · no trackers or remote assets' if language=='en'
                else 'Düzenlenebilir yerel çıktı · takipçi veya uzak varlık yok')
    digest=base64.b64encode(hashlib.sha256(SITE_STYLE.encode()).digest()).decode()
    csp="default-src 'none'; style-src 'sha256-"+digest+"'; img-src 'none'; media-src 'none'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
    document='''<!doctype html><html lang="%s"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="color-scheme" content="dark"><meta http-equiv="Content-Security-Policy" content="%s"><title>%s</title><style>%s</style></head><body id="top"><a class="skip" href="#features">%s</a><header class="shell nav"><div class="mark">%s</div><a href="#features">%s</a></header><main><section class="shell hero" aria-labelledby="site-title"><div><div class="eyebrow">%s</div><h1 id="site-title">%s</h1><p class="lead">%s</p><div class="actions"><a class="button" href="%s">%s</a><span class="proof">%s</span></div></div><div class="totem" aria-hidden="true"><div class="tile one"></div><div class="tile two"></div><div class="tile three"><div class="face"></div></div></div></section><section class="shell features" id="features" aria-labelledby="feature-title"><div class="section-head"><div><p class="kicker">%s</p><h2 id="feature-title">%s</h2></div><p>%s</p></div><div class="grid">%s</div></section></main><footer class="shell footer"><span>%s</span><span>%s</span></footer></body></html>'''%(
        language,esc(csp),esc(title),SITE_STYLE,
        'Skip to content' if language=='en' else 'İçeriğe geç',esc(title),esc(feature_heading),
        esc(eyebrow),esc(headline),esc(subhead),esc(href),esc(label),esc(local_note),
        'Why this works' if language=='en' else 'Neden işe yarıyor',esc(feature_heading),
        esc(feature_intro),cards,esc(title),'Built with Moxcrew' if language=='en' else 'Moxcrew ile üretildi')
    require('<script' not in document.lower() and 'javascript:' not in document.lower())
    content=json.dumps(data,ensure_ascii=False,sort_keys=True,separators=(',',':'))+'\n'
    summary=('# Website delivery\n\n## Result\n\nA standalone static landing page was generated.\n\n'
             '## Files\n\n- `index.html` — editable local preview\n- `content.json` — canonical owner content\n- `site-manifest.json` — hashes and checks\n\n'
             '## Checks performed\n\n- Owner content escaped\n- Script and remote-asset absence checked\n- Semantic landmarks, keyboard focus and responsive CSS present\n- Generated files reopened byte-for-byte\n\n'
             '## Checks not performed\n\n- No deployment, publication, analytics, conversion or Windows check\n\n'
             '## Revision notes\n\nFirst local version. Revise the exact content fields and regenerate to make a new version.\n')
    files={'index.html':document.encode(),'content.json':content.encode(),'summary.md':summary.encode()}
    for name,value in files.items():
        (OUT/name).write_bytes(value);require((OUT/name).read_bytes()==value,'PRODUCTION_OUTPUT_INVALID')
    manifest={'kind':'static_site','version':1,'offline':True,'published':False,'language':language,
              'featureCount':len(checked),'ctaKind':'local' if href.startswith('#') else 'https',
              'checks':{'contentEscaped':True,'scriptFree':True,'remoteAssetsAbsent':True,
                        'semanticLandmarks':True,'keyboardFocus':True,'responsiveLayout':True,
                        'reopenedExact':True},
              'files':[{'path':name,'bytes':len(value),'sha256':hashlib.sha256(value).hexdigest()}
                       for name,value in files.items()]}
    manifest_bytes=(json.dumps(manifest,ensure_ascii=False,sort_keys=True,indent=2)+'\n').encode()
    (OUT/'site-manifest.json').write_bytes(manifest_bytes)
    require((OUT/'site-manifest.json').read_bytes()==manifest_bytes,'PRODUCTION_OUTPUT_INVALID')
    return {'kind':'static_site','file':'index.html','manifest':'site-manifest.json','offline':True,
            'published':False,'featureCount':len(checked)}


def interactive(args):
    exact(args,('kind','title'),('columns','rows','series','currency','budget','items'))
    kind=args['kind'];require(kind in ('table','chart','budget'));title=text(args['title'],160)
    data={'kind':kind}
    if kind=='table':
        require(set(args)=={'kind','title','columns','rows'})
        columns=args['columns'];rows=args['rows'];require(isinstance(columns,list) and 1<=len(columns)<=30 and isinstance(rows,list) and len(rows)<=2000)
        for name in columns: text(name,160)
        require(len(set(columns))==len(columns) and len(rows)*len(columns)<=20000)
        for row in rows:
            require(isinstance(row,list) and len(row)==len(columns))
            for cell in row:
                if isinstance(cell,str): text(cell,2000)
                else: number(cell,-1e12,1e12)
        data.update(columns=columns,rows=rows)
    elif kind=='chart':
        require(set(args)=={'kind','title','series'});series=args['series'];require(isinstance(series,list) and 1<=len(series)<=100)
        for item in series: exact(item,('label','value'));text(item['label'],160);number(item['value'],0,1e12)
        data['series']=series
    else:
        require(set(args)=={'kind','title','currency','budget','items'});currency=text(args['currency'],3);require(re.fullmatch('[A-Z]{3}',currency) is not None)
        number(args['budget'],0,1e12);items=args['items'];require(isinstance(items,list) and 1<=len(items)<=100)
        for item in items: exact(item,('label','amount'));text(item['label'],160);number(item['amount'],0,1e12)
        data.update(currency=currency,budget=args['budget'],items=items)
    def digest(value): return base64.b64encode(hashlib.sha256(value.encode()).digest()).decode()
    csp="default-src 'none'; script-src 'sha256-"+digest(JS)+"'; style-src 'sha256-"+digest(STYLE)+"'; connect-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'"
    payload=json.dumps(data,ensure_ascii=True,allow_nan=False).replace('<','\\u003c').replace('>','\\u003e').replace('&','\\u0026')
    document='<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="'+html.escape(csp,quote=True)+'"><title>'+html.escape(title)+'</title><style>'+STYLE+'</style></head><body><main><h1>'+html.escape(title)+'</h1><p>Interactive local '+kind+' · changes stay in this browser tab</p><section id="tool" aria-label="Interactive '+kind+'"></section><footer>Standalone Moxcrew output. No network requests or external libraries.</footer></main><script id="data" type="application/json">'+payload+'</script><script>'+JS+'</script></body></html>'
    (OUT/'interactive.html').write_text(document,encoding='utf-8')
    return {'kind':'interactive','type':kind,'file':'interactive.html','offline':True}


def execute(operation,args,tools):
    if operation=='images.verify':
        exact(args,('source','width','height'));w=int(number(args['width'],1,4096));h=int(number(args['height'],1,4096))
        checked,w,h=png(source(args['source']).read_bytes(),(w,h));(OUT/'image.png').write_bytes(checked)
        return {'kind':'image','width':w,'height':h}
    if operation=='images.batch': return batch_images(args,tools)
    if operation=='images.mask': return make_mask(args,tools)
    if operation=='pdf.extract': return pdf_extract(args,tools)
    if operation=='pdf.merge': return pdf_merge(args,tools)
    if operation=='resources.ocr': return ocr(args,tools)
    if operation=='audio.transcribe': return transcribe(args,tools)
    if operation=='audio.speech': return speech(args,tools)
    if operation=='audio.podcast': return podcast(args,tools)
    if operation=='data.chart': return data_chart(args,tools)
    if operation=='data.scenario': return data_scenario(args,tools)
    if operation=='data.extract': return data_extract(args,tools)
    if operation=='documents.brief': return documents_brief(args,tools)
    if operation=='documents.rewrite': return documents_rewrite(args,tools)
    if operation=='documents.template': return documents_template(args,tools)
    if operation=='research.claims': return claim_check(args,tools)
    if operation=='research.compare': return research_compare(args,tools)
    if operation=='research.synthesis': return research_synthesis(args,tools)
    if operation=='research.changes': return source_changes(args,tools)
    if operation=='documents.render': return render(args,tools)
    if operation=='video.frame': return frame(args,tools)
    if operation=='video.package': return package(args,tools)
    if operation=='video.highlights': return video_highlights(args,tools)
    if operation=='video.compose': return compose_video(args,tools)
    if operation=='interactive.create': return interactive(args)
    if operation=='web.site': return static_site(args)
    raise ValueError('PRODUCTION_OPERATION_UNSUPPORTED')
