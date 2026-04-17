#!/usr/bin/env python3
"""Renders a focused mock of the Conduit TUI sidebar showing ahead/behind indicators."""
import sys
import time

# ── ANSI helpers ──────────────────────────────────────────────────────────────

def fg(h):
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return f'\033[38;2;{r};{g};{b}m'

def bg(h):
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return f'\033[48;2;{r};{g};{b}m'

BOLD = '\033[1m'
R    = '\033[0m'
HIDE = '\033[?25l'

# ── Night Owl palette ─────────────────────────────────────────────────────────

BG   = '#011627'
BGS  = '#0d2137'
BGH  = '#1d3b53'
TEXT = '#d6deeb'
MUT  = '#5f7e97'
YEL  = '#ffcb8b'
RED  = '#ef5350'
GRN  = '#addb67'
TEAL = '#7fdbca'

TW = 52   # narrow terminal for this focused view

_row = 1  # current draw row (1-based)

def at(row):
    return f'\033[{row};1H'

def line(parts, default_bg=BGS):
    global _row
    vis = sum(len(t) for _, t in parts)
    out = at(_row)
    out += ''.join(col + text for col, text in parts)
    out += bg(default_bg) + ' ' * max(0, TW - vis) + R
    sys.stdout.write(out)
    _row += 1

def separator():
    global _row
    sys.stdout.write(at(_row) + fg(MUT) + bg(BG) + '─' * TW + R)
    _row += 1

# ── Render ─────────────────────────────────────────────────────────────────────

sys.stdout.write('\033[2J' + HIDE)  # clear screen, hide cursor

# Header
line([
    (bg(BG) + fg(MUT), ' Workspaces'),
    (bg(BG),           ' ' * (TW - 11)),
], default_bg=BG)

separator()

# Project: conduit
line([
    (bg(BGS) + fg(TEAL), ' ▼'),
    (bg(BGS) + fg(TEXT), ' conduit'),
])

# Active workspace: slow-fern with ↑2 ↓1
line([
    (bg(BGH) + fg(TEXT),        '   slow-fern     '),
    (bg(BGH) + fg(YEL) + BOLD,  '↑2'),
    (bg(BGH) + fg(TEXT),        ' '),
    (bg(BGH) + fg(RED) + BOLD,  '↓1'),
    (R + bg(BGH) + fg(MUT),     '  #61 '),
    (bg(BGH) + fg(GRN),         '✓'),
    (bg(BGH),                   ' ' * 4),
], default_bg=BGH)

# Workspace: main (no indicators)
line([(bg(BGS) + fg(MUT), '   main')])

# Empty row
line([], default_bg=BGS)

# Project: my-api
line([
    (bg(BGS) + fg(TEAL), ' ▼'),
    (bg(BGS) + fg(TEXT), ' my-api'),
])

# Workspace: feature/parser with ↑3
line([
    (bg(BGS) + fg(TEXT),       '   feature/parser   '),
    (bg(BGS) + fg(YEL) + BOLD, '↑3'),
    (bg(BGS),                  ' ' * 6),
])

# Workspace: main with ↓2
line([
    (bg(BGS) + fg(TEXT),      '   main             '),
    (bg(BGS) + fg(RED) + BOLD, '↓2'),
    (bg(BGS),                  ' ' * 6),
])

# Fill remaining rows with bg color
for r in range(_row, 16):
    sys.stdout.write(at(r) + bg(BGS) + ' ' * TW + R)

sys.stdout.write(at(1))  # park cursor at row 1
sys.stdout.flush()
time.sleep(10)
