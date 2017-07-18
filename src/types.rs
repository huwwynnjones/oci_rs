use libc::{c_void, c_int};
use oci_bindings::OciDataType;
use oci_error::OciError;
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
    /// Represents null values in columns.
    Null,
}
impl SqlValue {
    /// Returns the internal value converting on the way to whichever type implements
    /// `FromSqlValue`.
    ///
    /// It returns an `Option` because conversion might not be possible.
    /// For example converting an `SqlValue::Integer` to a `String` works just fine, but converting
    /// an `SqlValue::Null` to an i64 does not make sense.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oci_rs::types::{SqlValue, ToSqlValue};
    ///
    /// let v = SqlValue::Integer(42);
    /// let i: i64 = v.value().expect("Won't covert to an i64");
    /// let s: String = v.value().expect("Won't convert to a String");
    ///
    /// assert_eq!(i, 42);
    /// assert_eq!(s, "42");
    /// ```
    ///
    pub fn value<T: FromSqlValue>(&self) -> Option<T> {
        T::from_sql_value(self)
    }

    /// Returns a pointer to the internal value that can be used by OCI.
    ///
    pub(crate) fn as_oci_ptr(&mut self) -> *mut c_void {
        match *self {
            SqlValue::VarChar(ref s) => s.as_ptr() as *mut c_void,
            SqlValue::Integer(ref mut i) => (i as *mut i64) as *mut c_void,
            SqlValue::Float(ref mut f) => (f as *mut f64) as *mut c_void,
            SqlValue::Null => panic!("Null not handled"),
        }

    }

    /// Gives the size in bytes of the internal value.
    ///
    /// It is used by the OCI library to allocate storage.
    ///
    pub(crate) fn size(&self) -> c_int {
        match *self {
            SqlValue::VarChar(ref s) => s.capacity() as c_int,
            SqlValue::Integer(..) |
            SqlValue::Float(..) => 8 as c_int,
            SqlValue::Null => panic!("Null not handled"),
        }
    }

    /// Converts to the relevant OCI internal type.
    ///
    pub(crate) fn as_oci_data_type(&self) -> OciDataType {
        match *self {
            SqlValue::VarChar(..) => OciDataType::SqlChar,
            SqlValue::Integer(..) => OciDataType::SqlInt,
            SqlValue::Float(..) => OciDataType::SqlFloat,
            SqlValue::Null => panic!("Null not handled"),
        }
    }

    /// Create an `SqlValue` from a slice of bytes and indication of the data type
    ///
    pub(crate) fn create_from_raw(data: &[u8],
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
                let i = LittleEndian::read_i64(data);
                Ok(SqlValue::Integer(i as i64))
            }
            OciDataType::SqlFloat => {
                let f = LittleEndian::read_f64(data);
                Ok(SqlValue::Float(f as f64))
            }
            _ => panic!("Not implemented yet"),
        }
    }
}

/// Allows conversion into a `SqlValue`.
/// 
pub trait ToSqlValue {
    /// Converts into a `SqlValue`.
    /// 
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

/// Allows conversion from a `SqlValue`.
/// 
pub trait FromSqlValue {
    /// Converts from a `SqlValue`.
    /// 
    /// It allows for impossible conversions though the use of `Option`.
    /// e.g. an `SqlValue::Null` cannot be converted into a i64.
    ///
    /// When the `TryFrom` trait becomes stable then this crate will probably switch to that
    /// instead.
    ///
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> where Self: Sized;
}

impl FromSqlValue for String {
    // Converts from a `SqlValue` into a `String`
    // 
    // Worth noting that this is intend to convert all types into a 
    // `String` representation of the value. It also does this for 
    // `SqlValue::Null` for which is returns "null". That might prove a bad idea.
    // 
    fn from_sql_value(sql_value: &SqlValue) -> Option<Self> {
        match *sql_value {
            SqlValue::VarChar(ref s) => Some(s.to_string()),
            SqlValue::Integer(i) => Some(format!("{}", i)),
            SqlValue::Float(f) => Some(format!("{}", f)),
            SqlValue::Null => Some("null".to_string()),
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
