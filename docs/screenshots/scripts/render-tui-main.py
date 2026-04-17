#!/usr/bin/env python3
"""Renders a mock Conduit TUI main view to the terminal using ANSI escape codes."""
import sys
import time

# ── ANSI helpers ──────────────────────────────────────────────────────────────

def fg(h):
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return f'\033[38;2;{r};{g};{b}m'

def bg(h):
    r, g, b = int(h[1:3], 16), int(h[3:5], 16), int(h[5:7], 16)
    return f'\033[48;2;{r};{g};{b}m'

BOLD  = '\033[1m'
R     = '\033[0m'
HIDE  = '\033[?25l'

# ── Night Owl palette ─────────────────────────────────────────────────────────

BG   = '#011627'
BGS  = '#0d2137'  # surface (sidebar bg)
BGH  = '#1d3b53'  # highlight (active row bg)
TEXT = '#d6deeb'
MUT  = '#5f7e97'
YEL  = '#ffcb8b'
RED  = '#ef5350'
GRN  = '#addb67'
TEAL = '#7fdbca'
PUR  = '#7e57c2'
ORG  = '#f78c6c'
BLU  = '#82aaff'

# ── Layout constants ──────────────────────────────────────────────────────────

TW = 158   # total terminal width (cols)
SW = 28    # sidebar visible width
CW = TW - SW - 1  # chat visible width (129)

_row = 1

def at(r):
    return f'\033[{r};1H'

# ── Line builders ─────────────────────────────────────────────────────────────

def sidebar_cell(parts, active=False):
    bg_col = BGH if active else BGS
    vis = sum(len(t) for _, t in parts)
    out = bg(bg_col)
    for col, text in parts:
        out += col + text
    out += bg(bg_col) + ' ' * max(0, SW - vis) + R
    return out

def chat_cell(parts):
    vis = sum(len(t) for _, t in parts)
    out = bg(BG)
    for col, text in parts:
        out += col + text
    out += bg(BG) + ' ' * max(0, CW - vis) + R
    return out

def row(sb, ch):
    global _row
    sep = fg(MUT) + bg(BG) + '│'
    sys.stdout.write(at(_row) + sb + sep + ch)
    _row += 1

def full_line(parts, default_bg=BG):
    global _row
    vis = sum(len(t) for _, t in parts)
    out = at(_row) + ''.join(col + text for col, text in parts)
    out += bg(default_bg) + ' ' * max(0, TW - vis) + R
    sys.stdout.write(out)
    _row += 1

def separator():
    global _row
    sys.stdout.write(at(_row) + fg(MUT) + bg(BG) + '─' * TW + R)
    _row += 1

def empty_row():
    row(sidebar_cell([]), chat_cell([]))

# ── Render ─────────────────────────────────────────────────────────────────────

sys.stdout.write('\033[2J' + HIDE)  # clear screen, hide cursor

# TAB BAR
tab_pad = TW - 39
full_line([
    (fg(TEAL) + bg(BG),           ' ●'),
    (fg(TEXT) + bg(BGH) + BOLD,   ' slow-fern '),
    (R + fg(MUT) + bg(BG),        ' conduit/main '),
    (fg(MUT) + bg(BG),            ' feature/ci '),
    (bg(BG),                      ' ' * tab_pad),
])

separator()

# CONTENT ROWS

empty_row()

# Project header: conduit
row(
    sidebar_cell([(fg(TEAL) + bg(BGS), ' ▼'), (fg(TEXT) + bg(BGS), ' conduit')]),
    chat_cell([])
)

# Active workspace + user message
row(
    sidebar_cell([
        (fg(TEXT) + bg(BGH), '  slow-fern  '),
        (fg(YEL)  + bg(BGH), '↑2'),
        (bg(BGH),             ' '),
    ], active=True),
    chat_cell([
        (fg(PUR) + bg(BG),  '▎'),
        (fg(TEXT) + bg(BG), ' Add error handling to the parser module.'),
    ])
)

# User message second line (trailing stripe) + sidebar empty
row(
    sidebar_cell([]),
    chat_cell([(fg(PUR) + bg(BG), '▎')])
)

empty_row()

# Project header: my-api + tool block header
row(
    sidebar_cell([(fg(TEAL) + bg(BGS), ' ▼'), (fg(TEXT) + bg(BGS), ' my-api')]),
    chat_cell([
        (fg(ORG) + bg(BG),        ' ┃'),
        (fg(ORG) + bg(BG) + BOLD, ' Bash '),
        (R + fg(MUT) + bg(BG),    ' cargo test -- parser_tests'),
    ])
)

row(
    sidebar_cell([(fg(TEXT) + bg(BGS), '   main')]),
    chat_cell([(fg(ORG) + bg(BG), ' ┃')])
)

row(
    sidebar_cell([(fg(TEXT) + bg(BGS), '   feature/auth')]),
    chat_cell([
        (fg(ORG) + bg(BG),  ' ┃'),
        (fg(MUT) + bg(BG),  '   running 4 tests'),
    ])
)

tests = [
    ('parse_empty',   24),
    ('parse_nested',  23),
    ('parse_error',   24),
    ('parse_unicode', 22),
]
for name, pad in tests:
    row(
        sidebar_cell([]),
        chat_cell([
            (fg(ORG) + bg(BG), ' ┃'),
            (fg(MUT) + bg(BG), f'   test {name}' + ' ' * pad),
            (fg(GRN) + bg(BG), '... ok'),
        ])
    )

row(sidebar_cell([]), chat_cell([(fg(ORG) + bg(BG), ' ┃')]))

row(
    sidebar_cell([]),
    chat_cell([
        (fg(ORG) + bg(BG), ' ┃'),
        (fg(MUT) + bg(BG), '   test result: '),
        (fg(GRN) + bg(BG), 'ok'),
        (fg(MUT) + bg(BG), '. 4 passed; 0 failed; 0 ignored'),
    ])
)

empty_row()

row(
    sidebar_cell([]),
    chat_cell([(fg(TEXT) + bg(BG), ' All 4 tests pass. The error handler is in place.')])
)
row(
    sidebar_cell([]),
    chat_cell([
        (fg(TEXT) + bg(BG), ' The module now returns '),
        (fg(ORG)  + bg(BG), '`ParseError::UnexpectedToken`'),
        (fg(TEXT) + bg(BG), ' on malformed input.'),
    ])
)

for _ in range(12):
    empty_row()

separator()

# INPUT ROW
sys.stdout.write(
    at(_row) +
    bg(BGS) + ' ' * SW + R +
    fg(MUT) + bg(BG) + '│' +
    fg(TEXT) + bg(BG) + '  > ' +
    fg(MUT)  + bg(BG) + '█' +
    bg(BG)   + ' ' * (CW - 5) + R
)
_row += 1

separator()

# STATUS BAR
status = [
    (fg(TEXT) + bg(BGH) + BOLD, ' claude-sonnet-4-6 '),
    (R + fg(MUT) + bg(BG),      '   12.4k / 200k tokens   '),
    (fg(MUT) + bg(BG),          '$0.08   '),
    (fg(GRN)  + bg(BG),         'Build'),
]
vis = sum(len(t) for _, t in status)
sys.stdout.write(at(_row) + ''.join(col + t for col, t in status))
sys.stdout.write(bg(BG) + ' ' * (TW - vis) + R)
_row += 1

sys.stdout.write(at(1))  # park cursor at row 1
sys.stdout.flush()
time.sleep(10)
