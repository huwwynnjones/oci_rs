use libc::{c_uint, c_int, c_void, c_uchar, size_t};

pub const OCI_DEFAULT: c_uint = 0;

pub const OCI_HTYPE_ENV: c_uint = 1;
pub const OCI_HTYPE_ERROR: c_uint = 2;
pub const OCI_HTYPE_SERVER: c_uint = 8;

pub const OCI_SUCCESS: c_int = 0;
pub const OCI_ERROR: c_int = -1;
pub const OCI_NO_DATA: c_int = 100;


#[derive(Debug)]
pub enum OCIEnv {}
#[derive(Debug)]
pub enum OCIServer {}

#[derive(Debug)]
pub enum ReturnCode {
    Success,
    Error,
    NoData,
}

pub trait ToReturnCode {
    fn to_return_code(&self) -> ReturnCode;
}

impl ToReturnCode for c_int {
    fn to_return_code(&self) -> ReturnCode {
        match *self {
            OCI_SUCCESS => ReturnCode::Success,
            OCI_ERROR => ReturnCode::Error,
            OCI_NO_DATA => ReturnCode::NoData,
            _ => ReturnCode::Error,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ErrorHandleType {
    Error = OCI_HTYPE_ERROR as isize,
    Environment = OCI_HTYPE_ENV as isize,
}

#[link(name="clntsh")]
extern "C" {
    /// Creates the environment handle. The signature has been changed to only
    /// allow null pointers for the user defined memory parameters. This means
    /// that user defined memory functions are not supported. I don't know how
    /// to specify function pointers in the signature but then send in null pointers
    /// when calling. Any attempt so far has been thwarted by the type system.
    /// # Safety
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
                        //xtramemsz: size_t,
                        xtramemsz: *const c_void,
                        //usrmempp: &*mut c_void)
                        usrmempp: *const c_void)
                        -> c_int;

    /// Frees a handle and deallocates the memory. Any child handles are automatically
    /// freed as well.
    /// See [Oracle docs](https://docs.oracle.com/database/122/
    /// LNOCI/handle-and-descriptor-functions.htm#LNOCI17135) for more info.
    pub fn OCIHandleFree(hndlp: *mut c_void, hnd_type: c_uint) -> c_int;

    pub fn OCIHandleAlloc(parenth: *const c_void,
                          hndlpp: &*mut c_void,
                          hnd_type: c_uint,
                          xtramem_sz: size_t,
                          usrmempp: &*mut c_void) -> c_int;

    pub fn OCIErrorGet(hndlp: *mut c_void,
                       recordno: c_uint,
                       sqlstate: *mut c_uchar,
                       errcodep: *mut c_int,
                       bufp: *mut c_uchar,
                       bufsiz: c_uint,
                       hnd_type: c_uint)
                       -> c_int;

}
