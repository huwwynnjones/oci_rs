use oci_bindings::{OCIEnv, OCIEnvCreate, OCI_DEFAULT, OCI_SUCCESS, ErrorHandleType, OCIHandleFree,
                   OCIServer, OCIHandleAlloc, OCI_HTYPE_SERVER, ReturnCode, ToReturnCode};
use oci_error::{OciError, get_error};
use std::ptr;
use libc::{c_int, c_void, c_uint, size_t};

/// Represents a connection to a database. Internally
/// it holds the environment
#[derive(Debug)]
pub struct Connection {
    environment: *mut OCIEnv,
    server: *mut OCIServer,
}
impl Connection {
    /// Creates a new Connection.
    /// # Errors
    /// Any errors encounter when trying to allocate
    pub fn new() -> Result<Connection, OciError> {
        let env = create_environment_handle()?;
        let server = create_server_handle(env)?;
        Ok(Connection {
            environment: env,
            server: server,
        })
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

fn create_server_handle(env: *const OCIEnv) -> Result<*mut OCIServer, OciError> {

    let mut server: *mut OCIServer = ptr::null_mut();
    let mut usrmempp: *mut c_void = ptr::null_mut();
    let server_result = unsafe {
        OCIHandleAlloc(env as *const c_void,
                       &mut server,
                       OCI_HTYPE_SERVER,
                       0 as size_t,
                       &usrmempp)
    };
    match server_result.to_return_code() {
        ReturnCode::Success => Ok(server),
        _ => Err(get_error(env as *mut c_void, ErrorHandleType::Environment)),
    }
}
