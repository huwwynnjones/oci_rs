use libc::{c_uint, c_int, c_void, c_uchar, size_t, c_ushort};

const OCI_DEFAULT: c_uint = 0;


const OCI_SUCCESS: c_int = 0;
const OCI_ERROR: c_int = -1;
const OCI_NO_DATA: c_int = 100;
const OCI_INVALID_HANDLE: c_int = -2;


const OCI_CRED_RDBMS: c_uint = 1;

const OCI_NTV_SYNTAX: c_uint = 1;

const SQLT_CHR: c_ushort = 1;
const SQLT_INT: c_ushort = 3;

#[derive(Debug)]
pub enum OCIEnv {}
#[derive(Debug)]
pub enum OCIServer {}
#[derive(Debug)]
pub enum OCIError {}
#[derive(Debug)]
pub enum OCISvcCtx {}
#[derive(Debug)]
pub enum OCISession {}
#[derive(Debug)]
pub enum OCIStmt {}
#[derive(Debug)]
pub enum OCISnapshot {}
#[derive(Debug)]
pub enum OCIBind {}

#[derive(Debug)]
pub enum EnvironmentMode {
    Default,
}
impl EnvironmentMode {
    pub fn to_environment_code(&self) -> c_uint {
        match *self {
            EnvironmentMode::Default => OCI_DEFAULT,
        }
    }
}

impl From<EnvironmentMode> for c_uint {
    fn from(mode: EnvironmentMode) -> Self {
        match mode {
            EnvironmentMode::Default => OCI_DEFAULT,
        }
    }
}

#[derive(Debug)]
pub enum ReturnCode {
    Success,
    Error,
    NoData,
    InvalidHandle,
}

impl From<c_int> for ReturnCode {
    fn from(number: c_int) -> Self {
        match number {
            OCI_SUCCESS => ReturnCode::Success,
            OCI_NO_DATA => ReturnCode::NoData,
            OCI_INVALID_HANDLE => ReturnCode::InvalidHandle,
            OCI_ERROR => ReturnCode::Error,
            _ => {
                panic!(format!("Found an unknown return code: {}, this should not happen.",
                               number))
            }
        }
    }
}

const OCI_HTYPE_ENV: c_uint = 1;
const OCI_HTYPE_ERROR: c_uint = 2;
const OCI_HTYPE_SVCCTX: c_uint = 3;
const OCI_HTYPE_STMT: c_uint = 4;
const OCI_HTYPE_SERVER: c_uint = 8;
const OCI_HTYPE_SESSION: c_uint = 9;

#[derive(Debug, Copy, Clone)]
pub enum HandleType {
    Environment,
    Error,
    Service,
    Statement,
    Server,
    Session,
}

impl From<HandleType> for c_uint {
    fn from(handle_type: HandleType) -> Self {
        match handle_type {
            HandleType::Environment => OCI_HTYPE_ENV,
            HandleType::Error => OCI_HTYPE_ERROR,
            HandleType::Service => OCI_HTYPE_SVCCTX,
            HandleType::Statement => OCI_HTYPE_STMT,
            HandleType::Server => OCI_HTYPE_SERVER,
            HandleType::Session => OCI_HTYPE_SESSION,
        }
    }
}

impl From<c_uint> for HandleType {
    fn from(number: c_uint) -> Self {
        match number {
            OCI_HTYPE_ENV => HandleType::Environment,
            OCI_HTYPE_ERROR => HandleType::Error,
            OCI_HTYPE_SVCCTX => HandleType::Service,
            OCI_HTYPE_STMT => HandleType::Statement,
            OCI_HTYPE_SERVER => HandleType::Server,
            OCI_HTYPE_SESSION => HandleType::Session,
            _ => {
                panic!(format!("Found an unknown handle type: {}, this should not happen.",
                               number))
            }
        }
    }
}

