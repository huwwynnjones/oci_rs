use libc::{c_uint, c_int, c_void, c_uchar, size_t};

const OCI_DEFAULT: c_uint = 0;

const OCI_HTYPE_ENV: c_uint = 1;
const OCI_HTYPE_ERROR: c_uint = 2;
const OCI_HTYPE_SVCCTX: c_uint = 3;
const OCI_HTYPE_STMT: c_uint = 4;
const OCI_HTYPE_SERVER: c_uint = 8;
const OCI_HTYPE_SESSION: c_uint = 9;

const OCI_SUCCESS: c_int = 0;
const OCI_ERROR: c_int = -1;
const OCI_NO_DATA: c_int = 100;

const OCI_ATTR_SERVER: c_uint = 6;
const OCI_ATTR_USERNAME: c_uint = 22;
const OCI_ATTR_PASSWORD: c_uint = 23;
const OCI_ATTR_SESSION: c_uint = 7;

const OCI_CRED_RDBMS: c_uint = 1;

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
}

impl From<c_int> for ReturnCode {
    fn from(number: c_int) -> Self {
        match number {
            OCI_SUCCESS => ReturnCode::Success,
            OCI_ERROR => ReturnCode::Error,
            OCI_NO_DATA => ReturnCode::NoData,
            _ => ReturnCode::Error,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum HandleType {
    Error,
    Environment,
    Server,
    Service,
    Session,
    Statement,
}

impl From<HandleType> for c_uint {
    fn from(handle_type: HandleType) -> Self {
        match handle_type {
            HandleType::Error => OCI_HTYPE_ERROR,
            HandleType::Environment => OCI_HTYPE_ENV,
            HandleType::Server => OCI_HTYPE_SERVER,
            HandleType::Service => OCI_HTYPE_SVCCTX,
            HandleType::Session => OCI_HTYPE_SESSION,
            HandleType::Statement => OCI_HTYPE_STMT,
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

#[derive(Debug)]
pub enum AttributeType {
    Server,
    UserName,
    Password,
    Session,
}

impl From<AttributeType> for c_uint {
    fn from(attribute_type: AttributeType) -> Self {
        match attribute_type {
            AttributeType::Server => OCI_ATTR_SERVER,
            AttributeType::UserName => OCI_ATTR_USERNAME,
            AttributeType::Password => OCI_ATTR_PASSWORD,
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


}
