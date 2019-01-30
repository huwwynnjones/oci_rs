use crate::common::set_handle_attribute;
use crate::connection::Connection;
use libc::{c_int, c_schar, c_short, c_uint, c_ushort, c_void};
use crate::oci_bindings::{
    AttributeType, DescriptorType, EnvironmentMode, FetchType, HandleType, OCIAttrGet, OCIBind,
    OCIBindByPos, OCIDefine, OCIDefineByPos, OCIDescriptorFree, OCIError, OCIParam, OCIParamGet,
    OCISnapshot, OCIStmt, OCIStmtExecute, OCIStmtFetch2, OCIStmtPrepare2, OCIStmtRelease,
    OCITransCommit, OciDataType, ReturnCode, StatementType, SyntaxType,
};
use crate::oci_error::{get_error, OciError};
use crate::row::Row;
use std::ptr;
use crate::types::{SqlValue, ToSqlValue};

#[derive(Debug)]
enum ResultState {
    Fetched,
    NotFetched,
}

/// Represents a statement that is executed against a database.
///
/// A `Statement` cannot be created directly, instead it is brought to life through
/// the `.create_prepared_statement` method of a [`Connection`][1]. It can only live as
/// long as its parent `Connection` and when it goes out of scope the underlying resources
/// will be released via a `Drop` implementation.
///
/// A `Statement` is stateful. Binding parameters and retrieving the result set will update the
/// state
/// of the object. The underlying OCI objects are stateful and re-use of an OCI statement for new
/// binding parameters or diferent results is more efficient than allocating resources for a new
/// statement. At the moment changing the SQL requires a new `Statement` but it might prove useful
/// in future to allow this to be also changed without new allocation in the OCI library.
///
/// See the [module level documentation][2] for an overview plus examples.
///
/// [1]: ../connection/struct.Connection.html
/// [2]: index.html
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
    /// Creates a new `Statement`.
    ///
    pub(crate) fn new(connection: &'conn Connection, sql: &str) -> Result<Self, OciError> {
        let statement = prepare_statement(connection, sql)?;
        Ok(Statement {
            connection,
            statement,
            bindings: Vec::new(),
            values: Vec::new(),
            result_set: Vec::new(),
            result_state: ResultState::NotFetched,
        })
    }

    /// Sets the parameters that will be used in a SQL statement with bind variables.
    ///
    /// The parameters are anything that implement the `ToSqlValue` trait.
    ///
    /// # Errors
    ///
    /// Any error in the underlying calls to the OCI library will be returned.
    ///
    /// # Examples
    ///
    /// Here are various ways to bind parameters:
    ///
    /// ```rust
    /// use oci_rs::connection::Connection;
    ///
    /// let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
    ///
    /// # let mut drop = conn.create_prepared_statement("DROP TABLE Dogs").unwrap();
    /// # drop.execute().ok();
    /// # let sql_create = "CREATE TABLE Dogs (DogId INTEGER,
    /// #                                      Name VARCHAR(20))";
    /// # let mut create = conn.create_prepared_statement(sql_create).unwrap();
    /// # create.execute().unwrap();
    /// # create.commit().unwrap();
    ///
    /// // Insert some values using bind variables
    /// let sql_insert = "INSERT INTO Dogs (DogId, Name)
    ///                   VALUES (:id, :name)";
    ///
    /// let mut insert = conn.create_prepared_statement(sql_insert).unwrap();
    ///
    /// let id = 1;
    /// let name = "Poodle";
    ///
    /// insert.bind(&[&id, &name]).unwrap();
    /// insert.execute().unwrap();
    ///
    /// insert.bind(&[&2, &"Bulldog"]).unwrap();
    /// insert.execute().unwrap();
    ///
    /// insert.commit();
    ///
    /// let sql_select = "SELECT Name FROM Dogs";
    ///
    /// let mut select = conn.create_prepared_statement(sql_select).unwrap();
    /// select.execute().unwrap();
    ///
    /// let correct_results = vec!["Poodle".to_string(), "Bulldog".to_string()];
    /// let results: Vec<String> = select.lazy_result_set()
    ///                                  .map(|row_result| row_result.unwrap())
    ///                                  .map(|row| row[0].value::<String>().unwrap())
    ///                                  .collect();
    ///
    /// assert_eq!(results, correct_results);
    /// ```
    /// For large scale inserts to the database this is a bit inefficient as many calls to bind
    /// the parameters are needed. OCI does support batch processing and/or arrays of bind
    /// parameters, however this is not yet available through this crate.
    ///
    pub fn bind(&mut self, params: &[&ToSqlValue]) -> Result<(), OciError> {
        // clear out previous bind parameters
        self.values.clear();

        // ensure that the vec is large enough to hold all the parameters
        // otherwise the vec will re-size, re-allocate and the addresses will change
        self.values.reserve(params.len());

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
                OCIBindByPos(
                    self.statement,
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
                    EnvironmentMode::Default.into(),
                )
            };
            match bind_result.into() {
                ReturnCode::Success => (),
                _ => {
                    return Err(get_error(
                        self.connection.error_as_void(),
                        HandleType::Error,
                        "Binding parameter",
                    ))
                }
            }
        }
        Ok(())
    }

    /// Executes the SQL statement.
    ///
    /// # Errors
    ///
    /// Any error in the underlying calls to the OCI library will be returned.
    ///
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
            OCIStmtExecute(
                self.connection.service(),
                self.statement,
                self.connection.error(),
                iters,
                rowoff,
                snap_in,
                snap_out,
                EnvironmentMode::Default.into(),
            )
        };
        match execute_result.into() {
            ReturnCode::Success => {
                self.results_not_fetched();
                Ok(())
            }
            _ => Err(get_error(
                self.connection.error_as_void(),
                HandleType::Error,
                "Executing statement",
            )),
        }
    }

    /// Returns the results of a `SELECT` statement.
    ///
    /// After the execution of a `SELECT` statement a result set will be available from the
    /// database. This will contain none or many `Row`s of data depending on the query. There are
    /// two options for seeing the results, the first is to call this method to retrieve all the
    /// rows in one go, the second is to iterate through them row by row.
    ///
    /// Should you go for the first option then the rows will be fetched once this method is
    /// called. They will not be fetched eagerly as part of the `.execute` call, although this is
    /// not apparent to the caller. Once the results are retrieved from the database then they will
    /// be held until either the `Statement` goes out of scope or `.execute` is called again. This
    /// way, repeated calls to `.result_set` will be the same. If there are no data then an empty
    /// `Vec<Row>` will be returned.
    ///
    /// # Errors
    ///
    /// Any error in the underlying calls to the OCI library will be returned.
    ///
    pub fn result_set(&mut self) -> Result<&[Row], OciError> {
        match self.result_state {
            ResultState::Fetched => (),
            ResultState::NotFetched => {
                let rows: Result<Vec<Row>, _> = self.lazy_result_set().collect();
                self.result_set = rows?;
                self.results_fetched();
            }
        }
        Ok(&self.result_set)
    }

    /// Set the number of rows that will be prefetched from the database.
    ///
    /// The OCI library internally manages the number of rows that are pre-fetched from the
    /// database. This can be tweaked. The OCI default is one row, so for each call to the
    /// database two rows are retrieved, thus half the number of round trips needed.
    ///
    /// # Errors
    ///
    /// Any error in the underlying calls to the OCI library will be returned.
    ///
    pub fn set_prefetch(&mut self, nmb_of_rows: i32) -> Result<(), OciError> {
        let size: c_uint = 0;
        let rows: c_uint = nmb_of_rows as c_uint;
        let rows_ptr: *const c_uint = &rows;
        set_handle_attribute(
            self.statement as *mut c_void,
            HandleType::Statement,
            rows_ptr as *mut c_void,
            size,
            AttributeType::PrefetchRows,
            self.connection.error(),
            "Setting prefetch rows in statement handle",
        )?;
        Ok(())
    }

    /// Returns the results of a `SELECT` statement row by row via the `RowIter` iterator.
    ///
    /// The `RowIter` returned can then be used to run through the rows of data in the result set.
    /// This is a more attractive option if there are many rows or you want to process the results
    /// in a pipeline.
    ///
    /// The same comments about pre-fetching configuration applies here as to `.result_set`.
    ///
    /// # Errors
    ///
    /// This method will not report errors directly however subsequent use of `RowIter` will return
    /// any OCI errors encountered as each row is fetched.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use oci_rs::connection::Connection;
    ///
    /// let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
    ///
    /// # let mut drop = conn.create_prepared_statement("DROP TABLE Countries").unwrap();
    /// # drop.execute().ok();
    /// # let sql_create = "CREATE TABLE Countries (CountryId INTEGER,
    /// #                                           Name VARCHAR(20))";
    /// # let mut create = conn.create_prepared_statement(sql_create).unwrap();
    /// # create.execute().unwrap();
    /// # create.commit().unwrap();
    ///
    /// // Insert some values using bind variables
    /// let sql_insert = "INSERT INTO Countries (CountryId, Name)
    ///                   VALUES (:id, :name)";
    /// let mut insert = conn.create_prepared_statement(sql_insert).unwrap();
    ///
    /// let countries = vec!["Great Britain",
    ///                      "Australia",
    ///                      "Burma",
    ///                      "Japan",
    ///                      "Sudan",
    ///                      "France",
    ///                      "Germany",
    ///                      "China"];
    ///
    /// for (index, country) in countries.iter().enumerate(){
    ///     let id = (index + 1) as i64;
    ///     insert.bind(&[&id, country]).unwrap();
    ///     insert.execute();
    /// }
    /// insert.commit();
    ///
    /// let sql_select = "SELECT Name FROM Countries";
    /// let mut select = conn.create_prepared_statement(sql_select).unwrap();
    /// select.execute().unwrap();
    ///
    /// let results: Vec<String> = select.lazy_result_set()
    ///                                  .map(|row_result| row_result.unwrap())
    ///                                  .map(|row| row[0].value::<String>().unwrap())
    ///                                  .filter(|country| country.contains("c") ||
    ///                                                    country.contains("C"))
    ///                                  .map(|country| country.to_uppercase())
    ///                                  .collect();
    /// assert_eq!(results.len(), 2);
    /// assert!(results.contains(&"CHINA".to_string()));
    /// assert!(results.contains(&"FRANCE".to_string()));
    /// ```
    ///
    pub fn lazy_result_set(&mut self) -> RowIter {
        match self.result_state {
            ResultState::Fetched => panic!("Lazy fetch already completed."),
            ResultState::NotFetched => {
                self.results_fetched();
                RowIter { statement: self }
            }
        }
    }

    /// Commits the changes to the database.
    ///
    /// When a statement makes changes to the database Oracle implicitly starts a
    /// transaction. If all is well and the session is closed normally this will cause an
    /// implicit commit of the changes. If anything goes wrong and the sesssion is not closed or
    /// the connection is broken, Oracle will roll back the changes. This method, therefore allows
    /// you to commit changes when you want, rather than relying on a successfull disconnection.
    ///
    /// # Errors
    ///
    /// Any error in the underlying calls to the OCI library will be returned.
    ///
    pub fn commit(&self) -> Result<(), OciError> {
        let commit_result = unsafe {
            OCITransCommit(
                self.connection.service(),
                self.connection.error(),
                EnvironmentMode::Default.into(),
            )
        };
        match commit_result.into() {
            ReturnCode::Success => Ok(()),
            _ => Err(get_error(
                self.connection.error_as_void(),
                HandleType::Error,
                "Commiting statement",
            )),
        }
    }

    /// Transition to fetched state.
    ///
    fn results_fetched(&mut self) -> () {
        self.result_state = ResultState::Fetched
    }

    /// Transition to not-fetched state.
    ///
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
    /// a failure of the underlying OCI function.
    fn drop(&mut self) {
        if let Err(err) = release_statement(self.statement, self.connection.error()) {
            panic!(format!(
                "Could not release the statement Statement: {}",
                err
            ))
        }
    }
}

