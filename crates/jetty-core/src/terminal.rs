use crate::snapshot::{CellSnapshot, GridSnapshot};
use crate::theme::Theme;
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
    theme: Theme,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Terminal {
        let size = Size { cols, lines: rows };
        let term = Term::new(Config::default(), &size, NoopListener);

        // Load theme from JETTY_THEME env var; default to "default_dark".
        let theme_name = std::env::var("JETTY_THEME").unwrap_or_else(|_| "default_dark".to_string());
        let mut theme = Theme::by_name(&theme_name);

        // Apply opacity override from JETTY_OPACITY (float 0.0..1.0).
        // This multiplies into the theme bg alpha, enabling composited transparency.
        if let Ok(op_str) = std::env::var("JETTY_OPACITY") {
            if let Ok(opacity) = op_str.parse::<f32>() {
                let opacity = opacity.clamp(0.0, 1.0);
                theme.bg[3] = (opacity * 255.0) as u8;
            }
        }

        Terminal { term, parser: Processor::new(), cols, rows, theme }
    }

    /// Replace the active theme at runtime.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Return a reference to the active theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
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
                let fg = resolve_rgb(&self.theme, cell.fg);
                let bg = resolve_rgb(&self.theme, cell.bg);
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
            bg_rgba: self.theme.bg,
            cursor_rgb: self.theme.cursor,
        }
    }
}

/// Convert a 256-color palette index to RGB (standard xterm scheme):
/// 0..=15 from the theme palette, 16..=231 the 6x6x6 cube, 232..=255 the grayscale ramp.
fn index_to_rgb(theme: &Theme, i: u8) -> [u8; 3] {
    match i {
        0..=15 => theme.palette[i as usize],
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

/// Map an alacritty cell color to RGB using the active theme.
/// True-color is exact; named and indexed colors resolve through the theme palette.
fn resolve_rgb(theme: &Theme, color: alacritty_terminal::vte::ansi::Color) -> [u8; 3] {
    use alacritty_terminal::vte::ansi::{Color, NamedColor};
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Indexed(i) => index_to_rgb(theme, i),
        Color::Named(n) => match n {
            NamedColor::Background => [theme.bg[0], theme.bg[1], theme.bg[2]],
            NamedColor::Foreground | NamedColor::BrightForeground => theme.fg,
            NamedColor::Black => index_to_rgb(theme, 0),
            NamedColor::Red => index_to_rgb(theme, 1),
            NamedColor::Green => index_to_rgb(theme, 2),
            NamedColor::Yellow => index_to_rgb(theme, 3),
            NamedColor::Blue => index_to_rgb(theme, 4),
            NamedColor::Magenta => index_to_rgb(theme, 5),
            NamedColor::Cyan => index_to_rgb(theme, 6),
            NamedColor::White => index_to_rgb(theme, 7),
            NamedColor::BrightBlack => index_to_rgb(theme, 8),
            NamedColor::BrightRed => index_to_rgb(theme, 9),
            NamedColor::BrightGreen => index_to_rgb(theme, 10),
            NamedColor::BrightYellow => index_to_rgb(theme, 11),
            NamedColor::BrightBlue => index_to_rgb(theme, 12),
            NamedColor::BrightMagenta => index_to_rgb(theme, 13),
            NamedColor::BrightCyan => index_to_rgb(theme, 14),
            NamedColor::BrightWhite => index_to_rgb(theme, 15),
            // Dim*/Cursor and any future variants: approximate with default fg.
            _ => theme.fg,
        },
    }
}
