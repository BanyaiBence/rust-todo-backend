#!/usr/bin/env bash
# Builds the architecture documentation:
#   1. renders docs/architecture/*.puml to docs/rendered/*.svg (committed, shown in README.md)
#   2. converts the SVGs to vector PDFs in docs/build/figures/
#   3. compiles docs/architecture.tex into docs/build/architecture.pdf
#
# Usage: docs/build.sh [--svg-only]
#
# Needs: plantuml (or Docker), rsvg-convert (or inkscape), and latexmk with pdflatex.
set -euo pipefail

cd "$(dirname "$0")"

svg_only=false
case "${1:-}" in
  "") ;;
  --svg-only) svg_only=true ;;
  *) echo "usage: $0 [--svg-only]" >&2; exit 2 ;;
esac

die() { echo "error: $*" >&2; exit 1; }

# 1. PlantUML -> SVG (cleared first so renamed or deleted diagrams leave nothing behind)
rm -f rendered/*.svg
if command -v plantuml >/dev/null; then
  plantuml -tsvg -o ../rendered architecture/*.puml
elif command -v docker >/dev/null; then
  docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/docs" -w /docs/architecture \
    plantuml/plantuml -tsvg -o ../rendered ./*.puml
else
  die "plantuml or docker is required"
fi
echo "Rendered SVGs to $PWD/rendered/"

$svg_only && exit 0

# 2. SVG -> PDF, so LaTeX can embed the diagrams as vector graphics.
rm -rf build/figures
mkdir -p build/figures
for svg in rendered/*.svg; do
  pdf="build/figures/$(basename "${svg%.svg}").pdf"
  if command -v rsvg-convert >/dev/null; then
    rsvg-convert -f pdf -o "$pdf" "$svg"
  elif command -v inkscape >/dev/null; then
    inkscape "$svg" --export-type=pdf --export-filename="$pdf" >/dev/null 2>&1
  else
    die "rsvg-convert (librsvg) or inkscape is required to embed diagrams in the PDF"
  fi
done

# 3. LaTeX -> PDF
command -v latexmk >/dev/null || die "latexmk and pdflatex (TeX Live) are required to build the PDF"

revision=$(git describe --always --dirty 2>/dev/null || echo unknown)
printf '\\newcommand{\\docversion}{%s}\n' "$revision" > build/version.tex

latexmk -pdf -outdir=build -interaction=nonstopmode -halt-on-error -quiet architecture.tex \
  || die "LaTeX build failed; see $PWD/build/architecture.log"

echo "Built $PWD/build/architecture.pdf"
