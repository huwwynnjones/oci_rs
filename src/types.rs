use libc::{c_void, c_int, c_ushort, c_uint};
use oci_bindings::{SqlDataType, OCIError, ReturnCode, OCINumber, OCINumberIsInt, OCINumberToInt,
                   OciNumberType, HandleType};
use oci_error::{OciError, get_error};
use byteorder::{ByteOrder, LittleEndian};

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

    pub fn create_from_raw(data: &Vec<u8>,
                           sql_type: &SqlDataType,
                           error: *mut OCIError)
                           -> Result<Self, OciError> {
        match *sql_type {
            SqlDataType::SqlChar => {
                match String::from_utf8(Vec::from(&data[..])) {
                    Ok(s) => Ok(SqlValue::SqlString(s.trim().to_string())),
                    Err(err) => Err(OciError::Conversion(err)),
                }
            }
            SqlDataType::SqlNum => {
                println!("data: {:?}, len {}", data, data.len());
                //create an oci number
                if raw_is_int(data, error)? {
                    let i = raw_as_int(data, error)?;
                    Ok(SqlValue::SqlInteger(i as i64))
                } else {
                    Ok(SqlValue::SqlInteger(0))
                }
            }
            SqlDataType::SqlInt => Ok(SqlValue::SqlInteger(0)),
        }
    }
}

fn raw_is_int(data: &Vec<u8>, error: *mut OCIError) -> Result<bool, OciError> {
    
    //need to create an oci_number
    let oci_number = l_data.as_ptr() as *const OCINumber;
    let mut result: bool = false;
    let result_ptr: *mut bool = &mut result;
    let is_int_result = unsafe { OCINumberIsInt(error, oci_number, result_ptr) };
    match is_int_result.into() {
        ReturnCode::Success => Ok(result),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Checking OCINumber is int"))
        }
    }
}

fn raw_as_int(data: &Vec<u8>, error: *mut OCIError) -> Result<c_uint, OciError> {

    let oci_number = data.as_ptr() as *const OCINumber;
    let rsl_length = 4 as c_uint;
    let mut result: c_uint = 0;
    let result_ptr: *mut c_uint = &mut result;
    let to_int_result = unsafe {
        OCINumberToInt(error,
                       oci_number,
                       rsl_length,
                       OciNumberType::Signed.into(),
                       result_ptr as *mut c_void)
    };
    match to_int_result.into() {
        ReturnCode::Success => Ok(result),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Converting OCINumber to int"))
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
        match *sql_value {
            SqlValue::SqlInteger(i) => Some(i),
            _ => None,
        }
    }
}
