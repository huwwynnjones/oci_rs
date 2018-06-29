#![warn(missing_docs)]
//! This crate provides a Rust wrapper to the [Oracle Call Interface][1] (OCI) library.
//! The Oracle site describes OCI as a "...comprehensive, high performance, native C
//! language interface to Oracle Database for custom or packaged applications...".
//!
//! # Overview
//!
//! The OCI library is the original Oracle C API for interacting with their database.
//! It is one of the driver options available in JDBC. Recently Oracle released a new API
//! called the [Oracle Database Programming Interface for Drivers and Applications][2] (ODPI-C)
//! that is supposed to simplify use of OCI, however the documentation for OCI
//! is more extensive and therefore easier to build a wrapper on top of.
//!
//! The OCI library is large and supports many use cases for interacting with a database. This
//! crate is currently concerned with support for executing SQL statements and so is limited when
//! compared to the whole of OCI.
//!
//! The overall design will be familiar to anyone who has used Java's JDBC, Haskell's HDBC or
//! Rust's [postgres][3] crate. Indeed, most design decisions were made based on reviewing the API
//! of these libraries.
//!
//! The basics are simple: a [`Connection`][4] represents a connection to a database, this
//! connection can be used to prepare one or more [`Statement`][5]s which are then used to
//! execute SQL against the database. If there are results then they can be returned all at
//! once or lazily via an iterator.
//! Datatypes are represented using [`SqlValue`][6] and allow type conversion from Oracle
//! to Rust types.
//!
//! ## Missing type conversions
//!
//! The following conversions are supported to/from Oracle SQL types to Rust types.
//! As Oracle uses `NUMBER` to represent all number types then all integer and floating point
//! types convert to it. Smaller integers or floats needed on the Rust side can be downcast.
//!
//! | Oracle SQL type          | Rust type               |
//! |--------------------------|-------------------------|
//! | VARCHAR                  | `String`                |
//! | VARCHAR2                 | `String`                |
//! | CHAR                     | `String`                |
//! | NUMBER                   | `i64`, `f64`            |
//! | DATE                     | `Date<Utc>`             |
//! | TIMESTAMP                | `DateTime<Utc>`         |
//! | TIMESTAMP WITH TIME ZONE | `DateTime<FixedOffset>` |
//!
//! Over time more types will be added.
//!
//! # Setup
//!
//! This crate is developed against version 12.2 of the OCI library. It is expected to work with
//! 12.x.x but is not tested. The OCI client library needs to be installed on your machine and
//! can be downloaded [here][7].
//!
//! If you are on Linux then you are likely to need to tell the linker where
//! to find the files. Adding this to my `.bashrc` file worked for me, however the details may vary
//! according to your distro. The below works on [OpenSuse][8].
//!
//! ```text
//! export LIBRARY_PATH=$LIBRARY_PATH:/usr/lib/oracle/12.2/client64/lib/
//! ```
//! You can build this crate on Windows hosts using the `windows-gnu` toolchain. The only requirement
//! for this is that `oci.dll` is on the PATH.
//!
//! This crate has been briefly tested against Windows but difficulties were faced.
//! The OCI library is named differently and so updates will be needed in the bindings to make it
//! compile. Once I can get chance to work out how to even build this using Visual Studio on
//! Windows, this will be addressed.
//!
//! Testing has been done against a local installation of [Oracle 11g Express Edition][9].
//! In order to run the crate tests then a local database needs to be
//! available on `localhost:1521/xe` with a user `oci_rs` and password `test`.
//!
//! Note that users of Debian based systems will face a lot of bother using Oracle databases
//! locally. It does not install easily due to lack of official packages and the use of Alien will
//! not help much. There are lots of complicated instructions available on the internet for how to
//! get it to work, however the easiest is to run it in a [Docker container][13]. I have switched
//! to [Ubuntu][12] and have had to resort to using Docker.
//!
//! In order to use `oci_rs` add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! oci_rs = "0.7.0"
//! ```
//! and this to your crate root:
//!
//! ```rust
//! extern crate oci_rs;
//! ```
//!
//! # Examples
//!
//! In the following example we will create a connection to a database and then create a table,
//! insert a couple of rows using bind variables and then execute a query to fetch them back again.
//! There is a lot of error handling needed. Every OCI function call can fail and so `Result` and
//! `Option` are used extensively. The below code takes the usual documentation shortcut of calling
//! `unwrap()` a lot but doing so in real client code will prove ill-fated. Any remote database
//! connection is
//! inherently unreliable.
//!
//! ```rust
//! use oci_rs::connection::Connection;
//!
//! let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
//!
//! # let mut drop = conn.create_prepared_statement("DROP TABLE Toys").unwrap();
//! # drop.execute().ok();
//!
//! // Create a table
//! let sql_create = "CREATE TABLE Toys (ToyId int,
//!                                      Name varchar(20),
//!                                      Price float)";
//! let mut create = conn.create_prepared_statement(sql_create).unwrap();
//!
//! // Execute the create statement
//! create.execute().unwrap();
//!
//! // Commit in case we lose connection (an abnormal disconnection would result
//! // in an automatic roll-back.)
//! create.commit().unwrap();
//!
//! // Insert some values using bind variables
//! let sql_insert = "INSERT INTO Toys (ToyId, Name, Price)
//!                   VALUES (:id, :name, :price)";
//! let mut insert = conn.create_prepared_statement(sql_insert).unwrap();
//!
//! let values = [(1, "Barbie", 23.45),
//!               (2, "Dinosaurs", -5.21)];
//!
//! // Run through the list of values, bind them and execute the statement
//! for value in values.iter() {
//!     insert.bind(&[&value.0, &value.1, &value.2]).unwrap();
//!     insert.execute().unwrap()
//! }
//!
//! insert.commit().unwrap();
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
//! assert_eq!(result_set.len(), 1);
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
//! Documentation for the underlying OCI library can be found [here][10] and error codes and their
//! descriptions [here][11]. The error descriptions are useful because they often contain
//! additional information that is not included in the text returned from the library.
//!
//! [1]: http://www.oracle.com/technetwork/database/features/oci/index-090945.html
//! [2]: https://github.com/oracle/odpi
//! [3]: https://crates.io/crates/postgres
//! [4]: connection/struct.Connection.html
//! [5]: statement/struct.Statement.html
//! [6]: types/enum.SqlValue.html
//! [7]: http://www.oracle.com/technetwork/database/features/instant-client/index-097480.html
//! [8]: https://www.opensuse.org/
//! [9]: http://www.oracle.com/technetwork/database/database-technologies/express-edition/
//! overview/index.html
//! [10]: http://docs.oracle.com/database/122/LNOCI/toc.htm
//! [11]: https://docs.oracle.com/database/122/ERRMG/toc.htm
//! [12]: https://www.ubuntu.com/
//! [13]: https://github.com/wnameless/docker-oracle-xe-11g
//!

