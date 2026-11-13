"""Render bundled official Gemma HTML snapshots as readable offline text.
Original HTML and provenance remain in legal/third-party/models/gemma-3.
No network, third-party packages, or user data.
"""
from html.parser import HTMLParser
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parent.parent


class Article(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.depth = 0
        self.found = False
        self.skip = None
        self.parts = []
        self.lists = []
        self.links = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == "div":
            if self.depth:
                self.depth += 1
            elif "devsite-article-body" in attrs.get("class", "").split():
                self.depth = 1
                self.found = True
        if not self.depth:
            return
        if tag in ("script", "style"):
            self.skip = tag
        if self.skip:
            return
        if tag in ("p", "h1", "h2", "h3", "h4", "table"):
            self.parts.append("\n\n")
        elif tag == "br":
            self.parts.append("\n")
        elif tag in ("ol", "ul"):
            self.lists.append(0 if tag == "ol" else None)
        elif tag == "li":
            label = "-"
            if self.lists and self.lists[-1] is not None:
                self.lists[-1] += 1
                label = str(self.lists[-1]) + "."
            self.parts.append("\n" + "  " * max(0, len(self.lists) - 1) + label + " ")
        elif tag == "a":
            self.links.append(attrs.get("href", ""))
        elif tag in ("td", "th"):
            self.parts.append(" | ")

    def handle_endtag(self, tag):
        if not self.depth:
            return
        if self.skip:
            if tag == self.skip:
                self.skip = None
            return
        if tag == "a" and self.links:
            href = self.links.pop()
            if href.startswith("https://"):
                self.parts.append(" (" + href + ")")
        elif tag in ("ol", "ul") and self.lists:
            self.lists.pop()
        elif tag in ("p", "h1", "h2", "h3", "h4", "tr"):
            self.parts.append("\n\n")
        elif tag == "div":
            self.depth -= 1

    def handle_data(self, data):
        if self.depth and not self.skip:
            self.parts.append(re.sub(r"\s+", " ", data))

    def text(self):
        if not self.found:
            raise ValueError("Official article body was not found")
        text = "".join(self.parts)
        text = re.sub(r"[ \t]+\n", "\n", text)
        return re.sub(r"\n{3,}", "\n\n", text).strip() + "\n"


def main():
    output = ROOT / "legal/model-agreements"
    output.mkdir(exist_ok=True)
    receipts = []
    for source_name, target, url, required in [
        ("TERMS.html", "gemma-3-terms.txt", "https://ai.google.dev/gemma/terms", ["Section 1", "Section 3", "Appendix", "Gemma 3"]),
        ("PROHIBITED-USE-POLICY.html", "gemma-3-policy.txt", "https://ai.google.dev/gemma/prohibited_use_policy", ["You may not use", "Last modified:", "sexually explicit"]),
    ]:
        source = ROOT / "legal/third-party/models/gemma-3" / source_name
        raw = source.read_bytes()
        article = Article()
        article.feed(raw.decode("utf-8"))
        text = article.text()
        if len(text) < 1000 or any(part not in text for part in required):
            raise ValueError(f"Incomplete agreement extraction: {source_name}")
        encoded = (f"Source: {url}\nOriginal HTML is bundled in third-party notices.\n\n" + text).encode("utf-8")
        (output / target).write_bytes(encoded)
        receipts.append({"source": source.relative_to(ROOT).as_posix(), "url": url,
                         "source_sha256": hashlib.sha256(raw).hexdigest(), "output": target,
                         "output_sha256": hashlib.sha256(encoded).hexdigest()})
    (output / "provenance.json").write_text(json.dumps(receipts, indent=2) + "\n", encoding="utf-8")
    print("Rendered and verified two bundled model agreements; original HTML retained.")


if __name__ == "__main__":
    main()
