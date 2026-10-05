# -*- coding: utf-8 -*-
"""HTML 清洗与题号提取工具"""
import re
from html.parser import HTMLParser

_HTML_ENTITIES = {
    "amp": "&", "lt": "<", "gt": ">", "quot": '"', "apos": "'",
    "nbsp": "\u00a0", "copy": "\u00a9", "reg": "\u00ae", "trade": "\u2122",
    "hellip": "\u2026", "mdash": "\u2014", "ndash": "\u2013",
    "ldquo": "\u201c", "rdquo": "\u201d", "lsquo": "\u2018", "rsquo": "\u2019",
}


class _TagStripper(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=False)
        self.text = []
        self.ignore = 0
        self.skip_tags = {"script", "style"}

    def handle_starttag(self, tag, attrs):
        if tag in self.skip_tags:
            self.ignore += 1

    def handle_endtag(self, tag):
        if tag in self.skip_tags and self.ignore > 0:
            self.ignore -= 1

    def handle_data(self, data):
        if self.ignore == 0:
            self.text.append(data)

    def handle_entityref(self, name):
        if self.ignore == 0:
            self.text.append(_HTML_ENTITIES.get(name, f"&{name};"))

    def handle_charref(self, name):
        if self.ignore == 0:
            try:
                code = int(name[1:], 16) if name.startswith(("x", "X")) else int(name)
                self.text.append(chr(code))
            except (ValueError, OverflowError):
                self.text.append(f"&#{name};")

    def handle_comment(self, data):
        pass


def strip_html(text):
    if not text:
        return ""
    stripper = _TagStripper()
    stripper.feed(str(text))
    return "".join(stripper.text)


_ETS_TH_RE = re.compile(r"^\s*ets_th(\d+)\s*", re.IGNORECASE)
_GLOBAL_NO_RE = re.compile(r"^\s*(\d+)\s*[.、,，)）]", re.IGNORECASE)


def split_br(text):
    if not text:
        return []
    return [strip_html(part).strip() for part in re.split(r"</?br\s*/?>", text) if strip_html(part).strip()]


def clean(text):
    return strip_html(str(text)).replace("\u200b", "").strip()


def clean_lines(text):
    if not text:
        return ""
    return "\n".join(
        line.strip()
        for line in re.split(r"</?br\s*/?>|</p>|<p[^>]*>", strip_html(text))
        if line.strip()
    )


def extract_ets_th(text):
    m = _ETS_TH_RE.match(text or "")
    return int(m.group(1)) if m else None


def strip_ets_th(text):
    m = _ETS_TH_RE.match(text or "")
    return text[m.end():].strip() if m else (text or "").strip()


def extract_global_no(text):
    m = _GLOBAL_NO_RE.match(text or "")
    return int(m.group(1)) if m else None


def strip_global_no(text):
    m = _GLOBAL_NO_RE.match(text or "")
    return text[m.end():].strip() if m else (text or "").strip()


def split_keywords(text, sep="|"):
    if not text:
        return []
    return [clean(k) for k in str(text).split(sep) if clean(k)]
