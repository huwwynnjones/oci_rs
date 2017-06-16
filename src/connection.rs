use oci_bindings::{OCIEnv, OCIEnvCreate, HandleType, OCIHandleFree, OCIServer, OCIHandleAlloc,
                   ReturnCode, EnvironmentMode, OCIError, OCISvcCtx, OCIServerAttach,
                   OCIServerDetach};
use oci_error::{OciError, get_error};
use std::ptr;
use libc::{c_void, size_t, c_int};

/// Represents a connection to a database. Internally
/// it holds the various handles that are needed to maintain
/// a connection to the database.
/// Once it goes out of scope it will free the handles via the
/// relevant OCI calls via a Drop implementation.
#[derive(Debug)]
pub struct Connection {
    environment: *mut OCIEnv,
    server: *mut OCIServer,
    error: *mut OCIError,
    service: *mut OCISvcCtx,
}
impl Connection {
    /// Creates a new Connection.
    ///
    /// # Errors
    ///
    /// Any errors encounter when trying to allocate handles
    /// in OCI library will bubble up here. The OciError will
    /// return the relevant Oracle error codes and text when
    /// available.
    ///
    /// # Examples
    ///
    pub fn new(connection_str: &str) -> Result<Connection, OciError> {
        let env = create_environment_handle()?;
        let server = create_server_handle(env)?;
        let error = create_error_handle(env)?;
        let service = create_service_handle(env)?;
        if let Some(err) = connect_to_database(server, error, connection_str) {
            return Err(err);
        }
        Ok(Connection {
            environment: env,
            server: server,
            error: error,
            service: service,
        })
    }
}

impl Drop for Connection {
    /// Frees the handles allocated by the OCI library.
    /// 
    /// # Panics
    /// 
    /// Panics if the resources can't be freed. This would be 
    /// a failure of the underlying OCIHandleFree function.
    fn drop(&mut self) {
        let disconnect_result =
            unsafe { OCIServerDetach(self.server, self.error, EnvironmentMode::Default.into()) };

        match disconnect_result.into() {
            ReturnCode::Success => (),
            _ => println!("Could not disconnect"), //log instead in future
        }

        let free_result = unsafe {
            OCIHandleFree(self.environment as *mut c_void,
                          HandleType::Environment.into())
        };

        match free_result.into() {
            ReturnCode::Success => (),
            _ => panic!("Could not free the handles in Connection"),
        }
    }
}

/// Creates an environment handle
fn create_environment_handle() -> Result<*mut OCIEnv, OciError> {
    let mut env: *mut OCIEnv = ptr::null_mut();
    let mode = EnvironmentMode::Default.into();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let env_result = unsafe {
        OCIEnvCreate(&mut env,
                     mode,
                     null_ptr,
                     null_ptr,
                     null_ptr,
                     null_ptr,
                     xtramem_sz,
                     null_ptr)
    };
    match env_result.into() {
        ReturnCode::Success => Ok(env),
        _ => {
            Err(get_error(env as *mut c_void,
                          HandleType::Environment,
                          "Environment handle creation"))
        }
    }
}

/// Creates a server handle
fn create_server_handle(env: *const OCIEnv) -> Result<*mut OCIServer, OciError> {
    let mut server: *mut c_void = ptr::null_mut();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let server_result = unsafe {
        OCIHandleAlloc(env as *const c_void,
                       &mut server,
                       HandleType::Server.into(),
                       xtramem_sz,
                       null_ptr)
    };
    match server_result.into() {
        ReturnCode::Success => Ok(server as *mut OCIServer),
        _ => {
            Err(get_error(env as *mut c_void,
                          HandleType::Environment,
                          "Server handle creation"))
        }
    }
}

/// Creates a error handle
fn create_error_handle(env: *const OCIEnv) -> Result<*mut OCIError, OciError> {
    let mut error: *mut c_void = ptr::null_mut();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let error_result = unsafe {
        OCIHandleAlloc(env as *const c_void,
                       &mut error,
                       HandleType::Error.into(),
                       xtramem_sz,
                       null_ptr)
    };
    match error_result.into() {
        ReturnCode::Success => Ok(error as *mut OCIError),
        _ => {
            Err(get_error(env as *mut c_void,
                          HandleType::Environment,
                          "Error handle creation"))
        }
    }
}

/// Creates a service handle
fn create_service_handle(env: *const OCIEnv) -> Result<*mut OCISvcCtx, OciError> {
    let mut service: *mut c_void = ptr::null_mut();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let service_result = unsafe {
        OCIHandleAlloc(env as *const c_void,
                       &mut service,
                       HandleType::Error.into(),
                       xtramem_sz,
                       null_ptr)
    };
    match service_result.into() {
        ReturnCode::Success => Ok(service as *mut OCISvcCtx),
        _ => {
            Err(get_error(env as *mut c_void,
                          HandleType::Environment,
                          "Service handle creation"))
        }
    }
}

/// Connect to the database
fn connect_to_database(server: *mut OCIServer,
                       error: *mut OCIError,
                       connection_str: &str)
                       -> Option<OciError> {

    let conn_bytes = connection_str.as_bytes();
    let conn_bytes_ptr = conn_bytes.as_ptr();
    let conn_len = connection_str.len() as c_int;

    let connect_result = unsafe {
        OCIServerAttach(server,
                        error,
                        conn_bytes_ptr,
                        conn_len,
                        EnvironmentMode::Default.into())
    };

    match connect_result.into() {
        ReturnCode::Success => None,
        _ => {
            Some(get_error(error as *mut c_void,
                           HandleType::Environment,
                           "Database connection"))
        }
    }
}
