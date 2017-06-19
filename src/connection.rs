use oci_bindings::{OCIEnv, OCIEnvCreate, HandleType, OCIHandleFree, OCIServer, OCIHandleAlloc,
                   ReturnCode, EnvironmentMode, OCIError, OCISvcCtx, OCIServerAttach,
                   OCIServerDetach, AttributeType, OCIAttrSet, OCISession, OCISessionBegin,
                   CredentialsType, OCISessionEnd};
use oci_error::{OciError, get_error};
use std::ptr;
use libc::{c_void, size_t, c_int, c_uint};

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
    user_session: *mut OCISession,
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
    ///```rust,no_run
    /// use oci_rs::connection::Connection;
    /// 
    /// let connection = Connection::new("localhost:1521/xe",
    ///                                  "user",
    ///                                  "password")
    ///                                  .expect("Something went wrong"); 
    ///```
    pub fn new(connection_str: &str,
               user_name: &str,
               password: &str)
               -> Result<Connection, OciError> {
        let env = create_environment_handle()?;
        let server = create_server_handle(env)?;
        let error = create_error_handle(env)?;
        let service = create_service_handle(env, server, error)?;
        let session = create_user_session_handle(env)?;
        if let Some(err) = connect_to_database(server, error, connection_str) {
            return Err(err);
        }
        if let Some(err) = set_user_name(session, user_name, error) {
            return Err(err);
        }
        if let Some(err) = set_password(session, password, error) {
            return Err(err);
        }
        if let Some(err) = start_user_session(service, error, session) {
            return Err(err);
        }
        Ok(Connection {
            environment: env,
            server: server,
            error: error,
            service: service,
            user_session: session,
        })
    }
}

impl Drop for Connection {
    /// Ends the current user session, disconnects from the
    /// database and frees the handles allocated by the OCI library.
    /// This should ensure there are no remaining processes or memory
    /// allocated.
    /// 
    /// # Panics
    ///
    /// Panics if the resources can't be freed. This would be
    /// a failure of the underlying OCIHandleFree function.
    fn drop(&mut self) {
        let session_end_result = unsafe {
            OCISessionEnd(self.service,
                          self.error,
                          self.user_session,
                          EnvironmentMode::Default.into())
        };

        match session_end_result.into() {
            ReturnCode::Success => (),
            _ => println!("Could not end user session"), //log instead in future
        }

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
    match allocate_handle(env, HandleType::Server) {
        Ok(server) => Ok(server as *mut OCIServer),
        Err(err) => Err(err),
    }
}

/// Creates a error handle
fn create_error_handle(env: *const OCIEnv) -> Result<*mut OCIError, OciError> {
    match allocate_handle(env, HandleType::Error) {
        Ok(error) => Ok(error as *mut OCIError),
        Err(err) => Err(err),
    }
}

/// Creates a service handle
fn create_service_handle(env: *const OCIEnv,
                         server: *mut OCIServer,
                         error: *mut OCIError)
                         -> Result<*mut OCISvcCtx, OciError> {
    let service_handle = match allocate_handle(env, HandleType::Service) {
        Ok(service) => service as *mut OCISvcCtx,
        Err(err) => return Err(err),
    };
    let size: c_uint = 0;
    match set_handle_attribute(service_handle as *mut c_void,
                               HandleType::Service,
                               server as *mut c_void,
                               size,
                               AttributeType::Server,
                               error,
                               "Setting server in service handle") {
        None => Ok(service_handle),
        Some(err) => Err(err),
    }
}

/// create sesion handle
fn create_user_session_handle(env: *const OCIEnv) -> Result<*mut OCISession, OciError> {
    match allocate_handle(env, HandleType::Session) {
        Ok(session) => Ok(session as *mut OCISession),
        Err(err) => Err(err),
    }
}

/// set user name
fn set_user_name(session: *mut OCISession,
                 user_name: &str,
                 error: *mut OCIError)
                 -> Option<OciError> {
    let user_name_bytes = user_name.as_bytes();
    let user_name_bytes_ptr = user_name_bytes.as_ptr();
    let user_name_len = user_name.len() as c_uint;

    match set_handle_attribute(session as *mut c_void,
                               HandleType::Session,
                               user_name_bytes_ptr as *mut c_void,
                               user_name_len,
                               AttributeType::UserName.into(),
                               error,
                               "Setting user name") {
        None => None,
        Some(err) => Some(err),
    }
}

/// set password
fn set_password(session: *mut OCISession,
                password: &str,
                error: *mut OCIError)
                -> Option<OciError> {
    let password_bytes = password.as_bytes();
    let password_bytes_ptr = password_bytes.as_ptr();
    let password_len = password.len() as c_uint;

    match set_handle_attribute(session as *mut c_void,
                               HandleType::Session,
                               password_bytes_ptr as *mut c_void,
                               password_len,
                               AttributeType::Password.into(),
                               error,
                               "Setting password") {
        None => None,
        Some(err) => Some(err),
    }
}

/// Set handle attribute
fn set_handle_attribute(handle: *mut c_void,
                        handle_type: HandleType,
                        attribute_handle: *mut c_void,
                        size: c_uint,
                        attribute_type: AttributeType,
                        error_handle: *mut OCIError,
                        error_description: &str)
                        -> Option<OciError> {
    let attr_set_result = unsafe {
        OCIAttrSet(handle,
                   handle_type.into(),
                   attribute_handle,
                   size,
                   attribute_type.into(),
                   error_handle)
    };
    match attr_set_result.into() {
        ReturnCode::Success => None,
        _ => {
            Some(get_error(error_handle as *mut c_void,
                           HandleType::Error,
                           error_description))
        }
    }
}



/// Allocate a handle
fn allocate_handle(env: *const OCIEnv, handle_type: HandleType) -> Result<*mut c_void, OciError> {
    let mut handle: *mut c_void = ptr::null_mut();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let allocation_result = unsafe {
        OCIHandleAlloc(env as *const c_void,
                       &mut handle,
                       handle_type.into(),
                       xtramem_sz,
                       null_ptr)
    };
    match allocation_result.into() {
        ReturnCode::Success => Ok(handle),
        _ => {
            Err(get_error(env as *mut c_void,
                          HandleType::Environment,
                          handle_type.into()))
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

/// start user session
fn start_user_session(service: *mut OCISvcCtx,
                      error: *mut OCIError,
                      session: *mut OCISession)
                      -> Option<OciError> {

    let session_result = unsafe {
        OCISessionBegin(service,
                        error,
                        session,
                        CredentialsType::Rdbms.into(),
                        EnvironmentMode::Default.into())
    };

    match session_result.into() {
        ReturnCode::Success => None,
        _ => {
            Some(get_error(error as *mut c_void,
                           HandleType::Error,
                           "Starting user session"))
        }
    }
}
