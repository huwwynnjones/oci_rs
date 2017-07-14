use oci_bindings::{HandleType, ReturnCode, EnvironmentMode, OCIError, AttributeType, OCIStmt,
                   OCIStmtPrepare2, SyntaxType, OCIStmtRelease, OCIStmtExecute, OCISnapshot,
                   OCITransCommit, OCIBind, OCIBindByPos, StatementType, OCIAttrGet, OCIParam,
                   OCIParamGet, OCIDefine, OCIDefineByPos, DescriptorType, OciDataType, FetchType,
                   OCIStmtFetch2, OCIDescriptorFree};
use oci_error::{OciError, get_error};
use types::{ToSqlValue, SqlValue};
use std::ptr;
use connection::Connection;
use row::Row;
use libc::{c_void, c_int, c_uint, c_ushort, c_short, c_schar};

#[derive(Debug)]
enum ResultState {
    Fetched,
    NotFetched,
}

#[derive(Debug)]
pub struct Statement<'conn> {
    connection: &'conn Connection,
    statement: *mut OCIStmt,
    bindings: Vec<*mut OCIBind>,
    values: Vec<SqlValue>,
    result_set: Vec<Row>,
    result_state: ResultState,
}
impl<'conn> Statement<'conn> {
    pub(crate) fn new(// crate) fn new(// crate) fn new(// crate) fn new(// crate) fn new(// crate) fn new(// crate) fn new(
                      connection: &'conn Connection,
                      sql: &str)
                      -> Result<Self, OciError> {
        let statement = prepare_statement(connection, sql)?;
        Ok(Statement {
            connection: connection,
            statement: statement,
            bindings: Vec::new(),
            values: Vec::new(),
            result_set: Vec::new(),
            result_state: ResultState::NotFetched,
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
                             self.connection.error(),
                             position,
                             self.values[index].as_oci_ptr(),
                             self.values[index].size(),
                             self.values[index].as_oci_data_type().into(),
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
                    return Err(get_error(self.connection.error_as_void(),
                                         HandleType::Error,
                                         "Binding parameter"))
                }
            }
        }
        Ok(())
    }

    pub fn execute(&mut self) -> Result<(), OciError> {

        let stmt_type = get_statement_type(self.statement, self.connection.error())?;
        let iters = match stmt_type {
            StatementType::Select => 0 as c_uint,
            _ => 1 as c_uint,
        };
        let rowoff = 0 as c_uint;
        let snap_in: *const OCISnapshot = ptr::null();
        let snap_out: *mut OCISnapshot = ptr::null_mut();
        let execute_result = unsafe {
            OCIStmtExecute(self.connection.service(),
                           self.statement,
                           self.connection.error(),
                           iters,
                           rowoff,
                           snap_in,
                           snap_out,
                           EnvironmentMode::Default.into())
        };
        match execute_result.into() {
            ReturnCode::Success => {
                self.results_not_fetched();
                Ok(())
            }
            _ => {
                Err(get_error(self.connection.error_as_void(),
                              HandleType::Error,
                              "Executing statement"))
            }
        }
    }

    pub fn result_set(&mut self) -> Result<&Vec<Row>, OciError> {
        match self.result_state {
            ResultState::Fetched => (),
            ResultState::NotFetched => {
                self.result_set = build_result_set(self.statement, self.connection.error())?;
                self.results_fetched();
            }
        }
        Ok(&self.result_set)
    }

    pub fn lazy_result_set(&mut self) -> RowIter {
        self.results_fetched();
        RowIter {
            statement: self,
        }
    }

    pub fn commit(&self) -> Result<(), OciError> {
        let commit_result = unsafe {
            OCITransCommit(self.connection.service(),
                           self.connection.error(),
                           EnvironmentMode::Default.into())
        };
        match commit_result.into() {
            ReturnCode::Success => Ok(()),
            _ => {
                Err(get_error(self.connection.error_as_void(),
                              HandleType::Error,
                              "Commiting statement"))
            }
        }
    }

    fn results_fetched(&mut self) -> () {
        self.result_state = ResultState::Fetched
    }

    fn results_not_fetched(&mut self) -> () {
        self.result_state = ResultState::NotFetched
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
        if let Err(err) = release_statement(self.statement, self.connection.error()) {
            panic!(format!("Could not release the statement Statement: {}", err))
        }

    }
}

#[derive(Debug)]
pub struct RowIter<'stmt> {
    statement: &'stmt Statement<'stmt>
}
impl<'stmt> Iterator for RowIter<'stmt> {
    type Item = Result<Row, OciError>;

