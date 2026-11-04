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
import zlib


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


def captions_file(captions,duration,path):
    require(isinstance(captions,list) and len(captions)<=1000)
    last=0;lines=[]
    for i,cap in enumerate(captions,1):
        exact(cap,('start','end','text'));start=number(cap['start'],last,duration);end=number(cap['end'],start+0.001,duration);last=end
        caption=text(cap['text'],500)
        # Plain SRT text only: do not allow ASS override tags or SRT/HTML formatting.
        caption=caption.replace('{','(').replace('}',')').replace('<','‹').replace('>','›').replace('\\','／')
        caption='\n'.join(line for line in caption.replace('\r','').split('\n') if line.strip())
        lines.append(f'{i}\n{stamp(start)} --> {stamp(end)}\n{caption}\n')
    path.write_text('\n'.join(lines),encoding='utf-8')


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
    path=source(args['source']);info=media_info(tools,path)
    duration=number(float(info.get('format',{}).get('duration',0)),0.02,1200)
    require(any(s.get('codec_type')=='audio' for s in info.get('streams',[])),'PRODUCTION_AUDIO_REQUIRED')
    wav=ROOT/'speech.wav'
    run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error','-n',*input_format(path),'-i',str(path),'-vn','-ar','16000','-ac','1','-c:a','pcm_s16le',str(wav)])
    target=ROOT/'speech-result'
    run_tool(tools,'whisper',['-m',model,'-f',str(wav),'-l',language,'-oj','-of',str(target),'-np'])
    result=target.with_suffix('.json');require(result.is_file() and result.stat().st_size<=1024*1024,'PRODUCTION_TRANSCRIPTION_INVALID')
    native=json.loads(result.read_text(encoding='utf-8'));items=native.get('transcription')
    require(isinstance(items,list) and len(items)<=1000,'PRODUCTION_TRANSCRIPTION_INVALID')
    segments=[];last=0
    for item in items:
        offsets=item.get('offsets',{});start=number(offsets.get('from'),0,duration*1000+50)/1000;end=number(offsets.get('to'),0,duration*1000+50)/1000
        require(start>=last and end>start,'PRODUCTION_TRANSCRIPTION_INVALID');last=start
        segments.append({'start':start,'end':min(end,duration),'text':text(item.get('text'),500).strip()})
    result_language=text(native.get('result',{}).get('language',language),12)
    require(re.fullmatch(r'[a-z]{2,3}|auto',result_language) is not None,'PRODUCTION_TRANSCRIPTION_INVALID')
    value={'kind':'transcript','text':'\n'.join(s['text'] for s in segments),'segments':segments,'durationSeconds':duration,'language':result_language,'engine':'whisper.cpp'}
    text(value['text'],150000)
    (OUT/'transcript.json').write_text(json.dumps(value,ensure_ascii=False),encoding='utf-8');(OUT/'transcript.txt').write_text(value['text'],encoding='utf-8')
    captions_file(segments,duration,OUT/'captions.srt')
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
    if operation=='resources.ocr': return ocr(args,tools)
    if operation=='audio.transcribe': return transcribe(args,tools)
    if operation=='audio.podcast': return podcast(args,tools)
    if operation=='data.chart': return data_chart(args,tools)
    if operation=='data.scenario': return data_scenario(args,tools)
    if operation=='data.extract': return data_extract(args,tools)
    if operation=='documents.brief': return documents_brief(args,tools)
    if operation=='documents.rewrite': return documents_rewrite(args,tools)
    if operation=='documents.template': return documents_template(args,tools)
    if operation=='research.claims': return claim_check(args,tools)
    if operation=='research.changes': return source_changes(args,tools)
    if operation=='documents.render': return render(args,tools)
    if operation=='video.frame': return frame(args,tools)
    if operation=='video.package': return package(args,tools)
    if operation=='interactive.create': return interactive(args)
    raise ValueError('PRODUCTION_OPERATION_UNSUPPORTED')
