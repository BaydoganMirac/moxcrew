"""Moxcrew's bounded, typed local production worker. Python standard library only."""
import csv
import hashlib
import io
import itertools
import json
import math
from pathlib import Path
import re
import runpy
import statistics
import struct
import subprocess
import tempfile
import textwrap
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


SLIDE_W, SLIDE_H = 12192000, 6858000
PRESENTATION_LAYOUTS = ("cover", "statement", "content", "split", "bar-chart")
DEFAULT_PRESENTATION_BRAND = {
    "name": "Moxcrew",
    "primary": "2854F4",
    "accent": "FF7A2F",
    "background": "F8F7F3",
    "text": "15223D",
    "muted": "56627A",
    "font": "Arial",
}


def presentation_color(value):
    value = text(value, 7).removeprefix("#").upper()
    require(re.fullmatch(r"[0-9A-F]{6}", value) is not None)
    return value


def presentation_contrast(first, second):
    def luminance(value):
        channels = [int(value[i:i + 2], 16) / 255 for i in (0, 2, 4)]
        channels = [v / 12.92 if v <= .04045 else ((v + .055) / 1.055) ** 2.4 for v in channels]
        return .2126 * channels[0] + .7152 * channels[1] + .0722 * channels[2]
    light, dark = sorted((luminance(first), luminance(second)), reverse=True)
    return (light + .05) / (dark + .05)


def presentation_brand(value=None):
    value = value or {}
    exact(value, (), tuple(DEFAULT_PRESENTATION_BRAND) + ("logo",))
    result = dict(DEFAULT_PRESENTATION_BRAND)
    for key in ("primary", "accent", "background", "text", "muted"):
        if key in value:
            result[key] = presentation_color(value[key])
    for key, limit in (("name", 80), ("font", 80)):
        if key in value:
            result[key] = text(value[key], limit).strip()
            require(result[key])
    require(presentation_contrast(result["text"], result["background"]) >= 4.5,
            "PRODUCTION_PRESENTATION_CONTRAST")
    require(presentation_contrast(result["muted"], result["background"]) >= 4.5,
            "PRODUCTION_PRESENTATION_CONTRAST")
    require(presentation_contrast(result["primary"], result["background"]) >= 3,
            "PRODUCTION_PRESENTATION_CONTRAST")
    if "logo" in value:
        exact(value["logo"], ("source",), ("alt",))
        result["logo"] = {
            "source": text(value["logo"]["source"], 1024),
            "alt": text(value["logo"].get("alt", result["name"]), 160),
        }
    return result


def presentation_theme(brand):
    return f'''<a:theme xmlns:a="{NS["a"]}" name="{escape(brand["name"])}"><a:themeElements><a:clrScheme name="{escape(brand["name"])}"><a:dk1><a:srgbClr val="{brand["text"]}"/></a:dk1><a:lt1><a:srgbClr val="{brand["background"]}"/></a:lt1><a:dk2><a:srgbClr val="{brand["muted"]}"/></a:dk2><a:lt2><a:srgbClr val="FFFFFF"/></a:lt2><a:accent1><a:srgbClr val="{brand["primary"]}"/></a:accent1><a:accent2><a:srgbClr val="{brand["accent"]}"/></a:accent2><a:accent3><a:srgbClr val="{brand["muted"]}"/></a:accent3><a:accent4><a:srgbClr val="7D8CB5"/></a:accent4><a:accent5><a:srgbClr val="42A889"/></a:accent5><a:accent6><a:srgbClr val="D9577B"/></a:accent6><a:hlink><a:srgbClr val="{brand["primary"]}"/></a:hlink><a:folHlink><a:srgbClr val="{brand["accent"]}"/></a:folHlink></a:clrScheme><a:fontScheme name="{escape(brand["name"])}"><a:majorFont><a:latin typeface="{escape(brand["font"])}"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="{escape(brand["font"])}"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="{escape(brand["name"])}"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="accent1"/></a:solidFill><a:solidFill><a:schemeClr val="accent2"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="12700"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="25400"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln><a:ln w="38100"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill><a:solidFill><a:schemeClr val="lt1"/></a:solidFill><a:solidFill><a:schemeClr val="dk1"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>'''


def scheme_fill(name, alpha=None):
    transparency = f'<a:alpha val="{alpha}"/>' if alpha is not None else ''
    return f'<a:solidFill><a:schemeClr val="{name}">{transparency}</a:schemeClr></a:solidFill>'


def presentation_paragraph(value, size, color="tx1", bold=False, align="l", bullet=False):
    properties = f' algn="{align}"' + (' marL="320000" indent="-220000"' if bullet else '')
    marker = '<a:buChar char="•"/>' if bullet else '<a:buNone/>'
    font = "+mj-lt" if bold or size >= 3000 else "+mn-lt"
    return f'<a:p><a:pPr{properties}>{marker}</a:pPr><a:r><a:rPr lang="en-US" sz="{size}" b="{1 if bold else 0}" dirty="0">{scheme_fill(color)}<a:latin typeface="{font}"/></a:rPr><a:t>{escape(value)}</a:t></a:r><a:endParaRPr lang="en-US" sz="{size}"/></a:p>'


def presentation_shape(ident, name, x, y, width, height, paragraphs=(), fill=None,
                       line=None, geometry="rect", anchor="t", margins=(0, 0, 0, 0)):
    l, top, r, bottom = margins
    text_box = ' txBox="1"' if paragraphs else ''
    non_visual = f'<p:nvSpPr><p:cNvPr id="{ident}" name="{escape(name)}"/><p:cNvSpPr{text_box}/><p:nvPr/></p:nvSpPr>'
    body = '' if not paragraphs else f'<p:txBody><a:bodyPr wrap="square" anchor="{anchor}" lIns="{l}" tIns="{top}" rIns="{r}" bIns="{bottom}"><a:normAutofit fontScale="90000" lnSpcReduction="10000"/></a:bodyPr><a:lstStyle/>{"".join(paragraphs)}</p:txBody>'
    fill_xml = '<a:noFill/>' if fill is None else scheme_fill(fill[0], fill[1] if len(fill) > 1 else None)
    line_xml = '<a:ln><a:noFill/></a:ln>' if line is None else f'<a:ln w="{line[1]}">{scheme_fill(line[0])}</a:ln>'
    return f'<p:sp>{non_visual}<p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{width}" cy="{height}"/></a:xfrm><a:prstGeom prst="{geometry}"><a:avLst/></a:prstGeom>{fill_xml}{line_xml}</p:spPr>{body}</p:sp>'


def raster_dimensions(data, extension):
    if extension == "png":
        require(data.startswith(b"\x89PNG\r\n\x1a\n") and len(data) >= 24,
                "PRODUCTION_IMAGE_FORMAT_UNSUPPORTED")
        width, height = struct.unpack(">II", data[16:24])
    else:
        require(data.startswith(b"\xff\xd8"), "PRODUCTION_IMAGE_FORMAT_UNSUPPORTED")
        index = 2
        width = height = 0
        while index + 9 < len(data):
            if data[index] != 0xff:
                index += 1
                continue
            marker = data[index + 1]
            if marker in (0xd8, 0xd9):
                index += 2
                continue
            length = int.from_bytes(data[index + 2:index + 4], "big")
            require(length >= 2 and index + 2 + length <= len(data),
                    "PRODUCTION_IMAGE_FORMAT_UNSUPPORTED")
            if marker in range(0xc0, 0xc4):
                height = int.from_bytes(data[index + 5:index + 7], "big")
                width = int.from_bytes(data[index + 7:index + 9], "big")
                break
            index += 2 + length
    require(16 <= width <= 8192 and 16 <= height <= 8192,
            "PRODUCTION_IMAGE_DIMENSIONS_INVALID")
    return width, height


