use std::ops::Index;
use types::SqlValue;

/// Represents a row of data returned from a SQL query.
///
#[derive(Debug)]
pub struct Row {
    columns: Vec<SqlValue>,
}
impl Row {
    pub(crate) fn new(// crate) fn new(
                      columns: Vec<SqlValue>)
                      -> Row {
        Row { columns: columns }
    }

    /// Returns the columns in the row.
    ///
    pub fn columns(&self) -> &Vec<SqlValue> {
        &self.columns
    }
}
impl Index<usize> for Row {
    type Output = SqlValue;

    fn index(&self, index: usize) -> &SqlValue {
        &self.columns[index]
    }
}