/// An iterator that will allow results to be returned row by row.
///
/// See [`Statement.lazy_result_set`][1] for more info.
///
/// [1]: struct.Statement.html#method.lazy_result_set
#[derive(Debug)]
pub struct RowIter<'stmt> {
    statement: &'stmt Statement<'stmt>,
}
impl<'stmt> Iterator for RowIter<'stmt> {
    type Item = Result<Row, OciError>;

    fn next(&mut self) -> Option<Result<Row, OciError>> {
        match build_result_row(self.statement.statement, self.statement.connection.error()) {
            Ok(option) => match option {
                Some(row) => Some(Ok(row)),
                None => None,
            },
            Err(err) => Some(Err(err)),
        }
    }
}

/// Release statement
fn release_statement(statement: *mut OCIStmt, error: *mut OCIError) -> Result<(), OciError> {
    let key_ptr = ptr::null();
    let key_len = 0 as c_uint;
    let release_result = unsafe {
        OCIStmtRelease(
            statement,
            error,
            key_ptr,
            key_len,
            EnvironmentMode::Default.into(),
        )
    };

    match release_result.into() {
        ReturnCode::Success => Ok(()),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Releasing statement",
        )),
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
        OCIStmtPrepare2(
            connection.service(),
            &statement,
            connection.error(),
            sql_ptr,
            sql_len,
            key_ptr,
            key_len,
            SyntaxType::Ntv.into(),
            EnvironmentMode::Default.into(),
        )
    };

    match prepare_result.into() {
        ReturnCode::Success => Ok(statement),
        _ => {
            let mut err_txt = String::from("Preparing statement: ");
            err_txt.push_str(sql);
            Err(get_error(
                connection.error_as_void(),
                HandleType::Error,
                &err_txt,
            ))
        }
    }
}