    fn next(&mut self) -> Option<Result<Row, OciError>> {
        match build_result_row(self.statement.statement, self.statement.connection.error()) {
            Ok(option) => {
                match option {
                    Some(row) => Some(Ok(row)),
                    None => None,
                }
            }
            Err(err) => Some(Err(err)),
        }
    }
}

/// Release statement
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

/// Create statement handle and prepare sql
fn prepare_statement(connection: &Connection, sql: &str) -> Result<*mut OCIStmt, OciError> {
    let statement: *mut OCIStmt = ptr::null_mut();
    let sql_ptr = sql.as_ptr();
    let sql_len = sql.len() as c_uint;
    let key_ptr = ptr::null();
    let key_len = 0 as c_uint;
    let prepare_result = unsafe {
        OCIStmtPrepare2(connection.service(),
                        &statement,
                        connection.error(),
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
            Err(get_error(connection.error_as_void(), HandleType::Error, &err_txt))
        }
    }
}

/// Find out what sort of statement was prepared
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

#[derive(Debug)]
struct Column {
    handle: *mut OCIParam,
    define: *mut OCIDefine,
    sql_type: OciDataType,
    buffer: Vec<u8>,
    buffer_ptr: *mut c_void,
    null_ind: Box<c_short>,
    null_ind_ptr: *mut c_short,
}
impl Column {
    fn new(statement: *mut OCIStmt,
           error: *mut OCIError,
           position: c_uint)
           -> Result<Column, OciError> {
        let parameter = allocate_parameter_handle(statement, error, position)?;
        let data_type = determine_external_data_type(parameter, error)?;
        let data_size = column_data_size(parameter, error)?;
        let (define, buffer, buffer_ptr, null_ind, null_ind_ptr) =
            define_output_parameter(statement, error, position, data_size, &data_type)?;
        Ok(Column {
            handle: parameter,
            define: define,
            sql_type: data_type,
            buffer: buffer,
            buffer_ptr: buffer_ptr,
            null_ind: null_ind,
            null_ind_ptr: null_ind_ptr,
        })
    }

    fn create_sql_value(&self) -> Result<SqlValue, OciError> {
        if self.is_null() {
            Ok(SqlValue::Null)
        } else {
            Ok(SqlValue::create_from_raw(&self.buffer, &self.sql_type)?)
        }
    }

    fn is_null(&self) -> bool {
        *self.null_ind == -1
    }
}

fn define_output_parameter
    (statement: *mut OCIStmt,
     error: *mut OCIError,
     position: c_uint,
     data_size: c_ushort,
     data_type: &OciDataType)
     -> Result<(*mut OCIDefine, Vec<u8>, *mut c_void, Box<c_short>, *mut c_short), OciError> {
    let buffer_size = match *data_type {
        OciDataType::SqlChar => data_size,
        _ => data_type.size(),
    };
    let mut buffer = vec![0; buffer_size as usize];
    let buffer_ptr = buffer.as_mut_ptr() as *mut c_void;
    let define: *mut OCIDefine = ptr::null_mut();
    let null_mut_ptr = ptr::null_mut();
    let mut indp: Box<c_short> = Box::new(0);
    let indp_ptr: *mut c_short = &mut *indp;
    let rlenp = null_mut_ptr as *mut c_ushort;
    let rcodep = null_mut_ptr as *mut c_ushort;
    let define_result = unsafe {
        OCIDefineByPos(statement,
                       &define,
                       error,
                       position,
                       buffer_ptr,
                       buffer_size as c_int,
                       data_type.into(),
                       indp_ptr as *mut c_void,
                       rlenp,
                       rcodep,
                       EnvironmentMode::Default.into())
    };
    match define_result.into() {
        ReturnCode::Success => Ok((define, buffer, buffer_ptr, indp, indp_ptr)),
        _ => {
            Err(get_error(error as *mut c_void,
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
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Getting column data size"))
        }
    }
}

/// Oracle needs to be told what to convert the internal column data
/// into. This is fine for char, but for numbers it is a bit tricky.
/// Internally Oracle stores all numbers as Number, it then expects
/// the caller to tell it what type to use on conversion e.g.
/// please give me an int for that Number. Here we try to fix the
/// conversion to either a integer or float. We can do this by checking the
/// scale and precision of the number in the column. If it the precision is
/// non-zero and scale is -127 then it is float.
fn determine_external_data_type(parameter: *mut OCIParam,
                                error: *mut OCIError)
                                -> Result<OciDataType, OciError> {

    let internal_data_type = column_internal_data_type(parameter, error)?;
    match internal_data_type {
        OciDataType::SqlChar => Ok(OciDataType::SqlChar),
        OciDataType::SqlNum => {
            let precision = column_data_precision(parameter, error)?;
            let scale = column_data_scale(parameter, error)?;
            if (precision != 0) && (scale == -127) {
                Ok(OciDataType::SqlFloat)
            } else {
                Ok(OciDataType::SqlInt)
            }
        }
        _ => panic!("Uknown external conversion"),
    }
}

fn column_internal_data_type(parameter: *mut OCIParam,
                             error: *mut OCIError)
                             -> Result<OciDataType, OciError> {
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
        ReturnCode::Success => Ok(data_type.into()),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Getting column data type"))
        }
    }
}