def register_raster(value, media, existing=()):
    exact(value, ("source",), ("alt", "fit"))
    path = source(value["source"])
    extension = path.suffix.lower().removeprefix(".")
    extension = "jpeg" if extension in ("jpg", "jpeg") else extension
    require(extension in ("png", "jpeg"), "PRODUCTION_IMAGE_FORMAT_UNSUPPORTED")
    data = path.read_bytes()
    width, height = raster_dimensions(data, extension)
    digest = hashlib.sha256(data).hexdigest()
    if digest not in media:
        used = {int(match[1]) for name in existing
                if (match := re.fullmatch(r"ppt/media/image([0-9]+)\.(png|jpe?g)", name))}
        used.update(entry["index"] for entry in media.values())
        index = next(i for i in range(1, 1000) if i not in used)
        media[digest] = {"index": index, "path": f"ppt/media/image{index}.{extension}",
                         "bytes": data, "width": width, "height": height,
                         "extension": extension}
    result = dict(media[digest])
    result.update(alt=text(value.get("alt", path.stem), 160), fit=value.get("fit", "cover"),
                  sha256=digest)
    require(result["fit"] in ("cover", "contain"))
    return result


def presentation_picture(ident, name, relationship, image, x, y, width, height):
    source_ratio = image["width"] / image["height"]
    box_ratio = width / height
    crop = ""
    if image["fit"] == "contain":
        if source_ratio > box_ratio:
            actual_height = int(width / source_ratio)
            y += (height - actual_height) // 2
            height = actual_height
        else:
            actual_width = int(height * source_ratio)
            x += (width - actual_width) // 2
            width = actual_width
    elif source_ratio > box_ratio:
        cut = int((1 - box_ratio / source_ratio) * 50000)
        crop = f'<a:srcRect l="{cut}" r="{cut}"/>'
    else:
        cut = int((1 - source_ratio / box_ratio) * 50000)
        crop = f'<a:srcRect t="{cut}" b="{cut}"/>'
    return f'<p:pic><p:nvPicPr><p:cNvPr id="{ident}" name="{escape(name)}" descr="{escape(image["alt"])}"/><p:cNvPicPr><a:picLocks noChangeAspect="1"/></p:cNvPicPr><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="{relationship}"/>{crop}<a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{width}" cy="{height}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:ln><a:noFill/></a:ln></p:spPr></p:pic>'


def presentation_slide(value):
    optional = ("layout", "eyebrow", "subtitle", "body", "bullets", "notes",
                "footer", "image", "chart")
    exact(value, ("title",), optional)
    slide = {"title": text(value["title"], 160).strip(),
             "layout": value.get("layout", "content")}
    require(slide["title"] and slide["layout"] in PRESENTATION_LAYOUTS)
    for key, limit in (("eyebrow", 80), ("subtitle", 240), ("body", 1600),
                       ("notes", 6000), ("footer", 240)):
        slide[key] = text(value.get(key, ""), limit).strip()
    bullets = value.get("bullets", [])
    require(isinstance(bullets, list) and len(bullets) <= 6)
    slide["bullets"] = [text(item, 220).strip() for item in bullets]
    require(all(slide["bullets"]))
    if "image" in value:
        exact(value["image"], ("source",), ("alt", "fit"))
        slide["image"] = value["image"]
    if "chart" in value:
        chart = value["chart"]
        exact(chart, ("series",), ("unit", "sourceLabel"))
        series = chart["series"]
        require(isinstance(series, list) and 1 <= len(series) <= 6)
        checked = []
        for item in series:
            exact(item, ("label", "value"))
            checked.append({"label": text(item["label"], 60).strip(),
                            "value": number(item["value"], 0, 1e12)})
        require(all(item["label"] for item in checked))
        slide["chart"] = {"series": checked, "unit": text(chart.get("unit", ""), 16),
                          "sourceLabel": text(chart.get("sourceLabel", ""), 240)}
    has_text = bool(slide["body"] or slide["bullets"])
    if slide["layout"] == "cover":
        require(not has_text and "chart" not in slide)
    elif slide["layout"] == "statement":
        require(not slide["bullets"] and "chart" not in slide and "image" not in slide)
    elif slide["layout"] == "content":
        require(has_text and "chart" not in slide and "image" not in slide)
    elif slide["layout"] == "split":
        require(has_text and (("chart" in slide) != ("image" in slide)))
    else:
        require("chart" in slide and not has_text and "image" not in slide)
    width = 36 if slide["layout"] == "split" else 68
    estimated = sum(max(1, len(textwrap.wrap(item, width=width, break_long_words=False)))
                    for item in ([slide["body"]] if slide["body"] else []) + slide["bullets"])
    require(estimated <= (10 if slide["layout"] == "split" else 13),
            "PRODUCTION_PRESENTATION_TEXT_OVERFLOW")
    require(len(textwrap.wrap(slide["title"], width=34, break_long_words=False)) <= 3,
            "PRODUCTION_PRESENTATION_TEXT_OVERFLOW")
    return slide


def number_label(value):
    return f"{value:,.2f}".rstrip("0").rstrip(".")