/// Find out what sort of statement was prepared
fn get_statement_type(
    statement: *mut OCIStmt,
    error: *mut OCIError,
) -> Result<StatementType, OciError> {
    let mut stmt_type: c_uint = 0;
    let stmt_type_ptr: *mut c_uint = &mut stmt_type;
    let mut size: c_uint = 0;
    let attr_check = unsafe {
        OCIAttrGet(
            statement as *const c_void,
            HandleType::Statement.into(),
            stmt_type_ptr as *mut c_void,
            &mut size,
            AttributeType::Statement.into(),
            error,
        )
    };

    match attr_check.into() {
        ReturnCode::Success => Ok(stmt_type.into()),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Getting statement type",
        )),
    }
}

#[derive(Debug)]

struct ColumnPtrHolder {
    define: *mut OCIDefine,
    buffer: Vec<u8>,
    buffer_ptr: *mut c_void,
    null_ind: Box<c_short>,
    null_ind_ptr: *mut c_short,
}

#[derive(Debug)]
struct Column {
    handle: *mut OCIParam,
    sql_type: OciDataType,
    column_ptr_holder: ColumnPtrHolder,
}
impl Column {
    fn new(
        statement: *mut OCIStmt,
        error: *mut OCIError,
        position: c_uint,
    ) -> Result<Column, OciError> {
        let parameter = allocate_parameter_handle(statement, error, position)?;
        let data_type = determine_external_data_type(parameter, error)?;
        let data_size = column_data_size(parameter, error)?;
        let column_ptr_holder =
            define_output_parameter(statement, error, position, data_size, &data_type)?;
        Ok(Column {
            handle: parameter,
            sql_type: data_type,
            column_ptr_holder,
        })
    }