extern crate byteorder;
extern crate chrono;
extern crate libc;

/// Connections to a database.
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
/// # Examples
///
/// Connection and statement creation with error handling:
///
/// ```rust
/// use oci_rs::connection::Connection;
///
/// let connection = match Connection::new("localhost:1521/xe",
///                                        "oci_rs",
///                                        "test") {
///     Ok(conn) => conn,
///     Err(err) => panic!("Failed to create a connection: {}", err),
/// };
///
/// let sql_create = "CREATE TABLE Cats (CatId INTEGER,
///                                      Name VARCHAR(20))";
///
/// let create = match connection.create_prepared_statement(sql_create) {
///     Ok(stmt) => stmt,
///     Err(err) => panic!("Failed to create a statement: {}", err),
/// };
/// ```
///
pub mod connection;

/// Errors.
///
/// Any errors arising from interaction with the OCI library will be returned as an `OciError`. All
/// Oracle errors will be returned as the `OciError::Oracle` type. The Oracle error
/// code and description can be seen through this.
///
/// # Examples
///
/// Here is an example of how an Oracle error will reach you if you choose to display it:
///
/// ```rust,should_panic
/// use oci_rs::connection::Connection;
///
/// let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
///
/// // Create a table
/// let sql_create = "CREATE TABLE BrokenToys (ToyId INT,
///                                            Name VARCHAR(20),
///                                            Price SINK)";
/// let mut create = conn.create_prepared_statement(sql_create).unwrap();
/// if let Err(err) = create.execute() {
///     panic!("Execution failed: {}", err)
/// }
/// ```
///
/// The above code will produce the following (this specific output comes from
/// running this documentation test without the `should_panic` attribute):
///
/// ```text
/// thread 'main' panicked at 'Execution failed: Executing statement
/// Error number: 1
/// Error code: ORA-902
/// Error text: ORA-00902: invalid datatype
/// ', <anon>:13
/// note: Run with `RUST_BACKTRACE=1` for a backtrace.
/// ```
/// In this case "SINK" is not a valid SQL data type.
///
/// Note that there might be more than one error and in such cases all errors will be listed, this
/// is why there is an error number.
///
pub mod oci_error;

