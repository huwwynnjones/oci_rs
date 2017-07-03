extern crate libc;
extern crate byteorder;
pub mod connection;
pub mod oci_error;
pub mod types;
pub mod row;
mod oci_bindings;

#[cfg(test)]
mod tests {
    use connection::Connection;
    const CONNECTION: &str = "localhost:1521/xe";
    const USER: &str = "oci_rs";
    const PASSWORD: &str = "test";

    #[test]
    #[allow(unused_variables)]
    fn create_connection() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
    }

    #[test]
    fn create_prepared_statement() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Books";
        let drop = match conn.create_prepared_statement(sql_drop) {
            Ok(s) => s,
            Err(err) => panic!("Failed to prepare drop Books: {}", err),
        };
        drop.execute().ok();
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

    #[test]
    fn bind() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Fruit";
        let drop = match conn.create_prepared_statement(sql_drop) {
            Ok(s) => s,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Fruit(FruitId integer, Name varchar(20))";
        let create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Fruit(FruitId, Name) VALUES(:id, :name)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        let name = String::from("Apple");
        let id: i64 = 22;
        if let Err(err) = insert.bind(&[&id, &name]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        let name = String::from("Pear");
        let id: i64 = 23;
        if let Err(err) = insert.bind(&[&id, &name]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        if let Err(err) = insert.bind(&[&24, &"Banana".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        if let Err(err) = insert.commit() {
            panic!("{}", err)
        }
    }

    #[test]
    fn query(){

        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Cars";
        let drop = match conn.create_prepared_statement(sql_drop) {
            Ok(s) => s,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Cars(CarId number(2), Name varchar(20))";
        let create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Cars(CarId, Name) VALUES('12', 'BMW')";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        let sql_query = "SELECT * FROM Cars";
        let mut select = match conn.create_prepared_statement(sql_query){
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = select.execute() {
            panic!("{}", err)
        }
        let result = match select.result() {
            Ok(res) => res,
            Err(err) => panic!("{}", err),
        };
        if result.is_empty(){
            panic!("Should not have an empty result")
        }
        let row = &result[0];
        let car_id: i64 = row[0].value().expect("Not an i64");
        //assert_eq!(car_id, 12);
        let car_name: String = row[1].value().expect("Not a string");
        assert_eq!(car_name, "BMW")
    }
}
