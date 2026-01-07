//! Table serialization logic.

use comrak::nodes::{AstNode, NodeTable, TableAlignment};

use super::Serializer;
use super::escape;

impl<'a> Serializer<'a> {
    pub(super) fn serialize_table<'b>(&mut self, node: &'b AstNode<'b>, table: &NodeTable) {
        let alignments = &table.alignments;
        // Collect all rows and cells first to calculate column widths
        let rows: Vec<_> = node.children().collect();
        if rows.is_empty() {
            return;
        }

        // Collect cell contents (with full inline formatting) and calculate max widths
        let mut all_cells: Vec<Vec<String>> = Vec::new();
        let mut col_widths: Vec<usize> = vec![0; alignments.len()];

        for row in &rows {
            let mut row_cells: Vec<String> = Vec::new();
            for (i, cell) in row.children().enumerate() {
                // Use collect_inline_content to preserve links and formatting
                let mut content = String::new();
                self.collect_inline_content(cell, &mut content);
                // Escape pipe characters in table cells to prevent cell boundary confusion
                let content = escape::escape_table_cell(&content);
                if i < col_widths.len() {
                    col_widths[i] = col_widths[i].max(content.len());
                }
                row_cells.push(content);
            }
            all_cells.push(row_cells);
        }

        // Ensure minimum column width for alignment markers
        for width in &mut col_widths {
            *width = (*width).max(3);
        }

        // Output header row
        if let Some(header_cells) = all_cells.first() {
            if self.in_block_quote {
                self.output.push_str("> ");
            }
            self.output.push('|');
            for (i, cell) in header_cells.iter().enumerate() {
                self.output.push(' ');
                let width = col_widths.get(i).copied().unwrap_or(3);
                self.output
                    .push_str(&format!("{:width$}", cell, width = width));
                self.output.push_str(" |");
            }
            self.output.push('\n');
        }

        // Output separator row with alignment
        if self.in_block_quote {
            self.output.push_str("> ");
        }
        self.output.push('|');
        for (i, alignment) in alignments.iter().enumerate() {
            self.output.push(' ');
            let width = col_widths.get(i).copied().unwrap_or(3);
            match alignment {
                TableAlignment::Left => {
                    self.output.push(':');
                    self.output.push_str(&"-".repeat(width - 1));
                }
                TableAlignment::Right => {
                    self.output.push_str(&"-".repeat(width - 1));
                    self.output.push(':');
                }
                TableAlignment::Center => {
                    self.output.push(':');
                    self.output.push_str(&"-".repeat(width - 2));
                    self.output.push(':');
                }
                TableAlignment::None => {
                    self.output.push_str(&"-".repeat(width));
                }
            }
            self.output.push_str(" |");
        }
        self.output.push('\n');

        // Output data rows (skip header)
        for row_cells in all_cells.iter().skip(1) {
            if self.in_block_quote {
                self.output.push_str("> ");
            }
            self.output.push('|');
            for (i, cell) in row_cells.iter().enumerate() {
                self.output.push(' ');
                let width = col_widths.get(i).copied().unwrap_or(3);
                self.output
                    .push_str(&format!("{:width$}", cell, width = width));
                self.output.push_str(" |");
            }
            self.output.push('\n');
        }
    }

    pub(super) fn serialize_table_row<'b>(&mut self, _node: &'b AstNode<'b>, _is_header: bool) {
        // Table rows are handled by serialize_table
    }
}