/// Types used in conversion between OCI and Rust types.
///
/// This module provides a type `SqlValue` and two traits `ToSqlValue` and `FromSqlValue` that
/// allow the underlying OCI types to be converted into Rust types. They do not map exactly to the
/// corresponding SQL standard types.
///
/// For number types in particular there are less `SqlValue`s than SQL types. Inside Oracle all
/// numbers are stored as a `NUMBER`. This is an Oracle format that can handle all integer and
/// float values with a precision of 38 digits. Regardless of whether the SQL statement specifies
/// an `INTEGER` or `FLOAT` or `LONG`, Oracle will store it as a `NUMBER`. The OCI library then
/// allows you
/// to convert it into any numeric type you like, but that forces you to explicitly state the type
/// of the columns when retrieving the values. To avoid this, this crate makes some executive
/// decisions based on the `NUMBER` value. As per the OCI documentation the basic type of a number
/// can be
/// determined by the scale and precision of the `NUMBER` value. If the precision is non-zero and
/// scale is -127 then the number is a `FLOAT` otherwise we can consider it an `INTEGER`.
/// So, according to this logic the caller will receive either `SqlValue::Integer` or
/// `SqlValue::Float`.
/// These two variants contain an `i64` and `f64` respectively. If a smaller type is needed in
/// Rust code,
/// then further conversions can be made. This appears to be sufficient to allow retrieval of data
/// in
/// queries, without having specify column types on the Rust side ahead of time.
///
/// Note: Oracle also supports types known as `BINARY_FLOAT` and `BINARY_DOUBLE`. These can also be
/// used to store numbers inside the database as an alternative to `NUMBER`. They are not currently
/// supported.
///
/// The traits allow conversion to and from Rust types into `SqlValue`.
///
/// ## Type conversions
///
/// | Oracle SQL type          | Rust type               |
/// |--------------------------|-------------------------|
/// | VARCHAR                  | `String`                |
/// | VARCHAR2                 | `String`                |
/// | CHAR                     | `String`                |
/// | NUMBER                   | `i64`, `f64`            |
/// | DATE                     | `Date<Utc>`             |
/// | TIMESTAMP                | `DateTime<Utc>`         |
/// | TIMESTAMP WITH TIME ZONE | `DateTime<FixedOffset>` |
///
/// # Examples
///
/// This example highlights the automatic conversion. If it is confusing then I suggest reading
/// [Communicating Intent][1] as it explains very well how Rust's trait system makes this work,
/// the [`postgres`][2] crate also makes use of the same process to
/// convert the column values in a result row into Rust. This crate copies `postgres`'s approach
/// except that it makes use of an intermediary `SqlValue` instead of returning a trait. I think
/// that it is fair to argue that `SqlValue` is not needed, `postgres` skips such an intermediary
/// value, but using it simplifies the current implementation.
///
/// ```rust
/// use oci_rs::connection::Connection;
///
/// let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
/// # let mut drop = conn.create_prepared_statement("DROP TABLE Men").unwrap();
/// # drop.execute().ok();
///
/// // Create a table
/// let sql_create = "CREATE TABLE Men (ManId INTEGER,
///                                     Name VARCHAR2(20),
///                                     Height FLOAT)";
///
/// let mut create = conn.create_prepared_statement(sql_create).unwrap();
///
/// // Execute the create statement
/// create.execute().unwrap();
///
/// // Commit in case we lose connection (an abnormal disconnection would result
/// // in an automatic roll-back.)
/// create.commit().unwrap();
///
/// // Insert some values
/// let sql_insert = "INSERT INTO Men (ManId, Name, Height)
///                   VALUES (1, 'Roger', 183.4)";
///  let mut insert = conn.create_prepared_statement(sql_insert).unwrap();
///
/// insert.execute().unwrap();
/// insert.commit().unwrap();
///
/// // Create a query
/// let sql_select = "SELECT * FROM Men";
///
/// let mut select = conn.create_prepared_statement(sql_select).unwrap();
///
/// // Execute
/// select.execute().unwrap();
///
/// // Get the result set
/// let result_set = select.result_set().unwrap();
/// assert_eq!(result_set.len(), 1);
/// let first_row = &result_set[0];
///
/// // Types are automatically converted
/// let id: i64 = first_row[0].value().unwrap();
/// let name: String = first_row[1].value().unwrap();
/// let height: f64 = first_row[2].value().unwrap();
///
/// assert_eq!(id, 1);
/// assert_eq!(name, "Roger");
/// assert_eq!(height, 183.4);
///
/// // Integer and Float can also be turned into Strings
/// let id_as_string: String = first_row[0].value().unwrap();
/// let height_as_string: String = first_row[2].value().unwrap();
///
/// assert_eq!(id_as_string, "1");
/// assert_eq!(height_as_string, "183.4");
/// ```
///
/// [1]: https://github.com/jaheba/stuff/blob/master/communicating_intent.md
/// [2]: https://crates.io/crates/postgres
pub mod types;