impl<'hnd> From<HandleType> for &'hnd str {
    fn from(handle_type: HandleType) -> Self {
        match handle_type {
            HandleType::Error => "Error handle",
            HandleType::Environment => "Environment handle",
            HandleType::Server => "Server handle",
            HandleType::Service => "Service handle",
            HandleType::Session => "Session handle",
            HandleType::Statement => "Statement handle",
        }
    }
}

const OCI_ATTR_SERVER: c_uint = 6;
const OCI_ATTR_USERNAME: c_uint = 22;
const OCI_ATTR_PASSWORD: c_uint = 23;
const OCI_ATTR_STMT: c_uint = 24;
const OCI_ATTR_SESSION: c_uint = 7;

#[derive(Debug)]
pub enum AttributeType {
    Server,
    UserName,
    Password,
    Statement,
    Session,
}

impl From<AttributeType> for c_uint {
    fn from(attribute_type: AttributeType) -> Self {
        match attribute_type {
            AttributeType::Server => OCI_ATTR_SERVER,
            AttributeType::UserName => OCI_ATTR_USERNAME,
            AttributeType::Password => OCI_ATTR_PASSWORD,
            AttributeType::Statement => OCI_ATTR_STMT,
            AttributeType::Session => OCI_ATTR_SESSION,
        }
    }
}

#[derive(Debug)]
pub enum CredentialsType {
    Rdbms,
}

impl From<CredentialsType> for c_uint {
    fn from(credentials_type: CredentialsType) -> Self {
        match credentials_type {
            CredentialsType::Rdbms => OCI_CRED_RDBMS,
        }
    }
}

#[derive(Debug)]
pub enum SyntaxType {
    Ntv,
}

impl From<SyntaxType> for c_uint {
    fn from(syntax_type: SyntaxType) -> Self {
        match syntax_type {
            SyntaxType::Ntv => OCI_NTV_SYNTAX,
        }
    }
}

#[derive(Debug)]
pub enum SqlType {
    SqlChar,
    SqlInt,
}

impl From<SqlType> for c_ushort {
    fn from(sql_type: SqlType) -> Self {
        match sql_type {
            SqlType::SqlChar => SQLT_CHR,
            SqlType::SqlInt => SQLT_INT,
        }
    }
}

const OCI_STMT_UNKNOWN: c_uint = 0;
const OCI_STMT_SELECT: c_uint = 1;
const OCI_STMT_UPDATE: c_uint = 2;
const OCI_STMT_DELETE: c_uint = 3;
const OCI_STMT_INSERT: c_uint = 4;
const OCI_STMT_CREATE: c_uint = 5;
const OCI_STMT_DROP: c_uint = 6;
const OCI_STMT_ALTER: c_uint = 7;
const OCI_STMT_BEGIN: c_uint = 8;
const OCI_STMT_DECLARE: c_uint = 9;

#[derive(Debug)]
pub enum StatementType {
    Unknown,
    Select,
    Update,
    Delete,
    Insert,
    Create,
    Drop,
    Alter,
    Begin,
    Declare,
}

impl From<StatementType> for c_uint {
    fn from(statement_type: StatementType) -> Self {
        match statement_type {
            StatementType::Unknown => OCI_STMT_UNKNOWN,
            StatementType::Select => OCI_STMT_SELECT,
            StatementType::Update => OCI_STMT_UPDATE,
            StatementType::Delete => OCI_STMT_DELETE,
            StatementType::Insert => OCI_STMT_INSERT,
            StatementType::Create => OCI_STMT_CREATE,
            StatementType::Drop => OCI_STMT_DROP,
            StatementType::Alter => OCI_STMT_ALTER,
            StatementType::Begin => OCI_STMT_BEGIN,
            StatementType::Declare => OCI_STMT_DECLARE,
        }
    }
}

impl From<c_uint> for StatementType {
    fn from(number: c_uint) -> Self {
        match number {
            OCI_STMT_UNKNOWN => StatementType::Unknown,
            OCI_STMT_SELECT => StatementType::Select,
            OCI_STMT_UPDATE => StatementType::Update,
            OCI_STMT_DELETE => StatementType::Delete,
            OCI_STMT_INSERT => StatementType::Insert,
            OCI_STMT_CREATE => StatementType::Create,
            OCI_STMT_DROP => StatementType::Drop,
            OCI_STMT_ALTER => StatementType::Alter,
            OCI_STMT_BEGIN => StatementType::Begin,
            OCI_STMT_DECLARE => StatementType::Declare,
            _ => {
                panic!(format!("Found an unknown statement type: {}, this should not happen.",
                               number))
            }
        }
    }
}

