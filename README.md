# oci_rs

oci_rs provides a Rust wrapper to the [Oracle Call Interface][1] (OCI) library.
The Oracle site describes OCI as a "...comprehensive, high performance, native C
language interface to Oracle Database for custom or packaged applications...".

This readme is lifted from the [crate documentation][12].

## Overview

The OCI library is the original Oracle C API for interacting with their database. It is the one
that later versions of JDBC is built on for example. Recently Oracle has released a new API
called the [Oracle Database Programming Interface for Drivers and Applications][2] (ODPI-C)
that is supposed to simplify use of OCI, however the documentation for OCI
is more extensive and therefore easier to build a wrapper on top of.

The OCI library is large and supports many use cases for interacting with a database. This
crate is currently concerned with support for executing SQL statements and so is limited when
compared to the whole of OCI.

The overall design will be familiar to anyone who has used Java's JDBC, Haskell's HDBC or
Rust's [postgres][3] crate. Indeed, most design decisions were
made based on reviewing the API of these libraries.

The basics are simple: a [`Connection`][4] represents a connection to a database, this connection
can be used to prepare one or more [`Statement`][5]s which are then used to execute SQL against the
database. If there are results then they can be returned all at once or lazily via an iterator.
Datatypes are represented using [`SqlValue`][6] and allow type conversion from Oracle
to Rust types.

### Missing type conversions

Currently only `String`, `i64` and `f64` are supported. In Oracle terms this means that anything
held in columns as `VARCHAR`, `VARCHAR2` and `Number` can be retrieved. As Oracle uses `Number` to
respresent all number types then this is less restricting that it first appears. More types
will be added.

## Setup

This crate is developed against version 12.2 of the OCI library. It is expected to work with
12.x.x but is not tested. The OCI client library needs to be installed on your machine and can be
downloaded [here][7].

If you are on Linux then you are likely to need to tell the linker where
to find the files. Adding this to my `.bashrc` file worked for me, however the details may vary
according to your distro, mine is [OpenSuse][8].

```text
export LIBRARY_PATH=$LIBRARY_PATH:/usr/lib/oracle/12.2/client64/lib/
```

This crate has not been tested against Windows and so the setup will be different.

Testing has been done against a local installation of [Oracle 11g Express Edition][9].
In order to run the crate tests then a local database needs to be
available on `localhost:1521/xe` with a user `oci_rs` and password `test`.

In order to use `oci_rs` add this to your `Cargo.toml`:

```toml
[dependencies]
oci_rs = "0.3.0"
```
and this to your crate root:

```rust
extern crate oci_rs;
```

## Examples

In the following example we will create a connection to a database and then create a table,
insert a couple of rows using bind variables and then execute a query to fetch them back again.
There is a lot of error handling needed. Every OCI function call can fail and so `Result` and
`Option` are used extensively. The below code takes the usual documentation shortcut of calling
`unwrap()` a lot but doing so in real client code will prove ill-fated. Any remote database connection is
inherently unreliable.

```rust
use oci_rs::connection::Connection;

let conn = Connection::new("localhost:1521/xe", "oci_rs", "test").unwrap();

# let mut drop = conn.create_prepared_statement("DROP TABLE Toys").unwrap();
# drop.execute().ok();

// Create a table
let sql_create = "CREATE TABLE Toys (ToyId int,
                                     Name varchar(20),
                                     Price float)";
let mut create = conn.create_prepared_statement(sql_create).unwrap();

// Execute the create statement
create.execute().unwrap();

// Commit in case we lose connection (an abnormal disconnection would result
// in an automatic roll-back.)
create.commit().unwrap();

// Insert some values using bind variables
let sql_insert = "INSERT INTO Toys (ToyId, Name, Price)
                  VALUES (:id, :name, :price)";
let mut insert = conn.create_prepared_statement(sql_insert).unwrap();

let values = [(1, "Barbie", 23.45),
              (2, "Dinosaurs", -5.21)];

// Run through the list of values, bind them and execute the statement
for value in values.iter() {
    insert.bind(&[&value.0, &value.1, &value.2]).unwrap();
    insert.execute().unwrap()
}

insert.commit().unwrap();

// Create a query
let sql_select = "SELECT * FROM Toys
                  WHERE Name='Barbie'";

let mut select = conn.create_prepared_statement(sql_select).unwrap();

// Execute
select.execute().unwrap();

// Get the result set
let result_set = select.result_set().unwrap();
assert_eq!(result_set.len(), 1);
let first_row = &result_set[0];

// Types are automatically converted
let id: i64 = first_row[0].value().unwrap();
let name: String = first_row[1].value().unwrap();
let price: f64 = first_row[2].value().unwrap();

assert_eq!(id, 1);
assert_eq!(name, "Barbie");
assert_eq!(price, 23.45);

```
## OCI docs

Documentation for the underlying OCI library can be found [here][10] and error codes and their
descriptions [here][11]. The error descriptions are useful because they often contain
additional information that is not included in the text returned from the library.

[1]: http://www.oracle.com/technetwork/database/features/oci/index-090945.html
[2]: https://github.com/oracle/odpi
[3]: https://crates.io/crates/postgres
[4]: connection/struct.Connection.html
[5]: statement/struct.Statement.html
[6]: types/enum.SqlValue.html
[7]: http://www.oracle.com/technetwork/database/features/instant-client/index-097480.html
[8]: https://www.opensuse.org/
[9]: http://www.oracle.com/technetwork/database/database-technologies/express-edition/overview/index.html
[10]: http://docs.oracle.com/database/122/LNOCI/toc.html
[11]: https://docs.oracle.com/database/122/ERRMG/toc.html
[12]: https://docs.rs/oci_rs/0.3.1/oci_rs/ 
