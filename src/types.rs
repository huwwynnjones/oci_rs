use libc::{c_void, c_int, c_ushort};
use oci_bindings::{OciDataType};
use oci_error::{OciError};
use byteorder::{ByteOrder, LittleEndian};

/// The types that support conversion from OCI to Rust types.
/// 
#[derive(Debug, PartialEq)]
pub enum SqlValue {
    /// Anything specified as `VARCHAR` or `VARCHAR2` will end up here.
    VarChar(String),
    /// All integers regardless of their stated size are represented with this variant. e.g.
    /// `SMALLINT` and `INTEGER` will both be held. 
    Integer(i64),
    /// All floating point types regardless of their size are represented with this variant. e.g.
    /// `REAL` and `FLOAT` will both be held.
    Float(f64),
}
impl SqlValue {
    /// Returns 
    pub fn value<T: FromSqlValue>(&self) -> Option<T> {
        T::from_sql_value(self)
    }

    pub fn as_oci_ptr(&mut self) -> *mut c_void {
        match *self {
            SqlValue::VarChar(ref s) => s.as_ptr() as *mut c_void,
            SqlValue::Integer(ref mut i) => (i as *mut i64) as *mut c_void,
            SqlValue::Float(ref mut f) => (f as *mut f64) as *mut c_void,
        }

    }

    pub fn size(&self) -> c_int {
        match *self {
            SqlValue::VarChar(ref s) => s.capacity() as c_int,
            SqlValue::Integer(..) | SqlValue::Float(..) => 8 as c_int,
        }
    }

    pub fn oci_data_type(&self) -> c_ushort {
        match *self {
            SqlValue::VarChar(..) => OciDataType::SqlChar.into(),
            SqlValue::Integer(..) => OciDataType::SqlInt.into(),
            SqlValue::Float(..) => OciDataType::SqlFloat.into(),
        }
    }

    pub fn create_from_raw(data: &[u8],
                           sql_type: &OciDataType)
                           -> Result<Self, OciError> {
        match *sql_type {
            OciDataType::SqlChar => {
                match String::from_utf8(Vec::from(data)) {
                    Ok(s) => Ok(SqlValue::VarChar(s.trim().to_string())),
                    Err(err) => Err(OciError::Conversion(Box::new(err))),
                }
            }
            OciDataType::SqlInt => {
                println!("data: {:?}, len {}", data, data.len());
                let i = LittleEndian::read_i64(data);
                Ok(SqlValue::Integer(i as i64))
            }
            OciDataType::SqlFloat => {
                println!("data: {:?}, len {}", data, data.len());
                let f = LittleEndian::read_f64(data);
                Ok(SqlValue::Float(f as f64))
            }
            _ => panic!("Not implemented yet"),
        }
    }
}

pub trait ToSqlValue {
    fn to_sql_value(&self) -> SqlValue;
}

impl ToSqlValue for String {
    fn to_sql_value(&self) -> SqlValue {
        let s = String::from(self.as_ref());
        SqlValue::VarChar(s)
    }
}

impl<'a> ToSqlValue for &'a str {
    fn to_sql_value(&self) -> SqlValue {
        let s = String::from(*self);
        SqlValue::VarChar(s)
    }
}

impl ToSqlValue for i64 {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::Integer(*self)
    }
}

impl ToSqlValue for f64 {
    fn to_sql_value(&self) -> SqlValue {
        SqlValue::Float(*self)
    }
}

pub trait FromSqlValue {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> where Self: Sized;
}

impl FromSqlValue for String {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::VarChar(ref s) => Some(String::from(s.to_string())),
            SqlValue::Integer(i) => Some(format!("{}", i)),
            SqlValue::Float(f) => Some(format!("{}", f)),
        }
    }
}

impl FromSqlValue for i64 {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::Integer(i) => Some(i),
            _ => None,
        }
    }
}

impl FromSqlValue for f64 {
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::Float(f) => Some(f),
            _ => None,
        }
    }
}
