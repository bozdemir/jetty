use crate::snapshot::{CellSnapshot, GridSnapshot};
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

#[derive(Clone, Copy)]
struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _event: Event) {}
}

#[derive(Clone, Copy)]
struct Size {
    cols: usize,
    lines: usize,
}
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.lines
    }
    fn screen_lines(&self) -> usize {
        self.lines
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

pub struct Terminal {
    term: Term<NoopListener>,
    parser: Processor,
    cols: usize,
    rows: usize,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Terminal {
        let size = Size { cols, lines: rows };
        let term = Term::new(Config::default(), &size, NoopListener);
        Terminal { term, parser: Processor::new(), cols, rows }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn snapshot(&self) -> GridSnapshot {
        use alacritty_terminal::index::{Column, Line, Point};
        let mut cells = vec![CellSnapshot::default(); self.cols * self.rows];
        let grid = self.term.grid();
        for row in 0..self.rows {
            for col in 0..self.cols {
                // NOTE: valid only while there is no scrollback (total_lines == screen_lines).
                // When scrollback is added, map visible row -> absolute line before indexing.
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];
                let fg = resolve_rgb(cell.fg);
                let bg = resolve_rgb(cell.bg);
                cells[row * self.cols + col] = CellSnapshot { c: cell.c, fg, bg };
            }
        }
        let cursor = self.term.grid().cursor.point;
        GridSnapshot {
            cols: self.cols,
            rows: self.rows,
            cells,
            cursor_row: cursor.line.0.max(0) as usize,
            cursor_col: cursor.column.0,
        }
    }
}

/// The 16 standard ANSI colors (xterm defaults), palette indices 0..=15.
const ANSI_16: [[u8; 3]; 16] = [
    [0, 0, 0],       // 0  black
    [205, 0, 0],     // 1  red
    [0, 205, 0],     // 2  green
    [205, 205, 0],   // 3  yellow
    [0, 0, 238],     // 4  blue
    [205, 0, 205],   // 5  magenta
    [0, 205, 205],   // 6  cyan
    [229, 229, 229], // 7  white
    [127, 127, 127], // 8  bright black
    [255, 0, 0],     // 9  bright red
    [0, 255, 0],     // 10 bright green
    [255, 255, 0],   // 11 bright yellow
    [92, 92, 255],   // 12 bright blue
    [255, 0, 255],   // 13 bright magenta
    [0, 255, 255],   // 14 bright cyan
    [255, 255, 255], // 15 bright white
];

/// Convert a 256-color palette index to RGB (standard xterm scheme):
/// 0..=15 standard, 16..=231 the 6x6x6 cube, 232..=255 the grayscale ramp.
fn index_to_rgb(i: u8) -> [u8; 3] {
    match i {
        0..=15 => ANSI_16[i as usize],
        16..=231 => {
            let c = i - 16;
            let levels = [0u8, 95, 135, 175, 215, 255];
            [
                levels[(c / 36) as usize],
                levels[((c % 36) / 6) as usize],
                levels[(c % 6) as usize],
            ]
        }
        232..=255 => {
            let v = 8 + (i - 232) * 10;
            [v, v, v]
        }
    }
}

/// Map an alacritty cell color to RGB: true-color is exact; named and indexed
/// colors resolve through the standard ANSI / xterm-256 palette.
fn resolve_rgb(color: alacritty_terminal::vte::ansi::Color) -> [u8; 3] {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Indexed(i) => index_to_rgb(i),
        Color::Named(n) => match n {
            NamedColor::Background => [18, 18, 23],
            NamedColor::Foreground | NamedColor::BrightForeground => [220, 220, 220],
            NamedColor::Black => index_to_rgb(0),
            NamedColor::Red => index_to_rgb(1),
            NamedColor::Green => index_to_rgb(2),
            NamedColor::Yellow => index_to_rgb(3),
            NamedColor::Blue => index_to_rgb(4),
            NamedColor::Magenta => index_to_rgb(5),
            NamedColor::Cyan => index_to_rgb(6),
            NamedColor::White => index_to_rgb(7),
            NamedColor::BrightBlack => index_to_rgb(8),
            NamedColor::BrightRed => index_to_rgb(9),
            NamedColor::BrightGreen => index_to_rgb(10),
            NamedColor::BrightYellow => index_to_rgb(11),
            NamedColor::BrightBlue => index_to_rgb(12),
            NamedColor::BrightMagenta => index_to_rgb(13),
            NamedColor::BrightCyan => index_to_rgb(14),
            NamedColor::BrightWhite => index_to_rgb(15),
            // Dim*/Cursor and any future variants: approximate with default fg.
            _ => [220, 220, 220],
        },
    }
}
