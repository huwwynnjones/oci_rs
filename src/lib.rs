extern crate libc;
pub mod connection;
pub mod oci_error;
mod oci_bindings;

#[cfg(test)]
mod tests {
    use connection::Connection;

    #[test]
    fn create_connection() {
        let conn = match Connection::new() {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
    }
}
