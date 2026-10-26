//! Shared row/column rules for Home drawing, scrolling, focus and reordering.
#[derive(Clone, Copy)]
pub struct Layout {
    count: usize,
    pub columns: usize,
    pub rows: usize,
}

impl Layout {
    pub fn new(count: usize, two_rows: bool) -> Self {
        let columns = if two_rows {
            count.div_ceil(2).max(1)
        } else {
            count.max(1)
        };
        Self {
            count,
            columns,
            rows: count.div_ceil(columns),
        }
    }

    pub fn cell(self, index: usize) -> (usize, usize) {
        (index % self.columns, index / self.columns)
    }

    pub fn neighbor(self, index: usize, dx: isize, dy: isize) -> Option<usize> {
        if index >= self.count {
            return None;
        }
        let (column, row) = self.cell(index);
        let column = column.checked_add_signed(dx)?;
        let row = row.checked_add_signed(dy)?;
        if column >= self.columns || row >= self.rows {
            return None;
        }
        let next = row * self.columns + column;
        if dy != 0 {
            // An incomplete last row selects its nearest existing column.
            Some(next.min(self.count - 1))
        } else {
            (next < self.count).then_some(next)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_rows_preserve_row_major_order_and_reach_the_incomplete_row() {
        let layout = Layout::new(13, true);
        assert_eq!((layout.columns, layout.rows), (7, 2));
        assert_eq!(layout.cell(7), (0, 1));
        assert_eq!(layout.neighbor(6, 0, 1), Some(12));
        assert_eq!(layout.neighbor(12, 0, -1), Some(5));
        assert_eq!(layout.neighbor(6, 1, 0), None);
        assert_eq!(layout.neighbor(7, -1, 0), None);
        assert_eq!(layout.neighbor(12, 0, 1), None);
        let single = Layout::new(13, false);
        assert_eq!(single.cell(12), (12, 0));
        assert_eq!(single.neighbor(6, 1, 0), Some(7));
        assert_eq!(single.neighbor(6, 0, 1), None);
        assert_eq!(Layout::new(0, true).neighbor(0, 0, 1), None);
        let six = Layout::new(6, true);
        assert_eq!((six.columns, six.rows), (3, 2));
    }
}