/// Rows of data returned from a query
///
/// A `Row` represents a row of data returned from a SQL query. Internally it holds the columns
/// and their values. It implements the `Index` trait and so columns can be accessed via an index
/// number.
///
pub mod row;

mod common;
mod oci_bindings;
/// SQL statements run against the database.
///
/// `Statement`s are created to run a SQL Statement against a database. They prepare the statement
/// for execution and allow bind variables to be set. If there are results then these these can be
/// returned all in one go or lazily through an iterator.
///
/// # Overview
///
/// The process is as follows:
///
/// 1. Create a `Statement` from a connection with a given SQL statement. This will create a
///    prepared statement on the Oracle side.
/// 2. If the SQL contains bind variable placeholders then these values should now be set via a
///    call to `.bind`. Although OCI supports both positional and named bind variables, only
///    positional are curently support by `Statement`. Oracle uses the form `:name` where `name`is
///    the bind variable.
/// 3. Execute the statement.
/// 4. Commit the transaction if data was changed. Oracle implicitly creates a transaction when data
///    is changed and commits automatically with a normal session close and log-off. If we
///    disconnect abnormally however, a rollback is initiated.
/// 5. If there are results i.e. it was a `SELECT` statement, then fetch the results. The entire
///    result set can be returned as a `Vec<Row>` or instead an iterator can be used to return the
///    `Row`s one by one. These are fetched from OCI by the iterator as needed.
///
/// A connection can create multiple `Statement`s. In the examples in this document there is
/// usually one for each of the `DROP`, `CREATE`, `INSERT` and `SELECT` SQL statements used in the
/// examples.
///
/// # Examples
///
/// We will run through the above process to create a table, add some values and then return them
/// lazily. Every OCI call can fail and the below example avoids handling errors in order to make
/// the example easier to read, notice how many `.unwrap`s are here.
///
/// ```rust
/// use oci_rs::connection::Connection;
/// use oci_rs::row::Row;
///
/// let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();
/// # let mut drop = conn.create_prepared_statement("DROP TABLE Cities").unwrap();
/// # drop.execute().ok();
///
/// // Create a table
/// let sql_create = "CREATE TABLE Cities (CityId INTEGER,
///                                        Name VARCHAR(20))";
///
/// let mut create = conn.create_prepared_statement(sql_create).unwrap();
///
/// // Execute the create statement
/// create.execute().unwrap();
///
/// // Commit in case we lose connection (an abnormal disconnection would result
/// // in an automatic roll-back.)
/// create.commit().unwrap();
///
/// // Insert some values using bind variables
/// let sql_insert = "INSERT INTO Cities (CityId, Name)
///                   VALUES (:id, :name)";
/// let mut insert = conn.create_prepared_statement(sql_insert).unwrap();
///
/// let values = vec![(1, "Paris"),
///                   (2, "London"),
///                   (3, "Hamburg"),
///                   (4, "Miami")];
///
/// // Run through the list of values, bind them and execute the statement
/// for value in values.iter() {
///     insert.bind(&[&value.0, &value.1]).unwrap();
///     insert.execute().unwrap()
/// }
///
/// insert.commit().unwrap();
///
/// // Create a query
/// let sql_select = "SELECT * FROM Cities";
///
/// let mut select = conn.create_prepared_statement(sql_select).unwrap();
///
/// // Execute
/// select.execute().unwrap();
///
/// // Get the result set row by row from an iterator
/// for (index, row_result) in select.lazy_result_set().enumerate(){
///     let row = row_result.unwrap();
///     let city_id: i64 = row[0].value().unwrap();
///     let city_name: String = row[1].value().unwrap();
///     assert_eq!(city_id, values[index].0);
///     assert_eq!(city_name, values[index].1);
/// }
///
/// // Or perhaps something a bit more convoluted just to make use of iterator adapters
///
/// // Execute again to get fresh results
/// select.execute().unwrap();
///
/// // Get cities containing an 'a':
/// let results: Vec<String> = select.lazy_result_set()
///                                  .map(|row_result| {
///                                           let row = row_result.unwrap();
///                                           row[1].value::<String>().unwrap()
///                                       })
///                                  .filter(|city| city.contains("a"))
///                                  .collect();
///
/// let correct_result = vec!["Paris".to_string(),
///                           "Hamburg".to_string(),
///                           "Miami".to_string()];
/// assert_eq!(results, correct_result);
/// ```
/// The final example is a bit awkard because we have `Result`s and `Option`s to deal with
/// (or ignored as in this case) but it is added as a reminder that iterator methods can be used.
///
pub mod statement;