    fn create_sql_value(&self) -> Result<SqlValue, OciError> {
        if self.is_null() {
            Ok(SqlValue::Null)
        } else {
            Ok(SqlValue::create_from_raw(
                &self.column_ptr_holder.buffer,
                &self.sql_type,
            )?)
        }
    }

    fn is_null(&self) -> bool {
        *self.column_ptr_holder.null_ind == -1
    }
}

fn define_output_parameter(
    statement: *mut OCIStmt,
    error: *mut OCIError,
    position: c_uint,
    data_size: c_ushort,
    data_type: &OciDataType,
) -> Result<ColumnPtrHolder, OciError> {
    let buffer_size = match *data_type {
        OciDataType::SqlVarChar | OciDataType::SqlChar => data_size,
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
        OCIDefineByPos(
            statement,
            &define,
            error,
            position,
            buffer_ptr,
            i32::from(buffer_size),
            data_type.into(),
            indp_ptr as *mut c_void,
            rlenp,
            rcodep,
            EnvironmentMode::Default.into(),
        )
    };
    match define_result.into() {
        ReturnCode::Success => Ok(ColumnPtrHolder {
            define,
            buffer,
            buffer_ptr,
            null_ind: indp,
            null_ind_ptr: indp_ptr,
        }),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Defining output parameter",
        )),
    }
}

fn column_data_size(parameter: *mut OCIParam, error: *mut OCIError) -> Result<c_ushort, OciError> {
    let mut size: c_ushort = 0;
    let size_ptr: *mut c_ushort = &mut size;
    let null_mut_ptr = ptr::null_mut();
    let size_result = unsafe {
        OCIAttrGet(
            parameter as *mut c_void,
            DescriptorType::Parameter.into(),
            size_ptr as *mut c_void,
            null_mut_ptr,
            AttributeType::DataSize.into(),
            error,
        )
    };
    match size_result.into() {
        ReturnCode::Success => Ok(size),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Getting column data size",
        )),
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
fn determine_external_data_type(
    parameter: *mut OCIParam,
    error: *mut OCIError,
) -> Result<OciDataType, OciError> {
    let internal_data_type = column_internal_data_type(parameter, error)?;
    match internal_data_type {
        OciDataType::SqlVarChar => Ok(OciDataType::SqlVarChar),
        OciDataType::SqlNum => {
            let precision = column_data_precision(parameter, error)?;
            let scale = column_data_scale(parameter, error)?;
            if (precision != 0) && (scale == -127) {
                Ok(OciDataType::SqlFloat)
            } else {
                Ok(OciDataType::SqlInt)
            }
        }
        OciDataType::SqlChar => Ok(OciDataType::SqlChar),
        OciDataType::SqlDate | OciDataType::SqlTimestamp | OciDataType::SqlTimestampTz => {
            Ok(internal_data_type)
        }
        _ => panic!("Uknown external conversion."),
    }
}

