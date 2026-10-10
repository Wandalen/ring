#!/usr/bin/env python3
"""Concatenate tmp/explainer/sections/*.html into tmp/explainer/index.html using shell.html.

Usage: build.py [--only NN]   (NN = two-digit section prefix; builds that section alone for previewing)
Prints basic sanity checks: one <section> per file, id/data-title present, balanced <style>/<script>, no <html>/<body>.
"""
import re, sys, pathlib

root = pathlib.Path(__file__).parent
shell = (root / 'shell.html').read_text()
only = None
if '--only' in sys.argv:
    only = sys.argv[sys.argv.index('--only') + 1]

files = sorted((root / 'sections').glob('*.html'))
if only:
    files = [f for f in files if f.name.startswith(only)]
if not files:
    sys.exit('no sections found')

nav, body, problems = [], [], []
for i, f in enumerate(files, 1):
    s = f.read_text()
    m = re.search(r'<section\s+id="(sec-[\w-]+)"[^>]*data-title="([^"]+)"', s)
    if not m:
        problems.append(f'{f.name}: missing <section id="sec-..." data-title="...">'); continue
    sid, title = m.groups()
    if s.count('<section') != 1 or s.count('</section>') != 1: problems.append(f'{f.name}: expected exactly one <section>')
    if s.count('<style') != s.count('</style>'): problems.append(f'{f.name}: unbalanced <style>')
    if s.count('<script') != s.count('</script>'): problems.append(f'{f.name}: unbalanced <script>')
    for bad in ('<html', '<body', '<head'):
        if bad in s: problems.append(f'{f.name}: contains {bad}')
    nav.append(f'<a href="#{sid}" data-target="{sid}">{i}. {title}</a>')
    body.append(s.replace('<h2>', f'<h2><span class="num">{i:02d}</span>', 1))

out = shell.replace('<!-- NAV -->', '\n    '.join(nav)).replace('<!-- SECTIONS -->', '\n'.join(body))
(root / 'index.html').write_text(out)
print(f'built index.html from {len(files)} section(s): ' + ', '.join(f.name for f in files))
print(f'size: {len(out.encode()) / 1024:.0f} KB')
for p in problems: print('PROBLEM:', p)
sys.exit(1 if problems else 0)