#[cfg(test)]
mod tests {
    use chrono::{Date, DateTime, FixedOffset, TimeZone, Timelike, Utc};
    use connection::Connection;
    use oci_error::OciError;
    const CONNECTION: &str = "localhost:1521/xe";
    const BAD_CONNECTION: &str = "localhost:1521/xp";
    const USER: &str = "oci_rs";
    const PASSWORD: &str = "test";
    const BAD_PASSWORD: &str = "toast";

    #[test]
    #[allow(unused_variables)]
    fn create_connection() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
    }

    #[test]
    #[allow(unused_variables)]
    #[should_panic]
    fn create_connection_with_bad_password() {
        let conn = match Connection::new(CONNECTION, USER, BAD_PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
    }

    #[test]
    #[allow(unused_variables)]
    fn create_connection_with_bad_connection_string() {
        let error = match Connection::new(BAD_CONNECTION, USER, PASSWORD) {
            Ok(conn) => panic!("Should not have been able to create a connection, test is wrong."),
            Err(err) => err,
        };
        let code = match error {
            OciError::Oracle(ref error_record) => &error_record.error_records()[0].0,
            OciError::Conversion(_) => {
                panic!("Should not have found a conversion error, test is wrong.")
            }
        };
        let tns_listener_error: i32 = 12514;
        assert_eq!(&tns_listener_error, code)
    }

    #[test]
    fn create_prepared_statement() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Books";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(s) => s,
            Err(err) => panic!("Failed to prepare drop Books: {}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Books(BookID int, Name varchar(200))";
        let mut stmt = match conn.create_prepared_statement(sql_create) {
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
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(s) => s,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Fruit(FruitId integer, Name varchar(20))";
        let mut create = match conn.create_prepared_statement(sql_create) {
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
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(s) => s,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Cars(CarId integer, Name varchar(20))";
        let mut create = match conn.create_prepared_statement(sql_create) {
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
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Flowers(FlowerId integer, Name varchar(20))";
        let mut create = match conn.create_prepared_statement(sql_create) {
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
    fn multi_row_query_with_prefetch() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Words";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Words(Word varchar(100))";
        let mut create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Words(Word) VALUES(:word)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };

        let words = ["Podgy", "Rumpy", "Wiggle", "Ponder", "Spice", "Constable"];

        for word in words.iter() {
            if let Err(err) = insert.bind(&[word]) {
                panic!("{}", err)
            }
            if let Err(err) = insert.execute() {
                panic!("{}", err)
            }
        }

        let sql_query = "SELECT * FROM Words";
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

        for row in result_set {
            let word: String = row[0].value().unwrap();
            assert!(words.contains(&word.as_ref()));
        }
    }

    #[test]
    fn lazy_multi_row_query() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Birds";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Birds(BirdId integer, Name varchar(20))";
        let mut create = match conn.create_prepared_statement(sql_create) {
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
        let mut select = match conn.create_prepared_statement(sql_query) {
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
    #[should_panic]
    fn lazy_multi_row_query_repeat_call() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Fish";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Fish(FishId integer, Name varchar(20))";
        let mut create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("{}", err)
        }
        let sql_insert = "INSERT INTO Fish(FishId, Name) VALUES(:id, :name)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = insert.bind(&[&1, &"Tuna".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        if let Err(err) = insert.bind(&[&2, &"Salmon".to_string()]) {
            panic!("{}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("{}", err)
        }
        let sql_query = "SELECT * FROM Fish";
        let mut select = match conn.create_prepared_statement(sql_query) {
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
        let pairs = [(1, "Tuna"), (2, "Salmon")];
        for (index, pair) in pairs.iter().enumerate() {
            let row = &result_set[index];
            let bird_id: i64 = row[0].value().expect("Not an i64");
            let bird_name: String = row[1].value().expect("Not a string");
            assert_eq!(bird_id, pair.0);
            assert_eq!(bird_name, pair.1);
        }

        let mut repeat_result_set = Vec::new();
        for row_result in select.lazy_result_set() {
            match row_result {
                Ok(row) => repeat_result_set.push(row),
                Err(err) => panic!("{}", err),
            }
        }
    }

    #[test]
    fn number_conversion() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Sweets";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Sweets(SweetId integer, Name varchar(20), Price float)";
        let mut create = match conn.create_prepared_statement(sql_create) {
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
        let values = [
            (1, "Toffee", 22.5),
            (2, "Haribo", -4.0),
            (3, "Gobstoppers", 34.5657),
        ];
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

    #[test]
    fn date_in_oracle_binary_format() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };

        let sql_drop = "DROP TABLE Birthdays";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();

        let sql_create = "CREATE TABLE Birthdays(id INTEGER,
                                                 Name VARCHAR2(200),
                                                 Birthday DATE)";
        let mut create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("Couldn't execute create Birthdays: {}", err)
        }

        let sql_insert = "INSERT INTO Birthdays(Id, Name, Birthday)
                          VALUES(:id, :name, :birthday)";
        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("Cannot create insert for Birthdays: {}", err),
        };

        let id = 1;
        let name = "Roger";
        let birthday = Utc.ymd(2006, 10, 1);

        if let Err(err) = insert.bind(&[&id, &name, &birthday]) {
            panic!("Cannot bind for insert to Birthdays: {}", err)
        }
        if let Err(err) = insert.execute() {
            panic!("Couldn't execute insert into Birthdays: {}", err)
        }

        let sql_select = "SELECT * FROM Birthdays";
        let mut select = match conn.create_prepared_statement(sql_select) {
            Ok(stmt) => stmt,
            Err(err) => panic!("Couldn't create select for Birthdays: {}", err),
        };
        if let Err(err) = select.execute() {
            panic!("Couldn't execute select for Birthdays: {}", err)
        }

        let result_set = match select.result_set() {
            Ok(res) => res,
            Err(err) => panic!("{}", err),
        };

        if result_set.is_empty() {
            panic!("Should not have an empty result")
        }

        let first_row = &result_set[0];

        let date: Date<Utc> = first_row[2].value().unwrap();
        assert_eq!(date, birthday);
    }

    ///Testing timezone conversions
    ///
    #[test]
    fn timezone_conversions() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Times";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Times(TimeId INTEGER,
                                             WorldTime TIMESTAMP(9) WITH TIME ZONE)";
        let mut create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("Couldn't execute create Times: {}", err)
        }

        let sql_insert = "INSERT INTO Times(TimeId, WorldTime)
                          VALUES(:id, :time)";

        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("Cannot create insert for Times: {}", err),
        };

        let utc = Utc.ymd(2018, 01, 05).and_hms(19, 25, 30);
        let utc_tz = utc.with_timezone(&FixedOffset::east(0));
        let east_5 = utc.with_timezone(&FixedOffset::east(5 * 3600));
        let east_5_30 = utc.with_timezone(&FixedOffset::east((5 * 3600) + (30 * 60)));
        let west_8 = utc.with_timezone(&FixedOffset::west(8 * 3600));
        let west_8_30 = utc.with_timezone(&FixedOffset::west((8 * 3600) + (30 * 60)));

        let times = [
            (1, utc_tz),
            (2, east_5),
            (3, east_5_30),
            (4, west_8),
            (5, west_8_30),
        ];

        for time in times.iter() {
            if let Err(err) = insert.bind(&[&time.0, &time.1]) {
                panic!("Cannot bind for insert to Times: {}", err)
            }

            if let Err(err) = insert.execute() {
                panic!(
                    "Couldn't insert id {} tz {} into Times: {}",
                    time.0, time.1, err
                )
            }
        }

        let sql_select = "SELECT WorldTime FROM Times
                          WHERE TimeId = :id";

        let mut select = match conn.create_prepared_statement(sql_select) {
            Ok(stmt) => stmt,
            Err(err) => panic!("Couldn't create select for Times: {}", err),
        };

        for time in times.iter() {
            if let Err(err) = select.bind(&[&time.0]) {
                panic!("Cannot bind for select for Times: {}", err)
            }

            if let Err(err) = select.execute() {
                panic!("Couldn't execute select for Times: {}", err)
            }

            let result_set = match select.result_set() {
                Ok(res) => res,
                Err(err) => panic!("{}", err),
            };
            if result_set.is_empty() {
                panic!("Should not have an empty result")
            }

            let first_row = &result_set[0];
            let timestamp_tz: DateTime<FixedOffset> = first_row[0].value().unwrap();
            assert_eq!(time.1, timestamp_tz);
        }
    }

    /// Testing various data conversions
    ///
    #[test]
    fn conversions() {
        let conn = match Connection::new(CONNECTION, USER, PASSWORD) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to create a connection: {}", err),
        };
        let sql_drop = "DROP TABLE Films";
        let mut drop = match conn.create_prepared_statement(sql_drop) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        drop.execute().ok();
        let sql_create = "CREATE TABLE Films(FilmId INTEGER,
                                             Name VARCHAR2(200),
                                             Genre CHAR(10),
                                             Released DATE,
                                             LastUpdate TIMESTAMP(9),
                                             LastViewed TIMESTAMP(9) WITH TIME ZONE)";
        let mut create = match conn.create_prepared_statement(sql_create) {
            Ok(stmt) => stmt,
            Err(err) => panic!("{}", err),
        };
        if let Err(err) = create.execute() {
            panic!("Couldn't execute create Films: {}", err)
        }

        let sql_insert = "INSERT INTO Films(FilmId, Name, Genre, Released, LastUpdate, LastViewed)
                          VALUES(:id, :name, :genre, :released, :updated, :viewed)";

        let mut insert = match conn.create_prepared_statement(sql_insert) {
            Ok(stmt) => stmt,
            Err(err) => panic!("Cannot create insert for Films: {}", err),
        };

        let id = 1;
        let name = "Guardians of the Galaxy";
        let genre = "Sci-fi    ";
        let released = Utc.ymd(2014, 7, 21);
        let updated = Utc::now();
        let viewed = updated.with_timezone(&FixedOffset::east((5 * 3600) + 1800));
        println!("viewed {}", viewed.hour());

        if let Err(err) = insert.bind(&[&id, &name, &genre, &released, &updated, &viewed]) {
            panic!("Cannot bind for insert to Films: {}", err)
        }

        if let Err(err) = insert.execute() {
            panic!("Couldn't execute insert into Films: {}", err)
        }

        let sql_select = "SELECT * FROM Films";

        let mut select = match conn.create_prepared_statement(sql_select) {
            Ok(stmt) => stmt,
            Err(err) => panic!("Couldn't create select for Films: {}", err),
        };

        if let Err(err) = select.execute() {
            panic!("Couldn't execute select for Films: {}", err)
        }

        let result_set = match select.result_set() {
            Ok(res) => res,
            Err(err) => panic!("{}", err),
        };
        if result_set.is_empty() {
            panic!("Should not have an empty result")
        }

        let first_row = &result_set[0];

        let genre_char: String = first_row[2].value().unwrap();
        assert_eq!(genre_char, genre);

        let date: Date<Utc> = first_row[3].value().unwrap();
        assert_eq!(date, released);

        let date_as_string: String = first_row[3].value().unwrap();
        assert_eq!(date_as_string, released.to_string());

        let timestamp: DateTime<Utc> = first_row[4].value().unwrap();
        assert_eq!(timestamp, updated);

        let timestamp_as_string: String = first_row[4].value().unwrap();
        assert_eq!(timestamp_as_string, updated.to_string());

        let timestamp_tz: DateTime<FixedOffset> = first_row[5].value().unwrap();
        assert_eq!(timestamp_tz, viewed);

        let timestamp_tz_as_string: String = first_row[5].value().unwrap();
        assert_eq!(timestamp_tz_as_string, viewed.to_string());
    }
}
