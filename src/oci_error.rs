use libc::{c_int, c_uchar, c_uint, c_void};
use oci_bindings::{HandleType, OCIErrorGet, ReturnCode};
use std::error;
use std::error::Error;
use std::fmt;
use std::ptr;

const MAX_ERROR_MESSAGE_SIZE: usize = 3024;

/// The various errors that might result when interacting with the OCI library.
///
#[derive(Debug)]
pub enum OciError {
    /// Contains the Oracle error details.
    /// Everything that comes back from the database will be retuned in this variant.
    Oracle(ErrorRecord),
    /// Picks up any errors that might come during conversion, such as a `Utf8Error`.
    /// It will not represent any Oracle errors.
    Conversion(Box<Error + Send + Sync>),
}

impl fmt::Display for OciError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            OciError::Oracle(ref err) => write!(f, "{}", err),
            OciError::Conversion(ref err) => write!(f, "{}", err),
        }
    }
}

impl error::Error for OciError {
    fn description(&self) -> &str {
        match *self {
            OciError::Oracle(_) => "Oracle error",
            OciError::Conversion(_) => "Cannot convert from OCI to Rust type",
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            OciError::Oracle(_) => None,
            OciError::Conversion(ref err) => Some(err.as_ref()),
        }
    }
}

/// Used to capture the errors details from OCI errors. Typically
/// these come as Oracle error codes and text such as
/// "ORA-24312: illegal parameters specified for allocating user memory"
#[derive(Debug)]
pub struct ErrorRecord {
    description: String,
    records: Vec<(i32, String)>,
}
impl ErrorRecord {
    /// Create a new ErrorRecord. The description is used to help show what action
    /// caused the error.
    fn new(description: &str) -> ErrorRecord {
        ErrorRecord {
            records: Vec::new(),
            description: description.to_string(),
        }
    }

    /// Get the error records
    pub fn error_records(&self) -> &[(i32, String)] {
        &self.records
    }

    /// Add a new error code and description to the ErrorRecord
    fn add_error(&mut self, code: i32, description: String) {
        self.records.push((code, description))
    }
}

impl fmt::Display for ErrorRecord {
    /// Collects all the errors and displays them one after another.
    /// It will show the description for the ErrorRecord itself
    /// followed by the Oracle error code and text for each error that
    /// was registered.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut text = String::new();
        text.push_str(&self.description);
        for (index, error) in self.records.iter().enumerate() {
            text.push_str(
                format!(
                    "\nError number: {}\nError code: ORA-{}\nError text: {}",
                    index + 1,
                    error.0,
                    &error.1
                ).as_ref(),
            )
        }
        write!(f, "{}", text)
    }
}

/// Fetches the error records registered against the handle provided. If it is called
/// out of sequence then the errors returned might be caused by a different function.
/// Often the caller will need to cast their handle to *mut `c_void` to make it work.
pub(crate) fn get_error(
    handle: *mut c_void,
    handle_type: HandleType,
    description: &str,
) -> OciError {
    let mut record_nmb: c_uint = 1;
    let sql_state: *mut c_uchar = ptr::null_mut();
    let mut error_record = ErrorRecord::new(description);

    loop {
        let mut error_code: c_int = 0;
        let mut error_message: [c_uchar; MAX_ERROR_MESSAGE_SIZE] = [0; MAX_ERROR_MESSAGE_SIZE];
        let error_message_ptr = error_message.as_mut_ptr();
        let error_result = unsafe {
            OCIErrorGet(
                handle,
                record_nmb,
                sql_state,
                &mut error_code,
                error_message_ptr,
                MAX_ERROR_MESSAGE_SIZE as c_uint,
                handle_type.into(),
            )
        };
        match error_result.into() {
            ReturnCode::NoData => break,
            ReturnCode::Success => {
                let first_null_byte_index = error_message.iter().position(|&x| x == 0).unwrap();
                let oracle_error_text =
                    String::from_utf8_lossy(&error_message[0..first_null_byte_index]).into_owned();

                error_record.add_error(error_code, oracle_error_text)
            }
            ReturnCode::Error => {
                error_record.add_error(error_code, "Call to OCIErrorGet failed".to_string())
            }
            ReturnCode::InvalidHandle => {
                error_record.add_error(error_code, "Invalid handle used to get errors".to_string())
            }
        }
        record_nmb += 1;
    }
    OciError::Oracle(error_record)
}
