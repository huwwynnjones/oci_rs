use std::ops::Index;
use types::SqlValue;

#[derive(Debug)]
pub struct Row {
    pub columns: Vec<SqlValue>,
}
impl Row {
    pub fn create_row(the_columns: Vec<SqlValue>) -> Row {
        Row { columns: the_columns }
    }
}
impl Index<usize> for Row {
    type Output = SqlValue;

    fn index(&self, index: usize) -> &SqlValue {
        &self.columns[index]
    }
}
