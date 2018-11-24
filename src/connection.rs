use common::set_handle_attribute;
use libc::{c_int, c_uint, c_void, size_t};
use oci_bindings::{
    AttributeType, CredentialsType, EnvironmentMode, HandleType, OCIEnv, OCIEnvCreate, OCIError,
    OCIHandleAlloc, OCIHandleFree, OCIServer, OCIServerAttach, OCIServerDetach, OCISession,
    OCISessionBegin, OCISessionEnd, OCISvcCtx, ReturnCode,
};
use oci_error::{get_error, OciError};
use statement::Statement;
use std::ptr;

/// Represents a connection to a database.
///
/// Internally it holds the various handles that are needed to maintain
/// a connection to the database. Once it goes out of scope it will free these handles using
/// the relevant OCI calls via a Drop implementation.
///
#[derive(Debug)]
pub struct Connection {
    environment: *mut OCIEnv,
    server: *mut OCIServer,
    error: *mut OCIError,
    service: *mut OCISvcCtx,
    session: *mut OCISession,
}
impl Connection {
    /// Creates a new `Connection`.
    ///
    /// # Errors
    ///
    /// Any errors encounter when trying to allocate handles in OCI library will bubble up here.
    /// The [`OciError`][1] will return the relevant Oracle error codes and text when available.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use oci_rs::connection::Connection;
    ///
    /// let connection = Connection::new("localhost:1521/xe",
    ///                                  "user",
    ///                                  "password")
    ///                                  .unwrap();
    /// ```
    ///
    /// [1]: ../oci_error/enum.OciError.html
    ///
    pub fn new(
        connection_str: &str,
        user_name: &str,
        password: &str,
    ) -> Result<Connection, OciError> {
        let environment = create_environment_handle()?;
        let server = create_server_handle(environment)?;
        let error = create_error_handle(environment)?;
        let service = create_service_handle(environment)?;
        let session = create_session_handle(environment)?;
        connect_to_database(server, connection_str, error)?;
        set_server_in_service(service, server, error)?;
        set_user_name_in_session(session, user_name, error)?;
        set_password_in_session(session, password, error)?;
        start_session(service, session, error)?;
        set_session_in_service(service, session, error)?;
        Ok(Connection {
            environment,
            server,
            error,
            service,
            session,
        })
    }

    /// Creates a new [`Statement`][2].
    ///
    /// A `Statement` can only live as long as the `Connection` that created it. The SQL
    /// statement that needs to be executed is supplied. A connection can have multiple
    /// statements active.
    ///
    /// # Errors
    ///
    /// Any OCI failures will be reported and the relevant Oracle error codes available.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use oci_rs::connection::Connection;
    ///
    /// let connection = Connection::new("localhost:1521/xe",
    ///                                  "user",
    ///                                  "password")
    ///                                  .unwrap();
    ///
    /// let sql_select = "SELECT * FROM SomeTable";
    /// let select_stmt = match connection.create_prepared_statement(sql_select) {
    ///     Ok(stmt) => stmt,
    ///     Err(err) => panic!("Oracle error: {}", err),
    /// };
    /// ```
    ///
    /// [2]: ../statement/struct.Statement.html
    pub fn create_prepared_statement(&self, sql: &str) -> Result<Statement, OciError> {
        Statement::new(self, sql)
    }

    /// Returns the error handle for the connection.
    pub(crate) fn error(&self) -> *mut OCIError {
        self.error
    }

    /// Some calls to OCI functions require the error handle to be converted to a `c_void`
    /// , this is a convience method for that.
    pub(crate) fn error_as_void(&self) -> *mut c_void {
        self.error as *mut c_void
    }

    /// Returns the service handle for the connection.
    pub(crate) fn service(&self) -> *mut OCISvcCtx {
        self.service
    }
}

impl Drop for Connection {
    /// Ends the current user session, disconnects from the database and frees the handles
    /// allocated by the OCI library.
    ///
    /// This should ensure there are no remaining processes or memory allocated.
    ///
    /// # Panics
    ///
    /// Panics if the resources can't be freed. This would be
    /// a failure of the underlying OCI resource freeing function.
    ///
    fn drop(&mut self) {
        let session_end_result = unsafe {
            OCISessionEnd(
                self.service,
                self.error,
                self.session,
                EnvironmentMode::Default.into(),
            )
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
            OCIHandleFree(
                self.environment as *mut c_void,
                HandleType::Environment.into(),
            )
        };

        match free_result.into() {
            ReturnCode::Success => (),
            _ => panic!("Could not free the handles in Connection"),
        }
    }
}

