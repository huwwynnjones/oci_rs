use libc::{c_void, c_int, c_ushort};
use oci_bindings::SqlType;

#[derive(Debug)]
pub enum SqlValue {
    SqlString(String),
    SqlInteger(i64),
}
impl SqlValue {
    pub fn as_oci_ptr(&mut self) -> *mut c_void {
        match *self {
            SqlValue::SqlString(ref s) => s.as_ptr() as *mut c_void,
            SqlValue::SqlInteger(ref mut i) => (i as *mut i64) as *mut c_void,
        }

    }

    pub fn size(&self) -> c_int {
        match *self {
            SqlValue::SqlString(ref s) => s.capacity() as c_int,
            SqlValue::SqlInteger(..) => 64 / 8 as c_int,
        }
    }

    pub fn oci_data_type(&self) -> c_ushort {
        match *self {
            SqlValue::SqlString(..) => SqlType::SqlChar.into(),
            SqlValue::SqlInteger(..) => SqlType::SqlInt.into(),
        }
    }
}

pub trait ToSqlValue {
    fn to_sql_value(&self) -> SqlValue;
}

impl ToSqlValue for String {
    fn to_sql_value(&self) -> SqlValue {
        let s = String::from(self.as_ref());
        SqlValue::SqlString(s)
    }
}

impl ToSqlValue for i64 {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::SqlInteger(*self)
    }
}
