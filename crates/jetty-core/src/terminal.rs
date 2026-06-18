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

/// Map an alacritty cell color to RGB. Named/indexed colors get sane fallbacks;
/// true-color (Spec) is used directly. Full palette mapping arrives with theming.
fn resolve_rgb(color: alacritty_terminal::vte::ansi::Color) -> [u8; 3] {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Named(NamedColor::Background) => [18, 18, 23],
        Color::Named(NamedColor::Foreground) => [220, 220, 220],
        Color::Named(_) => [220, 220, 220],
        Color::Indexed(_) => [220, 220, 220],
    }
}
