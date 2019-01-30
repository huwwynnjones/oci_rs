use libc::{c_uint, c_void};
use crate::oci_bindings::{AttributeType, HandleType, OCIAttrSet, OCIError, ReturnCode};
use crate::oci_error::{get_error, OciError};

/// Set handle attribute
pub fn set_handle_attribute(
    handle: *mut c_void,
    handle_type: HandleType,
    attribute_handle: *mut c_void,
    size: c_uint,
    attribute_type: AttributeType,
    error_handle: *mut OCIError,
    error_description: &str,
) -> Result<(), OciError> {
    let attr_set_result = unsafe {
        OCIAttrSet(
            handle,
            handle_type.into(),
            attribute_handle,
            size,
            attribute_type.into(),
            error_handle,
        )
    };
    match attr_set_result.into() {
        ReturnCode::Success => Ok(()),
        _ => Err(get_error(
            error_handle as *mut c_void,
            HandleType::Error,
            error_description,
        )),
    }
}
