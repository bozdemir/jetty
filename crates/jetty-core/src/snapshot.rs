#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CellSnapshot {
    pub c: char,
    pub fg: [u8; 3],
    pub bg: [u8; 3],
}

impl Default for CellSnapshot {
    fn default() -> Self {
        CellSnapshot { c: ' ', fg: [220, 220, 220], bg: [18, 18, 23] }
    }
}

#[derive(Clone, Debug)]
pub struct GridSnapshot {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<CellSnapshot>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl GridSnapshot {
    pub fn cell(&self, row: usize, col: usize) -> &CellSnapshot {
        &self.cells[row * self.cols + col]
    }
    pub fn row_text(&self, row: usize) -> String {
        (0..self.cols).map(|c| self.cell(row, c).c).collect::<String>()
    }
}
