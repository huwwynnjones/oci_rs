use oci_bindings::{OCIEnv, OCIEnvCreate, HandleType, OCIHandleFree, OCIServer, OCIHandleAlloc,
                   ReturnCode, EnvironmentMode, OCIError, OCISvcCtx, OCIServerAttach,
                   OCIServerDetach, AttributeType, OCIAttrSet, OCISession, OCISessionBegin,
                   CredentialsType, OCISessionEnd, OCIStmt, OCIStmtPrepare2, SyntaxType,
                   OCIStmtRelease, OCIStmtExecute, OCISnapshot, OCITransCommit, OCIBind,
                   OCIBindByPos, StatementType, OCIAttrGet, OCIParam, OCIParamGet, OCIDefine,
                   OCIDefineByPos, DescriptorType, SqlDataType, FetchType, OCIStmtFetch2,
                   OCIDescriptorFree};
use oci_error::{OciError, get_error};
use types::{ToSqlValue, SqlValue};
use std::ptr;
use row::Row;
use libc::{c_void, size_t, c_int, c_uint, c_ushort};

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
    /// Currently it defaults to an OCI multithreaded mode, the
    /// downside is that use of a connection in a single threaded
    /// environment might be slower.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use oci_rs::connection::Connection;
    ///
    /// let connection = Connection::new("localhost:1521/xe",
    ///                                  "user",
    ///                                  "password")
    ///                                  .expect("Something went wrong");
    /// ```
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
    let env: *mut OCIEnv = ptr::null_mut();
    let mode = EnvironmentMode::Threaded.into();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let env_result = unsafe {
        OCIEnvCreate(&env,
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
        Err(err) => Err(err),
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
    let user_name_ptr = user_name.as_ptr();
    let user_name_len = user_name.len() as c_uint;

    set_handle_attribute(session as *mut c_void,
                         HandleType::Session,
                         user_name_ptr as *mut c_void,
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
    let password_ptr = password.as_ptr();
    let password_len = password.len() as c_uint;

    set_handle_attribute(session as *mut c_void,
                         HandleType::Session,
                         password_ptr as *mut c_void,
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
    let handle: *mut c_void = ptr::null_mut();
    let xtramem_sz: size_t = 0;
    let null_ptr = ptr::null();
    let allocation_result = unsafe {
        OCIHandleAlloc(env as *const c_void,
                       &handle,
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
    let conn_ptr = connection_str.as_ptr();
    let conn_len = connection_str.len() as c_int;

    let connect_result = unsafe {
        OCIServerAttach(server,
                        error,
                        conn_ptr,
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
    bindings: Vec<*mut OCIBind>,
    values: Vec<SqlValue>,
    result: Vec<Row>,
}
impl<'conn> Statement<'conn> {
    fn new(connection: &'conn Connection, sql: &str) -> Result<Self, OciError> {
        let statement = prepare_statement(connection, sql)?;
        Ok(Statement {
            connection: connection,
            statement: statement,
            bindings: Vec::new(),
            values: Vec::new(),
            result: Vec::new(),
        })
    }

    pub fn bind(&mut self, params: &[&ToSqlValue]) -> Result<(), OciError> {
        self.values.clear();

        for (index, param) in params.iter().enumerate() {
            let sql_value = param.to_sql_value();
            self.values.push(sql_value);
            let binding: *mut OCIBind = ptr::null_mut();
            self.bindings.push(binding);

            let position = (index + 1) as c_uint;
            let null_mut_ptr = ptr::null_mut();
            let indp = null_mut_ptr;
            let alenp = null_mut_ptr as *mut c_ushort;
            let rcodep = null_mut_ptr as *mut c_ushort;
            let curelep = null_mut_ptr as *mut c_uint;
            let maxarr_len: c_uint = 0;
            let bind_result = unsafe {
                OCIBindByPos(self.statement,
                             &self.bindings[index],
                             self.connection.error,
                             position,
                             self.values[index].as_oci_ptr(),
                             self.values[index].size(),
                             self.values[index].oci_data_type(),
                             indp,
                             alenp,
                             rcodep,
                             maxarr_len,
                             curelep,
                             EnvironmentMode::Default.into())
            };
            match bind_result.into() {
                ReturnCode::Success => (),
                _ => {
                    return Err(get_error(self.connection.error as *mut c_void,
                                         HandleType::Error,
                                         "Binding parameter"))
                }
            }
        }
        Ok(())
    }

    pub fn execute(&self) -> Result<(), OciError> {

        let stmt_type = get_statement_type(self.statement, self.connection.error)?;
        let iters = match stmt_type {
            StatementType::Select => 0 as c_uint,
            _ => 1 as c_uint,
        };
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

    pub fn result(&mut self) -> Result<&Vec<Row>, OciError> {
        let row = build_row(self.statement, self.connection.error)?;
        self.result.push(row);
        Ok(&self.result)
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
    let sql_ptr = sql.as_ptr();
    let sql_len = sql.len() as c_uint;
    let key_ptr = ptr::null();
    let key_len = 0 as c_uint;
    let prepare_result = unsafe {
        OCIStmtPrepare2(connection.service,
                        &statement,
                        connection.error,
                        sql_ptr,
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

/// find out what sort of statement was prepared
fn get_statement_type(statement: *mut OCIStmt,
                      error: *mut OCIError)
                      -> Result<StatementType, OciError> {

    let mut stmt_type: c_uint = 0;
    let stmt_type_ptr: *mut c_uint = &mut stmt_type;
    let mut size: c_uint = 0;
    let attr_check = unsafe {
        OCIAttrGet(statement as *const c_void,
                   HandleType::Statement.into(),
                   stmt_type_ptr as *mut c_void,
                   &mut size,
                   AttributeType::Statement.into(),
                   error)
    };

    match attr_check.into() {
        ReturnCode::Success => Ok(stmt_type.into()),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Getting statement type"))
        }
    }
}

struct Column {
    handle: *mut OCIParam,
    handle_ptr: *mut c_void,
    binding: *mut OCIDefine,
    sql_type: SqlDataType,
    buffer: Vec<u8>,
    buffer_ptr: *mut c_void,
}
impl Column {
    fn new(statement: *mut OCIStmt,
           error: *mut OCIError,
           position: c_uint)
           -> Result<Column, OciError> {
        let parameter = allocate_parameter_handle(statement, error, position)?;
        let data_size = column_data_size(parameter, error)?;
        let data_type = column_data_type(parameter, error)?;
        let (binding, buffer, buffer_ptr) =
            define_output_parameter(statement, error, position, data_size, data_type)?;
        Ok(Column {
            handle: parameter,
            handle_ptr: parameter as *mut c_void,
            binding: binding,
            sql_type: data_type.into(),
            buffer: buffer,
            buffer_ptr: buffer_ptr,
        })
    }
}

fn define_output_parameter(statement: *mut OCIStmt,
                           error: *mut OCIError,
                           position: c_uint,
                           data_size: c_ushort,
                           data_type: c_ushort)
                           -> Result<(*mut OCIDefine, Vec<u8>, *mut c_void), OciError> {

    let mut buffer = vec![0; data_size as usize];
    let buffer_ptr = buffer.as_mut_ptr() as *mut c_void;
    let define_binding: *mut OCIDefine = ptr::null_mut();
    let null_mut_ptr = ptr::null_mut();
    let indp = null_mut_ptr;
    let rlenp = null_mut_ptr as *mut c_ushort;
    let rcodep = null_mut_ptr as *mut c_ushort;
    let define_result = unsafe {
        OCIDefineByPos(statement,
                       &define_binding,
                       error,
                       position,
                       buffer_ptr,
                       data_size as c_int,
                       data_type,
                       indp,
                       rlenp,
                       rcodep,
                       EnvironmentMode::Default.into())
    };
    match define_result.into() {
        ReturnCode::Success => Ok((define_binding, buffer, buffer_ptr)),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Defining output parameter"))
        }
    }
}

fn column_data_size(parameter: *mut OCIParam, error: *mut OCIError) -> Result<c_ushort, OciError> {
    let mut size: c_ushort = 0;
    let size_ptr: *mut c_ushort = &mut size;
    let null_mut_ptr = ptr::null_mut();
    let size_result = unsafe {
        OCIAttrGet(parameter as *mut c_void,
                   DescriptorType::Parameter.into(),
                   size_ptr as *mut c_void,
                   null_mut_ptr,
                   AttributeType::DataSize.into(),
                   error)
    };

    match size_result.into() {
        ReturnCode::Success => Ok(size),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Getting column data size"))
        }
    }
}

fn column_data_type(parameter: *mut OCIParam, error: *mut OCIError) -> Result<c_ushort, OciError> {
    let mut data_type: c_ushort = 0;
    let data_type_ptr: *mut c_ushort = &mut data_type;
    let null_mut_ptr = ptr::null_mut();
    let size_result = unsafe {
        OCIAttrGet(parameter as *mut c_void,
                   DescriptorType::Parameter.into(),
                   data_type_ptr as *mut c_void,
                   null_mut_ptr,
                   AttributeType::DataType.into(),
                   error)
    };

    match size_result.into() {
        ReturnCode::Success => Ok(data_type),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Getting column data type"))
        }
    }
}

fn allocate_parameter_handle(statement: *mut OCIStmt,
                             error: *mut OCIError,
                             position: c_uint)
                             -> Result<*mut OCIParam, OciError> {
    let handle: *mut OCIParam = ptr::null_mut();
    let handle_result = unsafe {
        OCIParamGet(statement as *const c_void,
                    HandleType::Statement.into(),
                    error,
                    &handle,
                    position)
    };
    match handle_result.into() {
        ReturnCode::Success => Ok(handle),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Allocating parameter handle"))
        }
    }
}

impl Drop for Column {
    fn drop(&mut self) {
        let descriptor_free_result =
            unsafe { OCIDescriptorFree(self.handle_ptr, DescriptorType::Parameter.into()) };
        match descriptor_free_result.into() {
            ReturnCode::Success => (),
            _ => panic!("Could not free the parameter descriptor in Column"),
        }
        //let free_result =
        //    unsafe { OCIHandleFree(self.binding as *mut c_void, HandleType::Define.into()) };
        //match free_result.into() {
        //    ReturnCode::Success => (),
        //    _ => panic!("Could not free the define handle in Column"),
        //}
    }
}

fn number_of_columns(statement: *mut OCIStmt, error: *mut OCIError) -> Result<c_uint, OciError> {

    let mut nmb_cols: c_uint = 0;
    let nmb_cols_ptr: *mut c_uint = &mut nmb_cols;

    let null_mut_ptr = ptr::null_mut();
    let column_result = unsafe {
        OCIAttrGet(statement as *mut c_void,
                   HandleType::Statement.into(),
                   nmb_cols_ptr as *mut c_void,
                   null_mut_ptr,
                   AttributeType::ParameterCount.into(),
                   error)
    };

    match column_result.into() {
        ReturnCode::Success => Ok(nmb_cols),
        _ => {
            return Err(get_error(error as *mut c_void,
                                 HandleType::Error,
                                 "Getting number of columns"))
        }
    }
}


fn build_row(statement: *mut OCIStmt, error: *mut OCIError) -> Result<Row, OciError> {

    let column_count = number_of_columns(statement, error)?;
    let mut columns = Vec::new();

    for position in 1..(column_count + 1) {
        let column = Column::new(statement, error, position)?;
        columns.push(column)
    }
    fetch_next_row(statement, error)?;
    //let sql_values: Vec<SqlValue> = columns.iter()
    //                        .map(|&col| SqlValue::create_from_raw(&col.buffer, &col.sql_type))
    //                        .collect();
    let mut sql_values = Vec::new();
    for col in columns {
       let sql_value = SqlValue::create_from_raw(&col.buffer, &col.sql_type)?;
       sql_values.push(sql_value)
    }
    Ok(Row::create_row(sql_values))
}

fn fetch_next_row(statement: *mut OCIStmt, error: *mut OCIError) -> Result<(), OciError> {
    let nrows = 1 as c_uint;
    let offset = 0 as c_int;
    let fetch_result = unsafe {
        OCIStmtFetch2(statement,
                      error,
                      nrows,
                      FetchType::Next.into(),
                      offset,
                      EnvironmentMode::Default.into())
    };
    match fetch_result.into() {
        ReturnCode::Success => Ok(()),
        _ => return Err(get_error(error as *mut c_void, HandleType::Error, "Fetching")),
    }
}


/// build a row from the data found
fn build_row_old(statement: *mut OCIStmt, error: *mut OCIError) -> Result<Row, OciError> {

    // this needs some cutting, this function so hairy
    let mut position: c_uint = 1;
    let parameter: *mut OCIParam = ptr::null_mut();
    let parameter_ptr = parameter as *mut c_void;
    let mut columns: Vec<SqlValue> = Vec::new();
    let mut define_bindings: Vec<*mut OCIDefine> = Vec::new();
    let mut data_buffers: Vec<Vec<u8>> = Vec::new();
    let mut data_buffer_ptrs: Vec<*mut c_void> = Vec::new();
    let mut sql_types: Vec<SqlDataType> = Vec::new();


    // don't need to loop there is a function to get nmb of columns
    loop {
        let index = (position - 1) as usize;

        let param_result = unsafe {
            OCIParamGet(statement as *const c_void,
                        HandleType::Statement.into(),
                        error,
                        &parameter,
                        position)
        };

        match param_result.into() {
            ReturnCode::Error => {
                let err = get_error(error as *mut c_void,
                                    HandleType::Error,
                                    "Getting a parameter");
                match err.last_error_code() {
                    Some(i) if i == 24334 => break,
                    None | Some(_) => return Err(err),
                }
            }
            _ => (),
        }

        let mut data_type: c_ushort = 0;
        let data_type_ptr: *mut c_ushort = &mut data_type;
        let mut size: c_uint = 0;
        let data_type_result = unsafe {
            OCIAttrGet(parameter as *mut c_void,
                       DescriptorType::Parameter.into(),
                       data_type_ptr as *mut c_void,
                       &mut size,
                       AttributeType::DataType.into(),
                       error)
        };
        match data_type_result.into() {
            ReturnCode::Success => (),
            _ => {
                let code: c_uint = AttributeType::DataType.into();
                let err_text = format!("Getting data type for parameter. 
                                       Attr code {}.
                                       Pos {}.
                                       type {}.",
                                       code,
                                       position,
                                       data_type);
                return Err(get_error(error as *mut c_void, HandleType::Error, &err_text));
            }
        }

        define_bindings.push(ptr::null_mut());
        sql_types.push(data_type.into());
        data_buffers.push(vec![0; sql_types[index].size()]);
        data_buffer_ptrs.push(data_buffers[index].as_mut_ptr() as *mut c_void);
        let null_mut_ptr = ptr::null_mut();
        let indp = null_mut_ptr;
        let rlenp = null_mut_ptr as *mut c_ushort;
        let rcodep = null_mut_ptr as *mut c_ushort;
        let define_result = unsafe {
            OCIDefineByPos(statement,
                           &define_bindings[index],
                           error,
                           position,
                           data_buffer_ptrs[index],
                           sql_types[index].size() as c_int,
                           data_type,
                           indp,
                           rlenp,
                           rcodep,
                           EnvironmentMode::Default.into())
        };
        match define_result.into() {
            ReturnCode::Success => (),
            _ => {
                return Err(get_error(error as *mut c_void,
                                     HandleType::Error,
                                     "Getting column value"))
            }
        }
        position += 1;
    }

    let nrows = 1 as c_uint;
    let offset = 0 as c_int;
    let fetch_result = unsafe {
        OCIStmtFetch2(statement,
                      error,
                      nrows,
                      FetchType::Next.into(),
                      offset,
                      EnvironmentMode::Default.into())
    };
    match fetch_result.into() {
        ReturnCode::Success => (),
        _ => return Err(get_error(error as *mut c_void, HandleType::Error, "Fetching")),
    }

    for (index, data) in data_buffers.iter().enumerate() {
        println!("data {:?}, size {}, cap {}",
                 data,
                 data.len(),
                 data.capacity());
        let sql_value = SqlValue::create_from_raw(data, &sql_types[index])?;
        columns.push(sql_value);
    }
    Ok(Row::create_row(columns))
}