def presentation_slide_xml(slide, index, count, brand, media, existing_media=(),
                           layout_target="../slideLayouts/slideLayout1.xml", notes_target=None):
    shapes = []
    relationships = [(1, "slideLayout", layout_target)]
    next_id = 2
    next_relationship = 2
    group = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
    shapes.append(presentation_shape(next_id, "Background", 0, 0, SLIDE_W, SLIDE_H,
                                     fill=("bg1",), line=None))
    next_id += 1
    image = register_raster(slide["image"], media, existing_media) if "image" in slide else None
    logo = register_raster(brand["logo"], media, existing_media) if "logo" in brand else None

    def add_text(name, x, y, width, height, paragraphs, **kwargs):
        nonlocal next_id
        shapes.append(presentation_shape(next_id, name, x, y, width, height, paragraphs,
                                         **kwargs))
        next_id += 1

    def add_picture(name, image_value, x, y, width, height):
        nonlocal next_id, next_relationship
        relationship = f"rId{next_relationship}"
        relationships.append((next_relationship, "image", "../media/" + Path(image_value["path"]).name))
        shapes.append(presentation_picture(next_id, name, relationship, image_value,
                                           x, y, width, height))
        next_id += 1
        next_relationship += 1

    if slide["layout"] == "cover":
        shapes.append(presentation_shape(next_id, "Primary band", 0, 0, 360000, SLIDE_H,
                                         fill=("accent1",)))
        next_id += 1
        if image:
            add_picture("Cover image", image, 7900000, 0, SLIDE_W - 7900000, SLIDE_H)
            title_width = 6500000
        else:
            shapes.append(presentation_shape(next_id, "Accent field", 8750000, 0,
                                             SLIDE_W - 8750000, SLIDE_H, fill=("accent1", 12000)))
            next_id += 1
            title_width = 8000000
        if logo:
            logo["fit"] = "contain"
            add_picture("Brand logo", logo, 720000, 460000, 2200000, 520000)
        elif brand["name"]:
            add_text("Brand", 720000, 450000, 3000000, 420000,
                     [presentation_paragraph(brand["name"].upper(), 1300, "accent1", True)])
        if slide["eyebrow"]:
            add_text("Eyebrow", 720000, 1420000, title_width, 420000,
                     [presentation_paragraph(slide["eyebrow"].upper(), 1400, "accent2", True)])
        add_text("Title", 720000, 1980000, title_width, 2300000,
                 [presentation_paragraph(slide["title"], 4400, "tx1", True)], anchor="ctr")
        if slide["subtitle"]:
            add_text("Subtitle", 720000, 4450000, title_width, 1000000,
                     [presentation_paragraph(slide["subtitle"], 2100, "tx1")])
    elif slide["layout"] == "statement":
        shapes.append(presentation_shape(next_id, "Accent rule", 740000, 950000, 150000, 4900000,
                                         fill=("accent2",)))
        next_id += 1
        if slide["eyebrow"]:
            add_text("Eyebrow", 1120000, 900000, 9000000, 420000,
                     [presentation_paragraph(slide["eyebrow"].upper(), 1400, "accent1", True)])
        add_text("Statement", 1120000, 1500000, 9400000, 3000000,
                 [presentation_paragraph(slide["title"], 4200, "tx1", True)], anchor="ctr")
        supporting = slide["subtitle"] or slide["body"]
        if supporting:
            add_text("Support", 1120000, 4700000, 8500000, 850000,
                     [presentation_paragraph(supporting, 2000, "accent3")])
    else:
        if slide["eyebrow"]:
            add_text("Eyebrow", 720000, 360000, 9000000, 380000,
                     [presentation_paragraph(slide["eyebrow"].upper(), 1300, "accent2", True)])
        add_text("Title", 720000, 760000, 10400000, 900000,
                 [presentation_paragraph(slide["title"], 3200, "tx1", True)])
        shapes.append(presentation_shape(next_id, "Title rule", 720000, 1640000, 10400000, 26000,
                                         fill=("accent1",)))
        next_id += 1
        if slide["layout"] in ("content", "split"):
            paragraphs = []
            if slide["body"]:
                paragraphs.append(presentation_paragraph(slide["body"], 2100, "tx1"))
            paragraphs.extend(presentation_paragraph(item, 2000, "tx1", bullet=True)
                              for item in slide["bullets"])
            width = 5200000 if slide["layout"] == "split" else 10400000
            add_text("Body", 720000, 1950000, width, 3950000, paragraphs,
                     margins=(0, 80000, 160000, 0))
            if slide["layout"] == "split":
                if image:
                    add_picture("Slide image", image, 6300000, 1950000, 5100000, 3650000)
                else:
                    chart = slide["chart"]
                    maximum = max(item["value"] for item in chart["series"]) or 1
                    row_height = min(570000, 3300000 // len(chart["series"]))
                    for row, item in enumerate(chart["series"]):
                        y = 2050000 + row * row_height
                        add_text(f"Chart label {row + 1}", 6300000, y, 1600000, row_height - 50000,
                                 [presentation_paragraph(item["label"], 1400, "tx1")], anchor="ctr")
                        bar_width = max(20000, int(2600000 * item["value"] / maximum))
                        shapes.append(presentation_shape(next_id, f"Chart bar {row + 1}", 7950000, y + 90000,
                                                         bar_width, max(120000, row_height - 180000),
                                                         fill=(("accent2" if row == 0 else "accent1"),),
                                                         geometry="roundRect"))
                        next_id += 1
                        add_text(f"Chart value {row + 1}", 10600000, y, 650000, row_height - 50000,
                                 [presentation_paragraph(number_label(item["value"]) + chart["unit"],
                                                         1300, "accent3", True, "r")], anchor="ctr")
        else:
            chart = slide["chart"]
            maximum = max(item["value"] for item in chart["series"]) or 1
            row_height = min(650000, 3800000 // len(chart["series"]))
            for row, item in enumerate(chart["series"]):
                y = 2050000 + row * row_height
                add_text(f"Chart label {row + 1}", 720000, y, 2300000, row_height - 40000,
                         [presentation_paragraph(item["label"], 1600, "tx1")], anchor="ctr")
                shapes.append(presentation_shape(next_id, f"Track {row + 1}", 3200000, y + 110000,
                                                 6600000, max(120000, row_height - 220000),
                                                 fill=("accent3", 14000), geometry="roundRect"))
                next_id += 1
                bar_width = max(20000, int(6600000 * item["value"] / maximum))
                shapes.append(presentation_shape(next_id, f"Bar {row + 1}", 3200000, y + 110000,
                                                 bar_width, max(120000, row_height - 220000),
                                                 fill=(("accent2" if row == 0 else "accent1"),),
                                                 geometry="roundRect"))
                next_id += 1
                add_text(f"Chart value {row + 1}", 10000000, y, 1200000, row_height - 40000,
                         [presentation_paragraph(number_label(item["value"]) + chart["unit"],
                                                 1500, "accent3", True, "r")], anchor="ctr")
            if chart["sourceLabel"]:
                add_text("Chart source", 720000, 5850000, 10400000, 350000,
                         [presentation_paragraph(chart["sourceLabel"], 1100, "accent3")])
    footer = slide["footer"]
    if footer:
        add_text("Footer", 720000, 6320000, 9500000, 260000,
                 [presentation_paragraph(footer, 1000, "accent3")])
    add_text("Slide number", 10800000, 6320000, 600000, 260000,
             [presentation_paragraph(f"{index:02}", 1000, "accent3", True, "r")])
    if notes_target:
        relationships.append((next_relationship, "notesSlide", notes_target))
    document = f'<p:sld xmlns:a="{NS["a"]}" xmlns:p="{NS["p"]}" xmlns:r="{OFFICE}"><p:cSld><p:spTree>{group}{"".join(shapes)}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sld>'
    return document, rels(relationships), {
        "slide": index, "layout": slide["layout"], "title": slide["title"],
        "notes": bool(slide["notes"]),
        "imageSha256": image["sha256"] if image else None,
        "chart": slide.get("chart"),
    }


def notes_slide_xml(value):
    group = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
    body = presentation_shape(2, "Notes", 600000, 600000, 5600000, 7200000,
                              [presentation_paragraph(value, 1800, "tx1")],
                              margins=(100000, 100000, 100000, 100000))
    return f'<p:notes xmlns:a="{NS["a"]}" xmlns:p="{NS["p"]}" xmlns:r="{OFFICE}"><p:cSld><p:spTree>{group}{body}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>'


def presentation_content_types(parts):
    overrides = []
    mapping = [
        (r"ppt/presentation.xml", "presentationml.presentation.main"),
        (r"ppt/slides/slide[0-9]+.xml", "presentationml.slide"),
        (r"ppt/slideMasters/slideMaster[0-9]+.xml", "presentationml.slideMaster"),
        (r"ppt/slideLayouts/slideLayout[0-9]+.xml", "presentationml.slideLayout"),
        (r"ppt/theme/theme[0-9]+.xml", "theme+xml"),
        (r"ppt/notesMasters/notesMaster[0-9]+.xml", "presentationml.notesMaster"),
        (r"ppt/notesSlides/notesSlide[0-9]+.xml", "presentationml.notesSlide"),
    ]
    for name in parts:
        for pattern, kind in mapping:
            if re.fullmatch(pattern, name):
                mime = ("application/vnd.openxmlformats-officedocument." + kind + "+xml"
                        if kind != "theme+xml" else
                        "application/vnd.openxmlformats-officedocument.theme+xml")
                overrides.append(f'<Override PartName="/{name}" ContentType="{mime}"/>')
                break
    overrides.extend([
        '<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>',
        '<Override PartName="/docProps/app.xml" ContentType="application/vnd.openxmlformats-officedocument.extended-properties+xml"/>',
    ])
    return f'<Types xmlns="{CONTENT}"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpeg" ContentType="image/jpeg"/><Default Extension="jpg" ContentType="image/jpeg"/>{"".join(sorted(set(overrides)))}</Types>'


def update_presentation_content_types(parts):
    node = xml(parts["[Content_Types].xml"])
    defaults = {item.attrib.get("Extension", "").lower() for item in node
                if item.tag.rsplit("}", 1)[-1] == "Default"}
    for extension, mime in (("png", "image/png"), ("jpeg", "image/jpeg"),
                            ("jpg", "image/jpeg")):
        if extension not in defaults and any(name.lower().endswith("." + extension) for name in parts):
            ET.SubElement(node, f"{{{CONTENT}}}Default",
                          {"Extension": extension, "ContentType": mime})
    overrides = {item.attrib.get("PartName") for item in node
                 if item.tag.rsplit("}", 1)[-1] == "Override"}
    for name, mime in [
        *[(name, "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml")
          for name in parts if re.fullmatch(r"ppt/notesMasters/notesMaster[0-9]+\.xml", name)],
        *[(name, "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml")
          for name in parts if re.fullmatch(r"ppt/notesSlides/notesSlide[0-9]+\.xml", name)],
        *[(name, "application/vnd.openxmlformats-officedocument.theme+xml")
          for name in parts if re.fullmatch(r"ppt/theme/theme[0-9]+\.xml", name)],
    ]:
        if "/" + name not in overrides:
            ET.SubElement(node, f"{{{CONTENT}}}Override",
                          {"PartName": "/" + name, "ContentType": mime})
    parts["[Content_Types].xml"] = ET.tostring(node, encoding="utf-8", xml_declaration=True)


def presentation_skeleton(brand, slide_count, has_notes):
    group = '<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>'
    ids = ''.join(f'<p:sldId id="{255 + i}" r:id="rId{1 + i}"/>' for i in range(1, slide_count + 1))
    notes_id = f'<p:notesMasterIdLst><p:notesMasterId r:id="rId{slide_count + 2}"/></p:notesMasterIdLst>' if has_notes else ''
    presentation = f'<p:presentation xmlns:p="{NS["p"]}" xmlns:r="{OFFICE}"><p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rId1"/></p:sldMasterIdLst>{notes_id}<p:sldIdLst>{ids}</p:sldIdLst><p:sldSz cx="{SLIDE_W}" cy="{SLIDE_H}" type="screen16x9"/><p:notesSz cx="6858000" cy="9144000"/></p:presentation>'
    relationships = [(1, "slideMaster", "slideMasters/slideMaster1.xml")]
    relationships.extend((1 + i, "slide", f"slides/slide{i}.xml") for i in range(1, slide_count + 1))
    if has_notes:
        relationships.append((slide_count + 2, "notesMaster", "notesMasters/notesMaster1.xml"))
    master = f'<p:sldMaster xmlns:a="{NS["a"]}" xmlns:p="{NS["p"]}" xmlns:r="{OFFICE}"><p:cSld name="Moxcrew"><p:spTree>{group}</p:spTree></p:cSld><p:clrMap accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" bg1="lt1" bg2="lt2" tx1="dk1" tx2="dk2" hlink="hlink" folHlink="folHlink"/><p:sldLayoutIdLst><p:sldLayoutId id="2147483649" r:id="rId1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>'
    layout = f'<p:sldLayout xmlns:a="{NS["a"]}" xmlns:p="{NS["p"]}" xmlns:r="{OFFICE}" type="blank" preserve="1"><p:cSld name="Moxcrew blank"><p:spTree>{group}</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>'
    parts = {
        "_rels/.rels": f'<Relationships xmlns="{REL}"><Relationship Id="rId1" Type="{OFFICE}/officeDocument" Target="ppt/presentation.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/><Relationship Id="rId3" Type="{OFFICE}/extended-properties" Target="docProps/app.xml"/></Relationships>',
        "docProps/core.xml": f'<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{escape(brand["name"])}</dc:title><dc:creator>Moxcrew</dc:creator></cp:coreProperties>',
        "docProps/app.xml": f'<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Application>Moxcrew</Application><Slides>{slide_count}</Slides></Properties>',
        "ppt/presentation.xml": presentation,
        "ppt/_rels/presentation.xml.rels": rels(relationships),
        "ppt/slideMasters/slideMaster1.xml": master,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels": rels([(1, "slideLayout", "../slideLayouts/slideLayout1.xml"), (2, "theme", "../theme/theme1.xml")]),
        "ppt/slideLayouts/slideLayout1.xml": layout,
        "ppt/slideLayouts/_rels/slideLayout1.xml.rels": rels([(1, "slideMaster", "../slideMasters/slideMaster1.xml")]),
        "ppt/theme/theme1.xml": presentation_theme(brand),
    }
    if has_notes:
        parts["ppt/notesMasters/notesMaster1.xml"] = f'<p:notesMaster xmlns:a="{NS["a"]}" xmlns:p="{NS["p"]}"><p:cSld><p:spTree>{group}</p:spTree></p:cSld><p:clrMap accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" bg1="lt1" bg2="lt2" tx1="dk1" tx2="dk2" hlink="hlink" folHlink="folHlink"/><p:hf hdr="0" ftr="0" dt="0" sldNum="0"/><p:notesStyle/></p:notesMaster>'
        parts["ppt/notesMasters/_rels/notesMaster1.xml.rels"] = rels([(1, "theme", "../theme/theme1.xml")])
    return parts


def presentation_previews(path, slide_count, tools):
    require(1 <= slide_count <= 12, "PRODUCTION_PRESENTATION_RENDER_LIMIT")
    require(tools.get("libreoffice"), "PRODUCTION_OFFICE_RENDERER_REQUIRED")
    require(tools.get("pdftoppm"), "PRODUCTION_PAGE_RENDERER_REQUIRED")
    profile = ROOT / "presentation-profile"
    (profile / "user").mkdir(parents=True)
    (profile / "user" / "registrymodifications.xcu").write_text('<?xml version="1.0"?><oor:items xmlns:oor="http://openoffice.org/2001/registry"><item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="MacroSecurityLevel" oor:op="fuse"><value>3</value></prop></item><item oor:path="/org.openoffice.Office.Common/Load"><prop oor:name="UpdateMode" oor:op="fuse"><value>0</value></prop></item></oor:items>')
    rendered = ROOT / "presentation-rendered"
    rendered.mkdir()
    run_tool(tools, "libreoffice", ["-env:UserInstallation=" + profile.as_uri(), "--headless",
             "--nologo", "--nodefault", "--norestore", "--convert-to", "pdf",
             "--outdir", str(rendered), str(path)])
    pdf = rendered / (path.stem + ".pdf")
    require(pdf.is_file() and pdf.read_bytes().startswith(b"%PDF-"), "PRODUCTION_RENDER_FAILED")
    (OUT / "slides.pdf").write_bytes(pdf.read_bytes())
    previews = []
    for page in range(1, slide_count + 1):
        prefix = OUT / f"slide-{page:02}"
        run_tool(tools, "pdftoppm", ["-f", str(page), "-l", str(page), "-singlefile",
                 "-scale-to", "1280", "-png", str(pdf), str(prefix)])
        image = prefix.with_suffix(".png")
        require(image.is_file(), "PRODUCTION_PAGE_NOT_FOUND")
        width, height = raster_dimensions(image.read_bytes(), "png")
        previews.append({"slide": page, "file": image.name, "width": width, "height": height})
    return previews


def presentation_active_content(parts):
    bad_names = ("vbaproject", "activex/", "embeddings/")
    return any(any(bad in name.lower() for bad in bad_names) or
               name.lower().endswith((".bin", ".js", ".html", ".htm")) for name in parts)


def presentation_slide_names(parts):
    return sorted((name for name in parts if re.fullmatch(r"ppt/slides/slide[0-9]+\.xml", name)),
                  key=lambda name: int(re.search(r"slide([0-9]+)", name)[1]))


def presentation_notes(parts, slide_name):
    relationships = f'ppt/slides/_rels/{Path(slide_name).name}.rels'
    require(relationships in parts, "PRODUCTION_PRESENTATION_NOTES_REQUIRED")
    target = None
    for relationship in xml(parts[relationships]):
        if relationship.attrib.get("Type", "").endswith("/notesSlide"):
            target = relationship.attrib.get("Target")
            break
    require(target is not None, "PRODUCTION_PRESENTATION_NOTES_REQUIRED")
    note_name = "ppt/notesSlides/" + Path(target).name
    require(note_name in parts, "PRODUCTION_PRESENTATION_NOTES_REQUIRED")
    value = "\n".join(node.text or "" for node in xml(parts[note_name]).findall(".//a:t", NS)).strip()
    require(1 <= len(value) <= 6000, "PRODUCTION_PRESENTATION_NOTES_REQUIRED")
    return value


def narrated_presentation(args, tools):
    exact(args, ("source", "segments"), ("fps",))
    source_path = source(args["source"])
    require(source_path.suffix.lower() == ".pptx", "PRODUCTION_PRESENTATION_FORMAT_UNSUPPORTED")
    parts = read_zip(source_path)
    require(not presentation_active_content(parts), "PRODUCTION_UNSAFE_DOCUMENT")
    slides = presentation_slide_names(parts)
    require(1 <= len(slides) <= 8, "PRODUCTION_PRESENTATION_NARRATION_LIMIT")
    segments = args["segments"]
    require(isinstance(segments, list) and len(segments) == len(slides),
            "PRODUCTION_PRESENTATION_NARRATION_COUNT")
    fps = args.get("fps", 30)
    require(type(fps) is int and fps in (24, 25, 30))
    notes = [presentation_notes(parts, slide_name) for slide_name in slides]
    checked = []
    total = 0.0
    for index, (note, segment) in enumerate(zip(notes, segments), 1):
        exact(segment, ("slide", "source"))
        require(type(segment["slide"]) is int and segment["slide"] == index,
                "PRODUCTION_PRESENTATION_NARRATION_ORDER")
        audio = source(segment["source"])
        info = media_info(tools, audio)
        streams = info.get("streams", [])
        require(sum(stream.get("codec_type") == "audio" for stream in streams) == 1
                and not any(stream.get("codec_type") == "video" for stream in streams),
                "PRODUCTION_PRESENTATION_NARRATION_AUDIO_REQUIRED")
        duration = float(info.get("format", {}).get("duration", 0))
        require(math.isfinite(duration) and .25 <= duration <= 60,
                "PRODUCTION_PRESENTATION_NARRATION_DURATION")
        total += duration
        checked.append({"slide": index, "audio": audio, "duration": duration,
                        "audioSha256": hashlib.sha256(audio.read_bytes()).hexdigest(),
                        "notesSha256": hashlib.sha256(note.encode("utf-8")).hexdigest(),
                        "notesCharacters": len(note)})
    require(total <= 180, "PRODUCTION_PRESENTATION_NARRATION_DURATION")
    deck = ROOT / "presentation-source.pptx"
    deck.write_bytes(source_path.read_bytes())
    previews = presentation_previews(deck, len(slides), tools)
    segment_dir = ROOT / "presentation-video-segments"
    segment_dir.mkdir()
    common = ["-nostdin", "-hide_banner", "-loglevel", "error", "-n"]
    pieces = []
    for item in checked:
        image = OUT / f'slide-{item["slide"]:02}.png'
        target = segment_dir / f'segment-{item["slide"]:02}.mp4'
        run_tool(tools, "ffmpeg", common + ["-loop", "1", "-framerate", str(fps),
                 "-f", "image2", "-i", str(image), *input_format(item["audio"]),
                 "-i", str(item["audio"]), "-t", f'{item["duration"]:.6f}',
                 "-map", "0:v:0", "-map", "1:a:0", "-vf", "format=yuv420p",
                 "-c:v", "mpeg4", "-q:v", "3", "-c:a", "aac", "-ar", "48000",
                 "-ac", "2", "-shortest", "-movflags", "+faststart", str(target)])
        require(target.is_file() and target.stat().st_size > 32,
                "PRODUCTION_PRESENTATION_VIDEO_INVALID")
        pieces.append(target)
    concat = ROOT / "presentation-video-concat.txt"
    concat.write_text("".join(f"file '{path.relative_to(ROOT).as_posix()}'\n"
                              for path in pieces), encoding="utf-8")
    video = OUT / "presentation-video.mp4"
    run_tool(tools, "ffmpeg", common + ["-f", "concat", "-safe", "1", "-i", str(concat),
             "-map", "0:v:0", "-map", "0:a:0", "-c", "copy", "-movflags",
             "+faststart", str(video)])
    result = media_info(tools, video)
    streams = result.get("streams", [])
    video_stream = next((stream for stream in streams if stream.get("codec_type") == "video"), None)
    audio_stream = next((stream for stream in streams if stream.get("codec_type") == "audio"), None)
    actual = float(result.get("format", {}).get("duration", 0))
    require(video_stream is not None and video_stream.get("width") == 1280
            and video_stream.get("height") == 720 and audio_stream is not None,
            "PRODUCTION_PRESENTATION_VIDEO_INVALID")
    require(abs(actual - total) <= max(.5, total * .01),
            "PRODUCTION_DURATION_MISMATCH")
    run_tool(tools, "ffmpeg", ["-nostdin", "-hide_banner", "-loglevel", "error",
             *input_format(video), "-i", str(video), "-map", "0:v:0", "-map", "0:a:0",
             "-f", "null", "-"])
    frames = []
    for label, moment in (("start", 0), ("middle", actual / 2),
                          ("end", max(0, min(actual, total) - .5))):
        frame = OUT / f"video-frame-{label}.png"
        run_tool(tools, "ffmpeg", ["-nostdin", "-hide_banner", "-loglevel", "error", "-n",
                 *input_format(video), "-ss", f"{moment:.6f}", "-i", str(video),
                 "-frames:v", "1", "-f", "image2", str(frame)])
        require(frame.is_file(), "PRODUCTION_PRESENTATION_VIDEO_INVALID")
        width, height = raster_dimensions(frame.read_bytes(), "png")
        require((width, height) == (1280, 720), "PRODUCTION_PRESENTATION_VIDEO_INVALID")
        frames.append({"label": label, "seconds": round(moment, 6), "file": frame.name})
    manifest = {"kind": "narrated-presentation", "sourceSha256": hashlib.sha256(
        source_path.read_bytes()).hexdigest(), "slideCount": len(slides), "fps": fps,
        "durationSeconds": actual, "checkedNarrationSeconds": total,
        "segments": [{key: value for key, value in item.items() if key != "audio"}
                     for item in checked], "renders": previews, "frames": frames,
        "streams": streams, "verification": {"allSlidesRendered": True,
        "fullVideoDecoded": True, "durationMatched": True, "speakerNotesPresent": True,
        "spokenTextVerified": False}}
    (OUT / "presentation-video-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8")


def motion_video(args, tools):
    exact(args, ("title", "scenes", "motion", "fps"))
    title = text(args["title"], 120).strip()
    require(title, "PRODUCTION_VIDEO_MOTION_TITLE_REQUIRED")
    motion = args["motion"]
    require(motion in ("calm", "push", "drift"), "PRODUCTION_VIDEO_MOTION_INVALID")
    fps = args["fps"]
    require(type(fps) is int and fps in (24, 25, 30))
    scenes = args["scenes"]
    require(isinstance(scenes, list) and 2 <= len(scenes) <= 8,
            "PRODUCTION_VIDEO_MOTION_SCENES_INVALID")
    checked = []
    total = 0.0
    for index, scene in enumerate(scenes, 1):
        exact(scene, ("layout", "duration", "eyebrow", "title", "subtitle"))
        require(scene["layout"] in ("cover", "statement"),
                "PRODUCTION_VIDEO_MOTION_LAYOUT_INVALID")
        duration = number(scene["duration"], 2, 12)
        slide = presentation_slide({key: scene[key] for key in
                                    ("layout", "eyebrow", "title", "subtitle")})
        total += duration
        checked.append({"scene": index, "duration": duration, "slide": slide})
    require(total <= 90, "PRODUCTION_VIDEO_MOTION_DURATION_INVALID")

    presentations({"slides": [{key: value for key, value in item["slide"].items()
                                if key in ("layout", "eyebrow", "title", "subtitle")}
                               for item in checked], "render": True},
                  "presentations.create", tools)
    deck = OUT / "presentation.pptx"
    storyboard = OUT / "motion-storyboard.pptx"
    deck.rename(storyboard)
    (OUT / "presentation-manifest.json").rename(OUT / "motion-storyboard.json")

    segment_dir = ROOT / "motion-video-segments"
    segment_dir.mkdir()
    common = ["-nostdin", "-hide_banner", "-loglevel", "error", "-n"]
    pieces = []
    motion_proofs = []
    for item in checked:
        image = OUT / f'slide-{item["scene"]:02}.png'
        target = segment_dir / f'segment-{item["scene"]:02}.mp4'
        duration = item["duration"]
        frames = max(1, round(duration * fps))
        if motion == "drift":
            x = (f"(iw-iw/zoom)*on/{frames}" if item["scene"] % 2 else
                 f"(iw-iw/zoom)*(1-on/{frames})")
            zoom = "1.08"
        else:
            x = "iw/2-(iw/zoom/2)"
            step = "0.00035" if motion == "calm" else "0.0012"
            maximum = "1.035" if motion == "calm" else "1.08"
            zoom = f"min(zoom+{step},{maximum})"
        fade_out = max(.35, duration - .35)
        video_filter = ("scale=1408:792:force_original_aspect_ratio=increase,"
                        "crop=1408:792,"
                        f"zoompan=z='{zoom}':x='{x}':y='ih/2-(ih/zoom/2)':"
                        f"d=1:s=1280x720:fps={fps},"
                        f"fade=t=in:st=0:d=0.35,fade=t=out:st={fade_out:.6f}:d=0.35,"
                        "format=yuv420p")
        run_tool(tools, "ffmpeg", common + ["-loop", "1", "-framerate", str(fps),
                 "-f", "image2", "-i", str(image), "-f", "lavfi", "-i",
                 "anullsrc=r=48000:cl=stereo", "-t", f"{duration:.6f}",
                 "-map", "0:v:0", "-map", "1:a:0", "-vf", video_filter,
                 "-c:v", "mpeg4", "-q:v", "3", "-c:a", "aac", "-ar", "48000",
                 "-ac", "2", "-shortest", "-movflags", "+faststart", str(target)])
        require(target.is_file() and target.stat().st_size > 32,
                "PRODUCTION_VIDEO_MOTION_OUTPUT_INVALID")
        proof = []
        for label, moment in (("early", .75), ("late", duration - .75)):
            frame = ROOT / f'motion-proof-{item["scene"]:02}-{label}.png'
            run_tool(tools, "ffmpeg", common + [*input_format(target), "-ss", f"{moment:.6f}",
                     "-i", str(target), "-frames:v", "1", "-f", "image2", str(frame)])
            require(frame.is_file(), "PRODUCTION_VIDEO_MOTION_OUTPUT_INVALID")
            proof.append({"seconds": moment,
                          "sha256": hashlib.sha256(frame.read_bytes()).hexdigest()})
        require(proof[0]["sha256"] != proof[1]["sha256"],
                "PRODUCTION_VIDEO_MOTION_NOT_DETECTED")
        motion_proofs.append({"scene": item["scene"], "frames": proof})
        pieces.append(target)
    concat = ROOT / "motion-video-concat.txt"
    concat.write_text("".join(f"file '{path.relative_to(ROOT).as_posix()}'\n"
                              for path in pieces), encoding="utf-8")
    video = OUT / "motion-video.mp4"
    run_tool(tools, "ffmpeg", common + ["-f", "concat", "-safe", "1", "-i", str(concat),
             "-map", "0:v:0", "-map", "0:a:0", "-c", "copy", "-movflags",
             "+faststart", str(video)])
    result = media_info(tools, video)
    streams = result.get("streams", [])
    video_stream = next((stream for stream in streams if stream.get("codec_type") == "video"), None)
    audio_stream = next((stream for stream in streams if stream.get("codec_type") == "audio"), None)
    actual = float(result.get("format", {}).get("duration", 0))
    require(video_stream is not None and video_stream.get("width") == 1280
            and video_stream.get("height") == 720 and audio_stream is not None,
            "PRODUCTION_VIDEO_MOTION_OUTPUT_INVALID")
    require(abs(actual - total) <= max(.5, total * .01),
            "PRODUCTION_DURATION_MISMATCH")
    run_tool(tools, "ffmpeg", ["-nostdin", "-hide_banner", "-loglevel", "error",
             *input_format(video), "-i", str(video), "-map", "0:v:0", "-map", "0:a:0",
             "-f", "null", "-"])
    middle_index = len(checked) // 2
    middle = sum(item["duration"] for item in checked[:middle_index]) + checked[middle_index]["duration"] / 2
    review_frames = []
    for label, moment in (("start", .75), ("middle", middle),
                          ("end", max(.75, actual - .75))):
        frame = OUT / f"motion-frame-{label}.png"
        run_tool(tools, "ffmpeg", common + [*input_format(video), "-ss", f"{moment:.6f}",
                 "-i", str(video), "-frames:v", "1", "-f", "image2", str(frame)])
        width, height = raster_dimensions(frame.read_bytes(), "png")
        require((width, height) == (1280, 720),
                "PRODUCTION_VIDEO_MOTION_OUTPUT_INVALID")
        review_frames.append({"label": label, "seconds": round(moment, 6),
                              "file": frame.name})
    manifest = {"kind": "motion-video", "title": title, "motion": motion,
                "fps": fps, "durationSeconds": actual,
                "storyboardSha256": hashlib.sha256(storyboard.read_bytes()).hexdigest(),
                "outputSha256": hashlib.sha256(video.read_bytes()).hexdigest(),
                "scenes": [{"scene": item["scene"], "duration": item["duration"],
                            **{key: item["slide"][key] for key in
                               ("layout", "eyebrow", "title", "subtitle")}}
                           for item in checked], "motionProofs": motion_proofs,
                "frames": review_frames, "streams": streams,
                "verification": {"storyboardRendered": True, "motionFramesDiffer": True,
                                 "fullVideoDecoded": True, "durationMatched": True,
                                 "dimensionsMatched": True, "audioPresent": True,
                                 "visualQualityReviewed": False}}
    (OUT / "motion-video-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, allow_nan=False),
        encoding="utf-8")


def apply_presentation_brand(theme, brand):
    node = xml(theme)
    scheme = node.find(".//a:clrScheme", NS)
    font_scheme = node.find(".//a:fontScheme", NS)
    require(scheme is not None and font_scheme is not None,
            "PRODUCTION_PRESENTATION_THEME_UNSUPPORTED")
    colors = {"dk1": brand["text"], "lt1": brand["background"], "dk2": brand["muted"],
              "lt2": "FFFFFF", "accent1": brand["primary"], "accent2": brand["accent"],
              "accent3": brand["muted"], "hlink": brand["primary"], "folHlink": brand["accent"]}
    for key, value in colors.items():
        target = scheme.find(f"a:{key}", NS)
        if target is not None:
            for child in list(target):
                target.remove(child)
            ET.SubElement(target, f'{{{NS["a"]}}}srgbClr', {"val": value})
    for target in font_scheme.findall(".//a:latin", NS):
        target.set("typeface", brand["font"])
    scheme.set("name", brand["name"])
    font_scheme.set("name", brand["name"])
    return ET.tostring(node, encoding="utf-8", xml_declaration=True)


def presentations(args, operation, tools):
    render = args.get("render", False)
    require(type(render) is bool)
    if operation == "presentations.create":
        exact(args, ("slides",), ("brand", "render"))
        slides = args["slides"]
        require(isinstance(slides, list) and 1 <= len(slides) <= 60)
        slides = [presentation_slide(slide) for slide in slides]
        if render:
            require(len(slides) <= 12, "PRODUCTION_PRESENTATION_RENDER_LIMIT")
        brand = presentation_brand(args.get("brand"))
        parts = presentation_skeleton(brand, len(slides), any(slide["notes"] for slide in slides))
        media = {}
        manifest_slides = []
        for index, slide in enumerate(slides, 1):
            notes_target = f"../notesSlides/notesSlide{index}.xml" if slide["notes"] else None
            document, relationships, manifest = presentation_slide_xml(
                slide, index, len(slides), brand, media, notes_target=notes_target)
            parts[f"ppt/slides/slide{index}.xml"] = document
            parts[f"ppt/slides/_rels/slide{index}.xml.rels"] = relationships
            if slide["notes"]:
                parts[f"ppt/notesSlides/notesSlide{index}.xml"] = notes_slide_xml(slide["notes"])
                parts[f"ppt/notesSlides/_rels/notesSlide{index}.xml.rels"] = rels([
                    (1, "notesMaster", "../notesMasters/notesMaster1.xml"),
                    (2, "slide", f"../slides/slide{index}.xml"),
                ])
            manifest_slides.append(manifest)
        for entry in media.values():
            parts[entry["path"]] = entry["bytes"]
        parts["[Content_Types].xml"] = presentation_content_types(parts)
        deck = write_zip("presentation.pptx", parts)
        checked = read_zip(deck)
        names = presentation_slide_names(checked)
        require(len(names) == len(slides) and all(slides[i]["title"] in
                [node.text or "" for node in xml(checked[name]).findall(".//a:t", NS)]
                for i, name in enumerate(names)), "PRODUCTION_PRESENTATION_INVALID")
        manifest = {"kind": "presentation", "operation": "create", "slides": manifest_slides,
                    "theme": {key: brand[key] for key in DEFAULT_PRESENTATION_BRAND},
                    "slideCount": len(slides), "editable": True,
                    "verification": {"packageReopened": True, "titlesMatched": True}}
    elif operation == "presentations.revise":
        exact(args, ("source", "sectionId", "title"),
              ("layout", "eyebrow", "subtitle", "body", "bullets", "notes",
               "footer", "image", "chart", "render"))
        source_path = source(args["source"])
        require(source_path.suffix.lower() == ".pptx", "PRODUCTION_PRESENTATION_FORMAT_UNSUPPORTED")
        parts = read_zip(source_path)
        require(not presentation_active_content(parts), "PRODUCTION_UNSAFE_DOCUMENT")
        names = presentation_slide_names(parts)
        require(1 <= len(names) <= 60)
        match = re.fullmatch(r"slide:([1-9][0-9]?)", text(args["sectionId"], 20))
        require(match is not None and int(match[1]) <= len(names), "PRODUCTION_SECTION_NOT_FOUND")
        target_index = int(match[1])
        target = f"ppt/slides/slide{target_index}.xml"
        before = {name: hashlib.sha256(parts[name]).hexdigest() for name in names}
        spec = {key: value for key, value in args.items()
                if key not in ("source", "sectionId", "render")}
        if not any(key in spec for key in ("body", "bullets", "chart", "image")) and "layout" not in spec:
            spec["layout"] = "statement"
        slide = presentation_slide(spec)
        rel_name = f"ppt/slides/_rels/slide{target_index}.xml.rels"
        layout_target = "../slideLayouts/slideLayout1.xml"
        notes_target = None
        if rel_name in parts:
            rel_node = xml(parts[rel_name])
            for relationship in rel_node:
                if relationship.attrib.get("Type", "").endswith("/slideLayout"):
                    layout_target = relationship.attrib["Target"]
                elif relationship.attrib.get("Type", "").endswith("/notesSlide"):
                    notes_target = relationship.attrib["Target"]
        if slide["notes"] and notes_target is None:
            require("ppt/notesMasters/notesMaster1.xml" in parts,
                    "PRODUCTION_PRESENTATION_NOTES_UNSUPPORTED")
            notes_target = f"../notesSlides/notesSlide{target_index}.xml"
        media = {}
        brand = presentation_brand()
        brand["name"] = ""
        document, relationships, slide_manifest = presentation_slide_xml(
            slide, target_index, len(names), brand, media, parts, layout_target, notes_target)
        parts[target] = document
        parts[rel_name] = relationships
        for entry in media.values():
            parts[entry["path"]] = entry["bytes"]
        if slide["notes"]:
            note_name = "ppt/notesSlides/" + Path(notes_target).name
            parts[note_name] = notes_slide_xml(slide["notes"])
            parts["ppt/notesSlides/_rels/" + Path(note_name).name + ".rels"] = rels([
                (1, "notesMaster", "../notesMasters/notesMaster1.xml"),
                (2, "slide", f"../slides/slide{target_index}.xml"),
            ])
        update_presentation_content_types(parts)
        deck = write_zip("presentation.pptx", parts)
        checked = read_zip(deck)
        after = {name: hashlib.sha256(checked[name]).hexdigest() for name in names}
        require(all(before[name] == after[name] for name in names if name != target),
                "PRODUCTION_PRESENTATION_PRESERVATION_FAILED")
        manifest = {"kind": "presentation", "operation": "revise",
                    "sourceSha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
                    "targetSlide": target_index, "slide": slide_manifest,
                    "slideCount": len(names), "editable": True,
                    "verification": {"packageReopened": True,
                                     "nonTargetSlidesPreserved": len(names) - 1}}
    else:
        exact(args, ("source", "brand"), ("render",))
        source_path = source(args["source"])
        require(source_path.suffix.lower() == ".pptx", "PRODUCTION_PRESENTATION_FORMAT_UNSUPPORTED")
        parts = read_zip(source_path)
        require(not presentation_active_content(parts), "PRODUCTION_UNSAFE_DOCUMENT")
        names = presentation_slide_names(parts)
        require(1 <= len(names) <= 60)
        brand = presentation_brand(args["brand"])
        preserved = {name: hashlib.sha256(parts[name]).hexdigest() for name in parts
                     if name.startswith(("ppt/slides/", "ppt/notesSlides/", "ppt/media/"))}
        theme_names = [name for name in parts if re.fullmatch(r"ppt/theme/theme[0-9]+\.xml", name)]
        require(theme_names, "PRODUCTION_PRESENTATION_THEME_UNSUPPORTED")
        for name in theme_names:
            parts[name] = apply_presentation_brand(parts[name], brand)
        scheme_aware = explicit = 0
        for name in names:
            node = xml(parts[name])
            scheme_aware += len(node.findall(".//a:schemeClr", NS))
            explicit += len(node.findall(".//a:srgbClr", NS))
        deck = write_zip("presentation.pptx", parts)
        checked = read_zip(deck)
        require(all(hashlib.sha256(checked[name]).hexdigest() == digest
                    for name, digest in preserved.items()),
                "PRODUCTION_PRESENTATION_PRESERVATION_FAILED")
        manifest = {"kind": "presentation", "operation": "brand",
                    "sourceSha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
                    "theme": {key: brand[key] for key in DEFAULT_PRESENTATION_BRAND},
                    "slideCount": len(names), "editable": True,
                    "coverage": {"schemeAwareElements": scheme_aware,
                                 "explicitColorElements": explicit,
                                 "partialForExplicitColors": explicit > 0},
                    "verification": {"packageReopened": True,
                                     "slideNotesAndMediaPartsPreserved": len(preserved)}}
    if render:
        manifest["renders"] = presentation_previews(deck, manifest["slideCount"], tools)
        manifest["verification"]["allSlidesRendered"] = True
    (OUT / "presentation-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2, allow_nan=False), encoding="utf-8")
    return manifest


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


def run_tool_report(tools, name, args):
    executable=tools.get(name)
    require(isinstance(executable,str) and Path(executable).is_absolute(),'PRODUCTION_TOOL_UNAVAILABLE')
    with tempfile.TemporaryFile() as output:
        def limits():
            import resource
            resource.setrlimit(resource.RLIMIT_FSIZE,(MAX_FILE,MAX_FILE))
        result=subprocess.run([executable,*args],stdout=subprocess.DEVNULL,stderr=output,
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
    raw=run_tool(tools,'ffprobe',['-v','error',*input_format(path),'-show_entries','format=duration:stream=codec_type,codec_name,width,height,sample_rate,channels','-of','json',str(path)])
    return json.loads(raw)


def audio_loudness(tools, path, target, filters=()):
    chain=list(filters)+[f'loudnorm=I={target}:TP=-1.5:LRA=11:print_format=json']
    raw=run_tool_report(tools,'ffmpeg',['-nostdin','-hide_banner','-nostats','-loglevel','info',
                        *input_format(path),'-i',str(path),'-vn','-af',','.join(chain),
                        '-f','null','-']).decode('utf-8','replace')
    start=raw.rfind('{');end=raw.rfind('}')
    require(0<=start<end,'PRODUCTION_AUDIO_LEVEL_UNMEASURABLE')
    report=json.loads(raw[start:end+1])
    result={key:float(report[key]) for key in
            ('input_i','input_tp','input_lra','input_thresh','target_offset')}
    require(all(math.isfinite(value) for value in result.values()),
            'PRODUCTION_AUDIO_LEVEL_UNMEASURABLE')
    return result


def media(args,operation,tools):
    common=['-nostdin','-hide_banner','-loglevel','error','-n','-protocol_whitelist','file,pipe']
    if operation=='images.resize':
        exact(args,('source','width','height'))
        w=int(number(args['width'],1,4096));h=int(number(args['height'],1,4096))
        run_tool(tools,'ffmpeg',common+input_format(source(args['source']))+['-i',str(source(args['source'])),'-vf',f'scale={w}:{h}:force_original_aspect_ratio=decrease','-frames:v','1',str(OUT/'image.png')])
    elif operation=='audio.cleanup':
        exact(args,('source','targetLufs','treatment'))
        target=args['targetLufs']
        require(type(target) in (int,float) and math.isfinite(target) and -24<=target<=-14,
                'PRODUCTION_AUDIO_CLEANUP_TARGET_INVALID')
        treatment=args['treatment']
        require(treatment in ('level-only','gentle','voice'),
                'PRODUCTION_AUDIO_CLEANUP_TREATMENT_INVALID')
        input_path=source(args['source']);source_digest=hashlib.sha256(input_path.read_bytes()).hexdigest()
        before=media_info(tools,input_path)
        duration=float(before.get('format',{}).get('duration',0))
        require(0<duration<=120,'PRODUCTION_DURATION_LIMIT')
        stream=next((s for s in before.get('streams',[]) if s.get('codec_type')=='audio'),None)
        require(stream is not None,'PRODUCTION_AUDIO_REQUIRED')
        channels=stream.get('channels')
        require(type(channels) is int and channels in (1,2),
                'PRODUCTION_AUDIO_CHANNELS_UNSUPPORTED')
        filters={'level-only':(),
                 'gentle':('highpass=f=60','afftdn=nf=-40'),
                 'voice':('highpass=f=80','lowpass=f=12000','afftdn=nf=-35')}[treatment]
        raw_before=audio_loudness(tools,input_path,target)
        measured=audio_loudness(tools,input_path,target,filters)
        normalize=(f'loudnorm=I={target}:TP=-1.5:LRA=11:'
                   f'measured_I={measured["input_i"]}:measured_TP={measured["input_tp"]}:'
                   f'measured_LRA={measured["input_lra"]}:measured_thresh={measured["input_thresh"]}:'
                   f'offset={measured["target_offset"]}:linear=true:print_format=summary')
        output=OUT/'cleaned-audio.wav'
        run_tool(tools,'ffmpeg',common+input_format(input_path)+['-i',str(input_path),'-vn',
                 '-af',','.join((*filters,normalize)),'-ar','48000','-ac',str(channels),
                 '-c:a','pcm_s16le',str(output)])
        after=media_info(tools,output)
        after_stream=next((s for s in after.get('streams',[]) if s.get('codec_type')=='audio'),None)
        actual=float(after.get('format',{}).get('duration',0))
        require(after_stream is not None and after_stream.get('sample_rate')=='48000'
                and after_stream.get('channels')==channels,
                'PRODUCTION_AUDIO_OUTPUT_INVALID')
        require(abs(actual-duration)<max(0.2,duration*.005),'PRODUCTION_DURATION_MISMATCH')
        run_tool(tools,'ffmpeg',['-nostdin','-hide_banner','-loglevel','error',
                 *input_format(output),'-i',str(output),'-map','0:a:0','-f','null','-'])
        measured_after=audio_loudness(tools,output,target)
        difference=abs(measured_after['input_i']-target)
        require(difference<=1,'PRODUCTION_AUDIO_LEVEL_MISMATCH')
        require(hashlib.sha256(input_path.read_bytes()).hexdigest()==source_digest,
                'PRODUCTION_SOURCE_CHANGED')
        manifest={'kind':'audio-cleanup','policy':{'treatment':treatment,'targetLufs':target},
                  'source':{'sha256':source_digest,
                            'durationSeconds':duration,'codec':stream.get('codec_name'),
                            'sampleRate':stream.get('sample_rate'),'channels':channels},
                  'output':{'file':output.name,
                            'sha256':hashlib.sha256(output.read_bytes()).hexdigest(),
                            'durationSeconds':actual,'codec':after_stream.get('codec_name'),
                            'sampleRate':after_stream.get('sample_rate'),'channels':channels},
                  'measurement':{'before':{'integratedLufs':raw_before['input_i'],
                                           'truePeakDb':raw_before['input_tp'],
                                           'loudnessRangeLu':raw_before['input_lra']},
                                 'after':{'integratedLufs':measured_after['input_i'],
                                          'truePeakDb':measured_after['input_tp'],
                                          'loudnessRangeLu':measured_after['input_lra']},
                                 'targetDifferenceLu':difference},
                  'verification':{'originalPreserved':True,'durationMatched':True,
                                  'channelsPreserved':True,'targetMatched':True,
                                  'fullAudioDecoded':True,'subjectiveQualityReviewed':False}}
        (OUT/'audio-cleanup-manifest.json').write_text(
            json.dumps(manifest,ensure_ascii=False,indent=2,allow_nan=False),encoding='utf-8')
        return
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
    if operation=='video.edit':
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
                node=xml(parts[name]);title='';body=[]
                for shape in node.findall('.//p:sp',NS):
                    identity=shape.find('./p:nvSpPr/p:cNvPr',NS)
                    lines=[item.text or '' for item in shape.findall('.//a:t',NS)]
                    if identity is not None and identity.attrib.get('name') in ('Title','Statement') and lines:
                        title=lines[0];body.extend(lines[1:])
                    else: body.extend(lines)
                if not title:
                    lines=[n.text or '' for n in node.findall('.//a:t',NS)]
                    title=lines[0] if lines else '';body=lines[1:]
                pages.append({'id':f'slide:{re.search(r"slide([0-9]+)",name)[1]}','title':title, 'text':'\n'.join(body)[:20000]})
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
    if operation in ('images.verify','images.batch','images.mask','pdf.extract','pdf.merge','resources.ocr','audio.transcribe','audio.podcast','data.chart','data.scenario','data.extract','documents.brief','documents.rewrite','documents.template','research.claims','research.compare','research.synthesis','research.changes','documents.render','video.frame','video.package','video.highlights','video.compose','interactive.create'):
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
    if operation == 'video.motion':
        motion_video(args,tools)
    elif operation == 'presentations.narrate':
        narrated_presentation(args,tools)
    elif operation in ('presentations.create','presentations.revise','presentations.brand'):
        presentations(args,operation,tools)
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
