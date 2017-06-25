
#[derive(Debug)]
pub enum SqlValue {
    SqlString{value: String, ptr: *const c_void},
    SqlInteger{value: i64, ptr: *const c_void},
}

pub trait ToSqlValue {
    fn to_sql_value(&self) -> SqlValue;
}

impl ToSqlValue for String {
    fn to_sql_value(&self) -> SqlValue {
        let s = String::from(self.as_ref()); 
        SqlValue::SqlString{value: s, ptr: s.as_ptr()}
    }
}
