"""Render the maintained English Markdown guide with the actual generated icon set."""
from __future__ import annotations
import argparse
import html
import io
import re
import tomllib
from pathlib import Path
from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, PageBreak, Table, TableStyle, Image
from PIL import Image as PillowImage

ROOT = Path(__file__).resolve().parents[1]

def inline(text: str) -> str:
    text = html.escape(text)
    text = re.sub(r"`([^`]+)`", r'<font name="Courier" size="9">\1</font>', text)
    return re.sub(r"\*\*([^*]+)\*\*", r"<b>\1</b>", text)

def icon_directory(version: str) -> Path:
    candidates = []
    for mode in ("release", "debug"):
        candidates.extend((ROOT / "target" / mode / "build").glob("attention-focus-*/out/icon-version.txt"))
    candidates = [p for p in candidates if p.read_text(encoding="utf-8").strip() == version]
    if not candidates:
        raise RuntimeError("Build the app first; current-version generated icons are required.")
    return max(candidates, key=lambda p: p.stat().st_mtime_ns).parent

def icon_image(directory: Path, name: str) -> Image:
    buffer = io.BytesIO()
    with PillowImage.open(directory / f"{name}.ico") as icon:
        icon.convert("RGBA").save(buffer, format="PNG")
    buffer.seek(0)
    return Image(buffer, width=28, height=28)

def render(output: Path) -> None:
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    directory = icon_directory(version)
    styles = getSampleStyleSheet()
    styles.add(ParagraphStyle(name="GuideBody", fontName="Helvetica", fontSize=10, leading=14, textColor=colors.HexColor("#243247"), spaceAfter=7))
    styles.add(ParagraphStyle(name="GuideItem", parent=styles["GuideBody"], leftIndent=14, firstLineIndent=-10, spaceAfter=5))
    styles["Heading1"].fontSize=23; styles["Heading1"].leading=28; styles["Heading1"].textColor=colors.HexColor("#213e69"); styles["Heading1"].spaceAfter=14
    styles["Heading2"].fontSize=13; styles["Heading2"].leading=17; styles["Heading2"].textColor=colors.HexColor("#213e69"); styles["Heading2"].spaceBefore=10; styles["Heading2"].spaceAfter=7
    lines=(ROOT / "docs/user-guide.md").read_text(encoding="utf-8").replace("{{VERSION}}",version).splitlines()
    story=[]
    i=0
    while i<len(lines):
        line=lines[i].strip()
        if not line: i+=1; continue
        if line=="<!-- pagebreak -->": story.append(PageBreak()); i+=1; continue
        if line=="<!-- icon-legend -->":
            legend=[("app","Application / Production","Blue diamond: identity only, never a running indicator."),("running","Production running","Green play symbol: tracking is active."),("stopped","Production stopped","Gray pause bars: tracking is stopped."),("app-development","Application / Development","Purple diamond: development identity only."),("running-development","Development running","Green play symbol with a purple D badge."),("stopped-development","Development stopped","Gray pause bars with a purple D badge.")]
            rows=[[icon_image(directory,name),Paragraph(f"<b>{title}</b><br/>{description}",styles["GuideBody"])] for name,title,description in legend]
            table=Table(rows,colWidths=[42,445],hAlign="LEFT")
            table.setStyle(TableStyle([("VALIGN",(0,0),(-1,-1),"MIDDLE"),("BACKGROUND",(0,0),(-1,-1),colors.HexColor("#f2f5fa")),("LINEBELOW",(0,0),(-1,-2),0.4,colors.white),("TOPPADDING",(0,0),(-1,-1),2),("BOTTOMPADDING",(0,0),(-1,-1),2)]))
            story.extend([table,Spacer(1,8)]); i+=1; continue
        if line.startswith("# "): story.append(Paragraph(inline(line[2:]),styles["Heading1"])); i+=1; continue
        if line.startswith("## "): story.append(Paragraph(inline(line[3:]),styles["Heading2"])); i+=1; continue
        match=re.match(r"^(\d+\.|-) (.*)",line)
        if match:
            prefix, text=match.groups();i+=1
            while i<len(lines) and lines[i].startswith("  "): text+=" "+lines[i].strip();i+=1
            story.append(Paragraph(inline(f"{prefix} {text}"),styles["GuideItem"]));continue
        paragraph=[line];i+=1
        while i<len(lines) and lines[i].strip() and not lines[i].startswith(("#","<!--","- ")) and not re.match(r"^\d+\. ",lines[i]): paragraph.append(lines[i].strip());i+=1
        story.append(Paragraph(inline(" ".join(paragraph)),styles["GuideBody"]))
    output.parent.mkdir(parents=True,exist_ok=True)
    def page(canvas,doc):
        canvas.saveState()
        canvas.setStrokeColor(colors.HexColor("#d7e0ec"));canvas.line(54,43,A4[0]-54,43)
        canvas.setFont("Helvetica",8);canvas.setFillColor(colors.HexColor("#52647c"))
        canvas.drawString(54,30,f"Attention Focus | User Guide | v{version}")
        canvas.drawRightString(A4[0]-54,30,f"{doc.page}");canvas.restoreState()
    document=SimpleDocTemplate(str(output),pagesize=A4,leftMargin=54,rightMargin=54,topMargin=42,bottomMargin=57,title=f"Attention Focus User Guide {version}",author="Attention Focus",subject=f"User guide for version {version}",pageCompression=1,invariant=1)
    document.build(story,onFirstPage=page,onLaterPages=page)

if __name__=="__main__":
    parser=argparse.ArgumentParser();parser.add_argument("--output",type=Path,required=True)
    render(parser.parse_args().output)
