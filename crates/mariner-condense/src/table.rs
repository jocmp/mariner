//! Places table cells on a grid, honoring `colspan`, `rowspan` and `dir`.

use crate::model::{Block, CellKind, Ids, TableBlock, TableCell, TableSlot};

const MAX_COL_SPAN: usize = 1_000;
const MAX_ROW_SPAN: usize = 65_534;

/// What a `<table>` turns into.
pub(crate) enum TableOutput {
    Table(TableBlock),
    /// A table with one row or one column: its cells' blocks, in order.
    Flat(Vec<Block>),
}

/// Collects cells row by row, then lays them out.
pub(crate) struct GridBuilder {
    placed: Vec<Placed>,
    row: Option<usize>,
    col: usize,
    cols: usize,
}

struct Placed {
    row: usize,
    col: usize,
    row_span: usize,
    col_span: usize,
    kind: CellKind,
    blocks: Vec<Block>,
}

impl GridBuilder {
    pub(crate) fn new() -> Self {
        Self {
            placed: Vec::new(),
            row: None,
            col: 0,
            cols: 0,
        }
    }

    pub(crate) fn start_row(&mut self) {
        self.row = Some(self.row.map_or(0, |row| row + 1));
        self.col = 0;
    }

    /// Adds a cell at the next free column of the current row. A `rowspan`
    /// of 0 spans to the last row.
    pub(crate) fn add(
        &mut self,
        kind: CellKind,
        row_span: usize,
        col_span: usize,
        blocks: Vec<Block>,
    ) {
        let Some(row) = self.row else {
            return;
        };
        let row_span = if row_span == 0 {
            MAX_ROW_SPAN
        } else {
            row_span.min(MAX_ROW_SPAN)
        };
        let col_span = col_span.clamp(1, MAX_COL_SPAN);
        while self.is_occupied(row, self.col) {
            self.col += 1;
        }
        self.placed.push(Placed {
            row,
            col: self.col,
            row_span,
            col_span,
            kind,
            blocks,
        });
        self.col += col_span;
        self.cols = self.cols.max(self.col);
    }

    /// Lays the cells out, mirroring columns when `rtl`.
    pub(crate) fn finish(self, ids: Ids, rtl: bool) -> TableOutput {
        let rows = self.row.map_or(0, |row| row + 1);
        let cols = self.cols;
        if rows <= 1 || cols <= 1 {
            let blocks = self
                .placed
                .into_iter()
                .flat_map(|cell| cell.blocks)
                .collect();
            return TableOutput::Flat(blocks);
        }

        let mut slots = vec![TableSlot::Empty; rows * cols];
        for cell in self.placed {
            let row_span = cell.row_span.min(rows - cell.row);
            let col_span = cell.col_span.min(cols - cell.col);
            let col = if rtl {
                cols - cell.col - col_span
            } else {
                cell.col
            };
            for r in cell.row..cell.row + row_span {
                for c in col..col + col_span {
                    let slot = &mut slots[r * cols + c];
                    if matches!(slot, TableSlot::Empty) && (r, c) != (cell.row, col) {
                        *slot = TableSlot::Covered { row: cell.row, col };
                    }
                }
            }
            let corner = &mut slots[cell.row * cols + col];
            if !matches!(corner, TableSlot::Cell(_)) {
                *corner = TableSlot::Cell(TableCell {
                    kind: cell.kind,
                    row_span,
                    col_span,
                    blocks: cell.blocks,
                });
            }
        }
        TableOutput::Table(TableBlock {
            ids,
            rows,
            cols,
            slots,
        })
    }

    fn is_occupied(&self, row: usize, col: usize) -> bool {
        self.placed.iter().any(|cell| {
            (cell.row..cell.row + cell.row_span).contains(&row)
                && (cell.col..cell.col + cell.col_span).contains(&col)
        })
    }
}
