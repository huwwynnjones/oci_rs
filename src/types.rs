use libc::{c_void, c_int, c_ushort};
use oci_bindings::SqlDataType;
use oci_error::OciError;
use byteorder::{BigEndian, ReadBytesExt, ByteOrder, LittleEndian};
use std::ffi::CString;

#[derive(Debug)]
pub enum SqlValue {
    SqlString(String),
    SqlInteger(i64),
}
impl SqlValue {
    pub fn value<T: FromSqlValue>(&self) -> Option<T> {
        T::from_sql_value(self)
    }

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
            SqlValue::SqlString(..) => SqlDataType::SqlChar.into(),
            SqlValue::SqlInteger(..) => SqlDataType::SqlInt.into(),
        }
    }

    pub fn create_from_raw(data: &Vec<u8>, sql_type: &SqlDataType) -> Result<Self, OciError> {
        match *sql_type {
            SqlDataType::SqlChar => {
                match String::from_utf8(Vec::from(&data[..])) {
                    Ok(s) => Ok(SqlValue::SqlString(s.trim().to_string())),
                    Err(err) => Err(OciError::Conversion(err)),
                }
            },
            SqlDataType::SqlInt | SqlDataType::SqlNum => {
                let i = LittleEndian::read_i64(&data); 
                Ok(SqlValue::SqlInteger(i))
            }
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

pub trait FromSqlValue {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> where Self: Sized;
}

impl FromSqlValue for String {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::SqlString(ref s) => Some(String::from(s.to_string())),
            SqlValue::SqlInteger(i) => Some(format!("{}", i)),
        }
    }
}

impl FromSqlValue for i64 {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value{
            SqlValue::SqlInteger(i) => Some(i),
            _ => None,
        }
    }
}