fn column_data_precision(parameter: *mut OCIParam,
                         error: *mut OCIError)
                         -> Result<c_short, OciError> {
    let mut precision: c_short = 0;
    let precision_ptr: *mut c_short = &mut precision;
    let null_mut_ptr = ptr::null_mut();
    let precision_result = unsafe {
        OCIAttrGet(parameter as *mut c_void,
                   DescriptorType::Parameter.into(),
                   precision_ptr as *mut c_void,
                   null_mut_ptr,
                   AttributeType::Precision.into(),
                   error)
    };
    match precision_result.into() {
        ReturnCode::Success => Ok(precision),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Getting column precision"))
        }
    }
}

fn column_data_scale(parameter: *mut OCIParam, error: *mut OCIError) -> Result<c_schar, OciError> {
    let mut scale: c_schar = 0;
    let scale_ptr: *mut c_schar = &mut scale;
    let null_mut_ptr = ptr::null_mut();
    let scale_result = unsafe {
        OCIAttrGet(parameter as *mut c_void,
                   DescriptorType::Parameter.into(),
                   scale_ptr as *mut c_void,
                   null_mut_ptr,
                   AttributeType::Scale.into(),
                   error)
    };
    match scale_result.into() {
        ReturnCode::Success => Ok(scale),
        _ => {
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Getting column scale"))
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
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Allocating parameter handle"))
        }
    }
}

impl Drop for Column {
    fn drop(&mut self) {
        // let define_free_result =
        //    unsafe { OCIHandleFree(self.define as *mut c_void, HandleType::Define.into()) };
        // match define_free_result.into() {
        //    ReturnCode::Success => (),
        //    _ => panic!("Could not free the define handle in Column"),
        // }
        let descriptor_free_result = unsafe {
            OCIDescriptorFree(self.handle as *mut c_void, DescriptorType::Parameter.into())
        };
        match descriptor_free_result.into() {
            ReturnCode::Success => (),
            _ => panic!("Could not free the parameter descriptor in Column"),
        }
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
            Err(get_error(error as *mut c_void,
                          HandleType::Error,
                          "Getting number of columns"))
        }
    }
}

fn build_result_row(statement: *mut OCIStmt,
                    error: *mut OCIError)
                    -> Result<Option<Row>, OciError> {
    let column_count = number_of_columns(statement, error)?;
    let mut columns = Vec::new();

    for position in 1..(column_count + 1) {
        let column = Column::new(statement, error, position)?;
        columns.push(column)
    }

    match fetch_next_row(statement, error) {
        Ok(result) => {
            match result {
                FetchResult::Data => (),
                FetchResult::NoData => return Ok(None),
            }
        }
        Err(err) => return Err(err),
    }

    let mut sql_values = Vec::new();
    for col in columns {
        sql_values.push(col.create_sql_value()?);
    }
    Ok(Some(Row::new(sql_values)))
}

fn build_result_set(statement: *mut OCIStmt, error: *mut OCIError) -> Result<Vec<Row>, OciError> {
    let mut rows = Vec::new();
    loop {
        let row = match build_result_row(statement, error) {
            Ok(result) => {
                match result {
                    Some(row) => row,
                    None => break,
                }
            }
            Err(err) => return Err(err),
        };
        rows.push(row)
    }
    Ok(rows)
}

enum FetchResult {
    Data,
    NoData,
}

fn fetch_next_row(statement: *mut OCIStmt, error: *mut OCIError) -> Result<FetchResult, OciError> {
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
        ReturnCode::Success => Ok(FetchResult::Data),
        ReturnCode::NoData => Ok(FetchResult::NoData),
        _ => Err(get_error(error as *mut c_void, HandleType::Error, "Fetching")),
    }
}
