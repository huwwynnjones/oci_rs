extern crate libc;
pub mod connection;
pub mod oci_error;
mod oci_bindings;

#[cfg(test)]
mod tests {
    use connection::Connection;

    #[test]
    fn create_connection() {
        let conn = match Connection::new("localhost:1521/xe",
                                         "huw", "morgen.Luc") {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
    }

    #[test]
    fn create_prepared_statement(){
        let conn = match Connection::new("localhost:1521/xe",
                                         "huw", "morgen.Luc") {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_create = "CREATE TABLE Books(BookID int, Name varchar(200))";
        let stmt = match conn.create_prepared_statement(sql_create) {
            Ok(s) => s,
            Err(err) => panic!("Failed to create a statement: {}", err),
        };
        if let Err(err) = stmt.execute() {
            panic!("Failed to execute: {}", err)
        }
        if let Err(err) = stmt.commit() {
            panic!("Failed to commit: {}", err)
        }
    }
}
