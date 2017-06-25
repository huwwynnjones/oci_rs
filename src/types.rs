use libc::{c_void, c_int};

#[derive(Debug)]
pub enum SqlValue {
    SqlString{value: String, ptr: *const c_void},
    SqlInteger{value: i64, ptr: *const c_void},
}
impl SqlValue {
    pub fn as_ptr(&self) -> *mut c_void {
        match *self {
            SqlValue::SqlString{value, ..} => value.as_ptr() as *mut c_void,
            SqlValue::SqlInteger{value, ..} => value as *mut c_void,
        }
    }

    pub fn size(&self) -> c_int {
        match *self {
            SqlValue::SqlString{value, ..} => value.capacity() as c_int,
            SqlValue::SqlInteger{..} => 64 / 8 as c_int, 
        }
    }
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