#[link(name="clntsh")]
extern "C" {
    /// Creates the environment handle. The signature has been changed to only
    /// allow null pointers for the user defined memory parameters. This means
    /// that user defined memory functions are not supported. I don't know how
    /// to specify function pointers in the signature but then send in null pointers
    /// when calling. Any attempt so far has been thwarted by the type system.
    ///
    /// # Safety
    ///
    /// C function so is unsafe.
    pub fn OCIEnvCreate(envhpp: &*mut OCIEnv,
                        mode: c_uint,
                        ctxp: *const c_void,
                        //maloc_cb: extern "C" fn(*const c_void, size_t) -> *const c_void,
                        maloc_cb: *const c_void,
                        //raloc_cb: extern "C" fn(*const c_void, *const c_void, size_t)
                        //                        -> *const c_void,
                        raloc_cb: *const c_void,
                        //mfree_cb: extern "C" fn(*const c_void, *const c_void) -> *const c_void,
                        mfree_cb: *const c_void,
                        xtramemsz: size_t,
                        //usrmempp: &*mut c_void)
                        usrmempp: *const c_void)
                        -> c_int;

    /// Frees a handle and deallocates the memory. Any child handles are automatically
    /// freed as well.
    /// See [Oracle docs](https://docs.oracle.com/database/122/
    /// LNOCI/handle-and-descriptor-functions.htm#LNOCI17135) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIHandleFree(hndlp: *mut c_void, hnd_type: c_uint) -> c_int;

    /// Allocates handles. As in OCIEnvCreate it allows user defined memory
    /// but I have effectively disabled that by setting the usrmempp parameter
    /// as a null pointer. Same problem, I don't know how to specifiy a function
    /// pointer by send in a null pointer when calling.
    /// See [Oracle docs](https://docs.oracle.com/database/122/
    /// LNOCI/handle-and-descriptor-functions.htm#LNOCI17134) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIHandleAlloc(parenth: *const c_void,
                          hndlpp: &*mut c_void,
                          hnd_type: c_uint,
                          xtramem_sz: size_t,
                          //usrmempp: &*mut c_void
                          usrmempp: *const c_void)
                          -> c_int;

    /// Gets an error record. The sqlstate parameter is unused.
    /// See [Oracle docs](https://docs.oracle.com/database/122/
    /// LNOCI/miscellaneous-functions.htm#GUID-4B99087C-74F6-498A-8310-D6645172390A) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIErrorGet(hndlp: *mut c_void,
                       recordno: c_uint,
                       sqlstate: *mut c_uchar,
                       errcodep: *mut c_int,
                       bufp: *mut c_uchar,
                       bufsiz: c_uint,
                       hnd_type: c_uint)
                       -> c_int;

    /// Connects to the database.
    /// See [Oracle docs](https://docs.oracle.com/database/122/
    /// LNOCI/connect-authorize-and-initialize-functions.htm#LNOCI17119) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIServerAttach(srvhp: *mut OCIServer,
                           errhp: *mut OCIError,
                           dblink: *const c_uchar,
                           dblink_len: c_int,
                           mode: c_uint)
                           -> c_int;

    /// Disconnects the database. Must be called during disconnection or else
    /// will leave zombie processes running in the OS.
    /// See [Oracle docs](https://docs.oracle.com/database/122/
    /// LNOCI/connect-authorize-and-initialize-functions.htm#LNOCI17121) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIServerDetach(srvhp: *mut OCIServer, errhp: *mut OCIError, mode: c_uint) -> c_int;

    /// Sets the value of an attribute of a handle, e.g. username in session
    /// handle.
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// handle-and-descriptor-functions.htm#GUID-3741D7BD-7652-4D7A-8813-AC2AEA8D3B03)
    /// for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIAttrSet(trgthndlp: *mut c_void,
                      trghndltyp: c_uint,
                      attributep: *mut c_void,
                      size: c_uint,
                      attrtype: c_uint,
                      errhp: *mut OCIError)
                      -> c_int;

    /// Gets the value of an attribute of a handle.
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// handle-and-descriptor-functions.htm#LNOCI17130) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIAttrGet(trgthndlp: *const c_void,
                      trghndltyp: c_uint,
                      attributep: *mut c_void,
                      sizep: *mut c_uint,
                      attrtype: c_uint,
                      errhp: *mut OCIError)
                      -> c_int;


    /// Creates and starts a user session.
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// connect-authorize-and-initialize-functions.htm#GUID-31B1FDB3-056E-4AF9-9B89-8DA6AA156947)
    /// for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCISessionBegin(svchp: *mut OCISvcCtx,
                           errhp: *mut OCIError,
                           userhp: *mut OCISession,
                           credt: c_uint,
                           mode: c_uint)
                           -> c_int;

    /// Stops a user session.
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// connect-authorize-and-initialize-functions.htm#LNOCI17123) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCISessionEnd(svchp: *mut OCISvcCtx,
                         errhp: *mut OCIError,
                         userhp: *mut OCISession,
                         mode: c_uint)
                         -> c_int;

    /// Prepares a SQL or PL/SQL statement for execution. The user has the option of using
    /// the statement cache, if it has been enabled.
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// statement-functions.htm#LNOCI17168) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIStmtPrepare2(svchp: *mut OCISvcCtx,
                           stmthp: &*mut OCIStmt,
                           errhp: *mut OCIError,
                           stmttext: *const c_uchar,
                           stmt_len: c_uint,
                           key: *const c_uchar,
                           keylen: c_uint,
                           language: c_uint,
                           mode: c_uint)
                           -> c_int;

    /// Releases the statement handle obtained by a call to OCIStmtPrepare2().
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// statement-functions.htm#LNOCI17169) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIStmtRelease(stmthp: *mut OCIStmt,
                          errhp: *mut OCIError,
                          key: *const c_uchar,
                          keylen: c_uint,
                          mode: c_uint)
                          -> c_int;

    /// Executes a statement.
    /// See [Oracle docs](https://docs.oracle.com/database/122/LNOCI/
    /// statement-functions.htm#LNOCI17163) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIStmtExecute(svchp: *mut OCISvcCtx,
                          stmtp: *mut OCIStmt,
                          errhp: *mut OCIError,
                          iters: c_uint,
                          rowoff: c_uint,
                          snap_in: *const OCISnapshot,
                          snap_out: *mut OCISnapshot,
                          mode: c_uint)
                          -> c_int;

    /// Commits the transaction associated with a specified service context.
    /// See [Oracle docs](https://docs.oracle.com/cd/E11882_01/appdev.112/e10646/
    /// oci17msc006.htm#LNOCI13112) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCITransCommit(svchp: *mut OCISvcCtx, errhp: *mut OCIError, flags: c_uint) -> c_int;

    /// Creates an association between a program variable and a placeholder in a SQL statement
    /// or PL/SQL block.
    /// See [Oracle docs](http://docs.oracle.com/database/122/LNOCI/
    /// bind-define-describe-functions.htm#LNOCI17141) for more info.
    ///
    /// # Safety
    ///
    /// Unsafe C
    pub fn OCIBindByPos(stmtp: *mut OCIStmt,
                        bindpp: &*mut OCIBind,
                        errhp: *mut OCIError,
                        position: c_uint,
                        valuep: *mut c_void,
                        value_sz: c_int,
                        dty: c_ushort,
                        indp: *mut c_void,
                        alenp: *mut c_ushort,
                        rcodep: *mut c_ushort,
                        maxarr_len: c_uint,
                        curelep: *mut c_uint,
                        mode: c_uint)
                        -> c_int;



}
