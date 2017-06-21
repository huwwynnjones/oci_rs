use oci_bindings::{OCIEnv, OCIEnvCreate, HandleType, OCIHandleFree, OCIServer, OCIHandleAlloc,
                   ReturnCode, EnvironmentMode, OCIError, OCISvcCtx, OCIServerAttach,
                   OCIServerDetach, AttributeType, OCIAttrSet, OCISession, OCISessionBegin,
                   CredentialsType, OCISessionEnd, OCIStmt, OCIStmtPrepare2, SyntaxType,
                   OCIStmtRelease, OCIStmtExecute, OCISnapshot, OCITransCommit};
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
    session: *mut OCISession,
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
        let service = create_service_handle(env)?;
        let session = create_session_handle(env)?;
        connect_to_database(server, connection_str, error)?;
        set_server_in_service(service, server, error)?;
        set_user_name_in_session(session, user_name, error)?;
        set_password_in_session(session, password, error)?;
        start_session(service, session, error)?;
        set_session_in_service(service, session, error)?;
        Ok(Connection {
            environment: env,
            server: server,
            error: error,
            service: service,
            session: session,
        })
    }

    /// Creates a new Statement struct. A Statement can only live
    /// as long as the Connection that created it.
    pub fn create_prepared_statement(&self, sql: &str) -> Result<Statement, OciError> {
        Statement::new(self, sql)
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
                          self.session,
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
fn create_service_handle(env: *const OCIEnv) -> Result<*mut OCISvcCtx, OciError> {
    match allocate_handle(env, HandleType::Service) {
        Ok(service) => Ok(service as *mut OCISvcCtx),
        Err(err) => return Err(err),
    }
}

/// set the server handle in the service handle
fn set_server_in_service(service: *mut OCISvcCtx,
                         server: *mut OCIServer,
                         error: *mut OCIError)
                         -> Result<(), OciError> {

    let size: c_uint = 0;
    set_handle_attribute(service as *mut c_void,
                         HandleType::Service,
                         server as *mut c_void,
                         size,
                         AttributeType::Server,
                         error,
                         "Setting server in service handle")?;
    Ok(())
}

/// create sesion handle
fn create_session_handle(env: *const OCIEnv) -> Result<*mut OCISession, OciError> {
    match allocate_handle(env, HandleType::Session) {
        Ok(session) => Ok(session as *mut OCISession),
        Err(err) => Err(err),
    }
}

/// set user name
fn set_user_name_in_session(session: *mut OCISession,
                            user_name: &str,
                            error: *mut OCIError)
                            -> Result<(), OciError> {
    let user_name_bytes = user_name.as_bytes();
    let user_name_bytes_ptr = user_name_bytes.as_ptr();
    let user_name_len = user_name.len() as c_uint;

    set_handle_attribute(session as *mut c_void,
                         HandleType::Session,
                         user_name_bytes_ptr as *mut c_void,
                         user_name_len,
                         AttributeType::UserName.into(),
                         error,
                         "Setting user name")?;
    Ok(())
}

/// set password
fn set_password_in_session(session: *mut OCISession,
                           password: &str,
                           error: *mut OCIError)
                           -> Result<(), OciError> {
    let password_bytes = password.as_bytes();
    let password_bytes_ptr = password_bytes.as_ptr();
    let password_len = password.len() as c_uint;

    set_handle_attribute(session as *mut c_void,
                         HandleType::Session,
                         password_bytes_ptr as *mut c_void,
                         password_len,
                         AttributeType::Password.into(),
                         error,
                         "Setting password")?;
    Ok(())
}

/// Set user session in service handle
fn set_session_in_service(service: *mut OCISvcCtx,
                          session: *mut OCISession,
                          error: *mut OCIError)
                          -> Result<(), OciError> {

    let size: c_uint = 0;
    set_handle_attribute(service as *mut c_void,
                         HandleType::Service.into(),
                         session as *mut c_void,
                         size,
                         AttributeType::Session,
                         error,
                         "Setting user session in service")?;
    Ok(())
}

/// Set handle attribute
fn set_handle_attribute(handle: *mut c_void,
                        handle_type: HandleType,
                        attribute_handle: *mut c_void,
                        size: c_uint,
                        attribute_type: AttributeType,
                        error_handle: *mut OCIError,
                        error_description: &str)
                        -> Result<(), OciError> {
    let attr_set_result = unsafe {
        OCIAttrSet(handle,
                   handle_type.into(),
                   attribute_handle,
                   size,
                   attribute_type.into(),
                   error_handle)
    };
    match attr_set_result.into() {
        ReturnCode::Success => Ok(()),
        _ => {
            Err(get_error(error_handle as *mut c_void,
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
                       connection_str: &str,
                       error: *mut OCIError)
                       -> Result<(), OciError> {
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
        ReturnCode::Success => Ok(()),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Environment,
                          "Database connection"))
        }
    }
}

/// start user session
fn start_session(service: *mut OCISvcCtx,
                 session: *mut OCISession,
                 error: *mut OCIError)
                 -> Result<(), OciError> {

    let session_result = unsafe {
        OCISessionBegin(service,
                        error,
                        session,
                        CredentialsType::Rdbms.into(),
                        EnvironmentMode::Default.into())
    };

    match session_result.into() {
        ReturnCode::Success => Ok(()),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Starting user session"))
        }
    }
}

