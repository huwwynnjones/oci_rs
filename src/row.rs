use std::ops::Index;
use types::SqlValue;

#[derive(Debug)]
pub struct Row {
    columns: Vec<SqlValue>,
}
impl Row {
    pub fn new(columns: Vec<SqlValue>) -> Row {
        Row { columns: columns }
    }
}
impl Index<usize> for Row {
    type Output = SqlValue;

    fn index(&self, index: usize) -> &SqlValue {
        &self.columns[index]
    }
}
