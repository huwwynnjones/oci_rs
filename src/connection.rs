use oci_bindings::{OCIEnv, OCIEnvCreate, OCI_DEFAULT, OCIErrorGet, OCI_SUCCESS, OCI_HTYPE_ENV,
                   ErrorHandleType, OCIHandleFree};
use oci_error::{OciError, get_error};
use std::ptr;
use libc::{size_t, c_int, c_void, c_uint, c_uchar};
use std::ffi::CString;

/// Represents a connection to a database. Internally
/// it holds the environment
#[derive(Debug)]
pub struct Connection {
    environment: *mut OCIEnv,
}
impl Connection {
    /// Creates a new Connection.
    /// # Errors
    /// Any errors encounter when trying to allocate
    pub fn new() -> Result<Connection, OciError> {
        match create_environment_handle() {
            Ok(env) => Ok(Connection { environment: env }),
            Err(err) => Err(err),
        }
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        let free_result = unsafe {
            OCIHandleFree(self.environment as *mut c_void,
                          ErrorHandleType::Environment as c_uint)
        };
        if free_result != OCI_SUCCESS {
            panic!("Could not free the handles in Connection")
        }
    }
}

/// Creates the environment handle
fn create_environment_handle() -> Result<*mut OCIEnv, OciError> {

    let mut env: *mut OCIEnv = ptr::null_mut();
    let mode = OCI_DEFAULT;
    let null_ptr = ptr::null();
    let error_code: c_int = unsafe {
        OCIEnvCreate(&mut env,
                     mode,
                     null_ptr,
                     null_ptr,
                     null_ptr,
                     null_ptr,
                     null_ptr,
                     null_ptr)
    };
    if error_code == OCI_SUCCESS {
        Ok(env)
    } else {
        Err(get_error(env as *mut c_void, ErrorHandleType::Environment))
    }
}