#[derive(Debug)]
pub struct Statement<'conn> {
    connection: &'conn Connection,
    statement: *mut OCIStmt,
}
impl<'conn> Statement<'conn> {
    fn new(connection: &'conn Connection, sql: &str) -> Result<Self, OciError> {
        let statement = prepare_statement(connection, sql)?;
        Ok(Statement {
            connection: connection,
            statement: statement,
        })
    }

    pub fn execute(&self) -> Result<(), OciError> {
        let iters = 1 as c_uint;
        let rowoff = 0 as c_uint;
        let snap_in: *const OCISnapshot = ptr::null();
        let snap_out: *mut OCISnapshot = ptr::null_mut();
        let execute_result = unsafe {
            OCIStmtExecute(self.connection.service,
                           self.statement,
                           self.connection.error,
                           iters,
                           rowoff,
                           snap_in,
                           snap_out,
                           EnvironmentMode::Default.into())
        };
        match execute_result.into() {
            ReturnCode::Success => Ok(()),
            _ => {
                Err(get_error(self.connection.error as *mut c_void,
                              HandleType::Error,
                              "Executing statement"))
            }
        }
    }

    pub fn commit(&self) -> Result<(), OciError> {
        let commit_result = unsafe {
            OCITransCommit(self.connection.service,
                           self.connection.error,
                           EnvironmentMode::Default.into())
        };
        match commit_result.into() {
            ReturnCode::Success => Ok(()),
            _ => {
                Err(get_error(self.connection.error as *mut c_void,
                              HandleType::Error,
                              "Commiting statement"))
            }
        }
    }
}

impl<'conn> Drop for Statement<'conn> {
    /// Frees any internal handles allocated by the OCI library.
    ///
    /// # Panics
    ///
    /// Panics if the resources can't be freed. This would be
    /// a failure of the underlying OCIStmtRelease function.
    fn drop(&mut self) {
        if let Err(err) = release_statement(self.statement, self.connection.error) {
            panic!(format!("Could not release the statement Statement: {}", err))
        }

    }
}

// release statement
fn release_statement(statement: *mut OCIStmt, error: *mut OCIError) -> Result<(), OciError> {

    let key_ptr = ptr::null();
    let key_len = 0 as c_uint;
    let release_result = unsafe {
        OCIStmtRelease(statement,
                       error,
                       key_ptr,
                       key_len,
                       EnvironmentMode::Default.into())
    };

    match release_result.into() {
        ReturnCode::Success => Ok(()),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Releasing statement"))
        }
    }
}

/// create statement handle and prepare sql
fn prepare_statement(connection: &Connection, sql: &str) -> Result<*mut OCIStmt, OciError> {
    let statement: *mut OCIStmt = ptr::null_mut();
    let sql_bytes = sql.as_bytes();
    let sql_bytes_ptr = sql_bytes.as_ptr();
    let sql_len = sql.len() as c_uint;
    let key_ptr = ptr::null();
    let key_len = 0 as c_uint;
    let prepare_result = unsafe {
        OCIStmtPrepare2(connection.service,
                        &statement,
                        connection.error,
                        sql_bytes_ptr,
                        sql_len,
                        key_ptr,
                        key_len,
                        SyntaxType::Ntv.into(),
                        EnvironmentMode::Default.into())
    };

    match prepare_result.into() {
        ReturnCode::Success => Ok(statement),
        _ => {
            let mut err_txt = String::from("Preparing statement: ");
            err_txt.push_str(sql);
            Err(get_error(connection.error as *mut c_void, HandleType::Error, &err_txt))
        }
    }
}