fn column_internal_data_type(
    parameter: *mut OCIParam,
    error: *mut OCIError,
) -> Result<OciDataType, OciError> {
    let mut data_type: c_ushort = 0;
    let data_type_ptr: *mut c_ushort = &mut data_type;
    let null_mut_ptr = ptr::null_mut();
    let size_result = unsafe {
        OCIAttrGet(
            parameter as *mut c_void,
            DescriptorType::Parameter.into(),
            data_type_ptr as *mut c_void,
            null_mut_ptr,
            AttributeType::DataType.into(),
            error,
        )
    };
    match size_result.into() {
        ReturnCode::Success => Ok(data_type.into()),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Getting column data type",
        )),
    }
}

fn column_data_precision(
    parameter: *mut OCIParam,
    error: *mut OCIError,
) -> Result<c_short, OciError> {
    let mut precision: c_short = 0;
    let precision_ptr: *mut c_short = &mut precision;
    let null_mut_ptr = ptr::null_mut();
    let precision_result = unsafe {
        OCIAttrGet(
            parameter as *mut c_void,
            DescriptorType::Parameter.into(),
            precision_ptr as *mut c_void,
            null_mut_ptr,
            AttributeType::Precision.into(),
            error,
        )
    };
    match precision_result.into() {
        ReturnCode::Success => Ok(precision),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Getting column precision",
        )),
    }
}

fn column_data_scale(parameter: *mut OCIParam, error: *mut OCIError) -> Result<c_schar, OciError> {
    let mut scale: c_schar = 0;
    let scale_ptr: *mut c_schar = &mut scale;
    let null_mut_ptr = ptr::null_mut();
    let scale_result = unsafe {
        OCIAttrGet(
            parameter as *mut c_void,
            DescriptorType::Parameter.into(),
            scale_ptr as *mut c_void,
            null_mut_ptr,
            AttributeType::Scale.into(),
            error,
        )
    };
    match scale_result.into() {
        ReturnCode::Success => Ok(scale),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Getting column scale",
        )),
    }
}

fn allocate_parameter_handle(
    statement: *mut OCIStmt,
    error: *mut OCIError,
    position: c_uint,
) -> Result<*mut OCIParam, OciError> {
    let handle: *mut OCIParam = ptr::null_mut();
    let handle_result = unsafe {
        OCIParamGet(
            statement as *const c_void,
            HandleType::Statement.into(),
            error,
            &handle,
            position,
        )
    };
    match handle_result.into() {
        ReturnCode::Success => Ok(handle),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Allocating parameter handle",
        )),
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
        OCIAttrGet(
            statement as *mut c_void,
            HandleType::Statement.into(),
            nmb_cols_ptr as *mut c_void,
            null_mut_ptr,
            AttributeType::ParameterCount.into(),
            error,
        )
    };

    match column_result.into() {
        ReturnCode::Success => Ok(nmb_cols),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Getting number of columns",
        )),
    }
}

fn build_result_row(
    statement: *mut OCIStmt,
    error: *mut OCIError,
) -> Result<Option<Row>, OciError> {
    let column_count = number_of_columns(statement, error)?;
    let columns: Vec<Column> = (1..=column_count)
        .map(|position| Column::new(statement, error, position))
        .collect::<Result<Vec<Column>, _>>()?;

    match fetch_next_row(statement, error) {
        Ok(result) => match result {
            FetchResult::Data => (),
            FetchResult::NoData => return Ok(None),
        },
        Err(err) => return Err(err),
    }

    let sql_values: Result<Vec<_>, _> = columns
        .into_iter()
        .map(|col| col.create_sql_value())
        .collect();

    Ok(Some(Row::new(sql_values?)))
}

enum FetchResult {
    Data,
    NoData,
}

fn fetch_next_row(statement: *mut OCIStmt, error: *mut OCIError) -> Result<FetchResult, OciError> {
    let nrows = 1 as c_uint;
    let offset = 0 as c_int;
    let fetch_result = unsafe {
        OCIStmtFetch2(
            statement,
            error,
            nrows,
            FetchType::Next.into(),
            offset,
            EnvironmentMode::Default.into(),
        )
    };
    match fetch_result.into() {
        ReturnCode::Success => Ok(FetchResult::Data),
        ReturnCode::NoData => Ok(FetchResult::NoData),
        _ => Err(get_error(
            error as *mut c_void,
            HandleType::Error,
            "Fetching",
        )),
    }
}