/// Creates an environment handle
fn create_environment_handle() -> Result<*mut OCIEnv, OciError> {
    let env: *mut OCIEnv = ptr::null_mut();
    let mode = EnvironmentMode::Threaded.into();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let env_result = unsafe {
        OCIEnvCreate(
            &env, mode, null_ptr, null_ptr, null_ptr, null_ptr, xtramem_sz, null_ptr,
        )
    };
    match env_result.into() {
        ReturnCode::Success => Ok(env),
        _ => Err(get_error(
            env as *mut c_void,
            HandleType::Environment,
            "Environment handle creation",
        )),
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
        Err(err) => Err(err),
    }
}

/// set the server handle in the service handle
fn set_server_in_service(
    service: *mut OCISvcCtx,
    server: *mut OCIServer,
    error: *mut OCIError,
) -> Result<(), OciError> {
    let size: c_uint = 0;
    set_handle_attribute(
        service as *mut c_void,
        HandleType::Service,
        server as *mut c_void,
        size,
        AttributeType::Server,
        error,
        "Setting server in service handle",
    )?;
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
fn set_user_name_in_session(
    session: *mut OCISession,
    user_name: &str,
    error: *mut OCIError,
) -> Result<(), OciError> {
    let user_name_ptr = user_name.as_ptr();
    let user_name_len = user_name.len() as c_uint;

    set_handle_attribute(
        session as *mut c_void,
        HandleType::Session,
        user_name_ptr as *mut c_void,
        user_name_len,
        AttributeType::UserName,
        error,
        "Setting user name",
    )?;
    Ok(())
}

/// set password
fn set_password_in_session(
    session: *mut OCISession,
    password: &str,
    error: *mut OCIError,
) -> Result<(), OciError> {
    let password_ptr = password.as_ptr();
    let password_len = password.len() as c_uint;

    set_handle_attribute(
        session as *mut c_void,
        HandleType::Session,
        password_ptr as *mut c_void,
        password_len,
        AttributeType::Password,
        error,
        "Setting password",
    )?;
    Ok(())
}

/// Set user session in service handle
fn set_session_in_service(
    service: *mut OCISvcCtx,
    session: *mut OCISession,
    error: *mut OCIError,
) -> Result<(), OciError> {
    let size: c_uint = 0;
    set_handle_attribute(
        service as *mut c_void,
        HandleType::Service,
        session as *mut c_void,
        size,
        AttributeType::Session,
        error,
        "Setting user session in service",
    )?;
    Ok(())
}

/// Allocate a handle
fn allocate_handle(env: *const OCIEnv, handle_type: HandleType) -> Result<*mut c_void, OciError> {
    let handle: *mut c_void = ptr::null_mut();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let allocation_result = unsafe {
        OCIHandleAlloc(
            env as *const c_void,
            &handle,
            handle_type.into(),
            xtramem_sz,
            null_ptr,
        )
    };
    match allocation_result.into() {
        ReturnCode::Success => Ok(handle),
        _ => Err(get_error(
            env as *mut c_void,
            HandleType::Environment,
            handle_type.into(),
        )),
    }
}

/// Connect to the database
fn connect_to_database(
    server: *mut OCIServer,
    connection_str: &str,
    error: *mut OCIError,
) -> Result<(), OciError> {
    let conn_ptr = connection_str.as_ptr();
    let conn_len = connection_str.len() as c_int;

    let connect_result = unsafe {
        OCIServerAttach(
            server,
            error,
            conn_ptr,
            conn_len,
            EnvironmentMode::Default.into(),
        )
    };

    match connect_result.into() {
        ReturnCode::Success => Ok(()),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Database connection",
        )),
    }
}

/// start user session
fn start_session(
    service: *mut OCISvcCtx,
    session: *mut OCISession,
    error: *mut OCIError,
) -> Result<(), OciError> {
    let session_result = unsafe {
        OCISessionBegin(
            service,
            error,
            session,
            CredentialsType::Rdbms.into(),
            EnvironmentMode::Default.into(),
        )
    };

    match session_result.into() {
        ReturnCode::Success => Ok(()),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Starting user session",
        )),
    }
}
