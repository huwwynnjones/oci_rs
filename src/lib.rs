#![warn(missing_docs)]
//! This crate provides a Rust wrapper to the [Oracle Call Interface][1] (OCI) library. 
//! The Oracle site describes OCI as a "...comprehensive, high performance, native C 
//! language interface to Oracle Database for custom or packaged applications...". 
//! 
//! # Overview
//! 
//! The OCI library is the original Oracle C API for interacting with their database. It is the one
//! that later versions of JDBC is built on for example. Recently Oracle has released a new API 
//! called the [Oracle Database Programming Interface for Drivers and Applications](https://githu
//! b.com/oracle/odpi) (ODPI-C) that is supposed to simplify use of OCI, but documentation for OCI 
//! is more extensive and therefore easier to build a wrapper on top of.
//! 
//! The OCI library is large and supports many use cases for interacting with a database. This
//! crate is currently concerned with support for executing SQL statements and so is limited when
//! compared to the whole of OCI.
//! 
//! The overall design will be familiar to anyone who has used Java's JDBC, Haskell's HDBC or
//! Rust's [postgres](https://crates.io/crates/postgres) crate. Indeed, most design decisions were
//! made based on reviewing the API of these libraries. 
//! 
//! The basics are simple: a [`Connection`](connection/struct.Connection.html) represents a 
//! connection to a database, this connection can be used to prepare one or more [`Statement`]
//! (statement/struct.Statement.html)s which are then used to execute SQL against the database. If 
//! there are results then they can be returned all at once or lazily via an iterator. Datatypes are 
//! represented using [`SqlValue`](types/enum.SqlValue.html) and allow type conversion from Oracle 
//! to Rust types.
//! 
//! ## Missing type conversions
//! 
//! Currently only `String`, `i64` and `f64` are supported. In Oracle terms this means that anything 
//! held in columns as `VARCHAR`, `VARCHAR2` and `Number` can be retrieved. As Oracle uses `Number` to
//! respresent all number types then this is less restricting that it first appears. More types
//! will be added.
//! 
//! # Setup
//! 
//! This crate is developed against version 12.2 of the OCI library. It is expected to work with 
//! 12.x.x but is not tested. The OCI client library needs to be installed on your machine and can be
//! downloaded [here](http://www.oracle.com/technetwork/database/features/instant-client/
//! index-097480.html). If you are on Linux then you are likely to need to tell the linker where
//! to find the files.
//! 
//! Adding this to my `.bashrc` file worked for me. The details will vary according to your distro,
//! mine is [OpenSuse](https://www.opensuse.org/).
//! 
//! ```ignore
//! export LIBRARY_PATH=$LIBRARY_PATH:/usr/lib/oracle/12.2/client64/lib/
//! ```
//! 
//! This crate has not been tested against Windows and so the setup will be different.
//! 
//! Testing has been done against a local installation of [Oracle 11g Express Edition](http://www.
//! oracle.com/technetwork/database/database-technologies/express-edition/overview/index.html). 
//! In order to run the crate tests then a local database needs to be
//! available on `localhost:1521/xe` with a user `oci_rs` and password `test`.
//! 
//! Add this to your `Cargo.toml`:
//! 
//! ```toml
//! [dependencies]
//! oci_rs = "0.1.0"
//! ```
//! and this to your crate root:
//! 
//! ```ignore
//! extern crate oci_rs;
//! ```
//! 
//! # Examples
//!
//! In the following example we will create a connection to a database and then create a table,
//! insert a couple of rows using bind variables and then execute a query to fetch them back again.
//! There is a lot of error handling needed. Every OCI function call can fail and so `Result` and
//! `Option` are used extensively. The below code takes the usual documentation shortcut of calling
//! `unwrap()` a lot but doing so in real client code will prove ill-fated. A database connection is
//! inherently unreliable as they mostly run on another machine.
//! 
//! ```rust
//! use oci_rs::connection::Connection;
//! 
//! let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
//! 
//! # let drop = conn.create_prepared_statement("DROP TABLE Toys").unwrap();
//! # drop.execute().ok();
//! 
//! // Create a table
//! let sql_create = "CREATE TABLE Toys (ToyId int,
//!                                      Name varchar(20),
//!                                      Price float)";
//! let create = conn.create_prepared_statement(sql_create).unwrap();
//! 
//! // Execute the create statement
//! create.execute().unwrap();
//! 
//! // Insert some values using bind variables
//! let sql_insert = "INSERT INTO Toys (ToyId, Name, Price) 
//!                   VALUES (:id, :name, :price)";
//! let mut insert = conn.create_prepared_statement(sql_insert).unwrap();
//! 
//! let values = [(1, "Barbie", 23.45), 
//!               (2, "Dinosaurs", -5.21)];
//! 
//! // Run through the list of values, bind them and executing the insert statement
//! for value in values.iter() {
//!     insert.bind(&[&value.0, &value.1, &value.2]).unwrap();
//!     insert.execute().unwrap()
//! }
//! 
//! // Create a query
//! let sql_select = "SELECT * FROM Toys
//!                   WHERE Name='Barbie'";
//! 
//! let mut select = conn.create_prepared_statement(sql_select).unwrap();
//! 
//! // Execute
//! select.execute().unwrap();
//! 
//! // Get the result set 
//! let result_set = select.result_set().unwrap();
//! let first_row = &result_set[0];
//! 
//! // Types are automatically converted
//! let id: i64 = first_row[0].value().unwrap();
//! let name: String = first_row[1].value().unwrap();
//! let price: f64 = first_row[2].value().unwrap();
//! 
//! assert_eq!(id, 1);
//! assert_eq!(name, "Barbie");
//! assert_eq!(price, 23.45);
//! 
//! ```
//! # OCI docs
//! 
//! Documentation for the underlying OCI library can be found [here](http://docs.oracle.com/
//! database/122/LNOCI/toc.htm) and error codes and there descriptions [here](https://docs.oracle.com/
//! database/122/ERRMG/toc.htm). The error descriptions are useful because they often contain
//! additional information that is not included in the text returned from the library.
//! 
//! [1]:  http://www.oracle.com/technetwork/database/features/oci/index-090945.html
extern crate libc;
extern crate byteorder;
/// Manages connections to a database.
/// 
/// The current implementation only supports a simple connection to the database. There is
/// one user session, but multiple statements can be created. Multiple connections can be created
/// safely because it defaults to setting the OCI environment mode as multi-threaded and 
/// therefore the OCI library takes care of concurrency. The cost of this is that a purely single
/// threaded client application might run slower.
/// 
/// More advanced connection options such as connection and statement pooling are not yet
/// available.
/// 
pub mod connection;
pub mod oci_error;
pub mod types;
pub mod row;
pub mod statement;
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
    fn query() {
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
        let sql_create = "CREATE TABLE Cars(CarId integer, Name varchar(20))";
        let create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Cars(CarId, Name) VALUES('12', 'BMW')";
        let insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        let sql_query = "SELECT * FROM Cars";
        let mut select = match conn.create_prepared_statement(sql_query) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = select.execute() {
            panic!("{}", err)
        }
        let result_set = match select.result_set() {
            Ok(res) => res,
            Err(err) => panic!("{}", err),
        };
        if result_set.is_empty() {
            panic!("Should not have an empty result")
        }
        let row = &result_set[0];
        let car_id: i64 = row[0].value().expect("Not an i64");
        assert_eq!(car_id, 12);
        let car_name: String = row[1].value().expect("Not a string");
        assert_eq!(car_name, "BMW")
    }

    #[test]
    fn multi_row_query() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Flowers";
        let drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Flowers(FlowerId integer, Name varchar(20))";
        let create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Flowers(FlowerId, Name) VALUES(:id, :name)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = insert.bind(&[&1, &"Rose".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        if let Err(err) = insert.bind(&[&2, &"Tulip".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        let sql_query = "SELECT * FROM Flowers";
        let mut select = match conn.create_prepared_statement(sql_query) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = select.execute() {
            panic!("{}", err)
        }
        let result_set = match select.result_set() {
            Ok(res) => res,
            Err(err) => panic!("{}", err),
        };
        if result_set.is_empty() {
            panic!("Should not have an empty result")
        }
        let pairs = [(1, "Rose"), (2, "Tulip")];
        for (index, pair) in pairs.iter().enumerate() {
            let row = &result_set[index];
            let flower_id: i64 = row[0].value().expect("Not an i64");
            let flower_name: String = row[1].value().expect("Not a string");
            assert_eq!(flower_id, pair.0);
            assert_eq!(flower_name, pair.1);
        }
    }

    #[test]
    fn lazy_multi_row_query() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Birds";
        let drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Birds(BirdId integer, Name varchar(20))";
        let create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Birds(BirdId, Name) VALUES(:id, :name)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = insert.bind(&[&1, &"Chafinch".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        if let Err(err) = insert.bind(&[&2, &"Eagle".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        let sql_query = "SELECT * FROM Birds";
        let select = match conn.create_prepared_statement(sql_query) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = select.execute() {
            panic!("{}", err)
        }
        let mut result_set = Vec::new();
        for row_result in select.lazy_result_set() {
            match row_result {
                Ok(row) => result_set.push(row),
                Err(err) => panic!("{}", err),
            }
        }
        if result_set.is_empty() {
            panic!("Should not have an empty result")
        }
        let pairs = [(1, "Chafinch"), (2, "Eagle")];
        for (index, pair) in pairs.iter().enumerate() {
            let row = &result_set[index];
            let bird_id: i64 = row[0].value().expect("Not an i64");
            let bird_name: String = row[1].value().expect("Not a string");
            assert_eq!(bird_id, pair.0);
            assert_eq!(bird_name, pair.1);
        }
    }

    #[test]
    fn number_conversion() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Sweets";
        let drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Sweets(SweetId integer, Name varchar(20), Price float)";
        let create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Sweets(SweetId, Name, Price) VALUES(:id, :name, :price)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        let values = [(1, "Toffee", 22.5), (2, "Haribo", -4.0), (3, "Gobstoppers", 34.5657)];
        for value in values.iter() {
            if let Err(err) = insert.bind(&[&value.0, &value.1.to_string(), &value.2]) {
                panic!("{}", err)
            }
            if let Err(err) = insert.execute() {
                panic!("{}", err)
            }
        }
        let sql_query = "SELECT * FROM Sweets";
        let mut query = match conn.create_prepared_statement(sql_query) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = query.execute() {
            panic!("{}", err)
        }
        let result_set = match query.result_set() {
            Ok(res) => res,
            Err(err) => panic!("{}", err),
        };
        if result_set.is_empty() {
            panic!("Should not have an empty result")
        }
        for (index, value) in values.iter().enumerate() {
            let row = &result_set[index];
            let sweet_id: i64 = row[0].value().expect("Not an i64");
            let sweet_name: String = row[1].value().expect("Not a string");
            let sweet_price: f64 = match row[2].value() {
                Some(p) => p,
                None => panic!("{:?}", row[2]),
            };
            assert_eq!(sweet_id, value.0);
            assert_eq!(sweet_name, value.1);
            assert_eq!(sweet_price, value.2);
        }
    }
}
