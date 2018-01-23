# oci_rs

oci_rs provides a Rust wrapper to the [Oracle Call Interface][1] (OCI) library.
The Oracle site describes OCI as a "...comprehensive, high performance, native C
language interface to Oracle Database for custom or packaged applications...".

Documentation is available [here][12].

## Setup

This crate is developed against version 12.2 of the OCI library. It is expected to work with
12.x.x but is not tested. The OCI client library needs to be installed on your machine and can be
downloaded [here][7].

If you are on Linux then you are likely to need to tell the linker where
to find the files. Adding this to my `.bashrc` file worked for me, however the details may vary
according to your distro. The below works on [OpenSuse][8].

```text
export LIBRARY_PATH=$LIBRARY_PATH:/usr/lib/oracle/12.2/client64/lib/
```

This crate has been briefly tested against Windows but difficulties were faced. The OCI library is named differently and so updates will be needed in the bindings to make it compile. Once I can get chance to work out how to even build this using Visual Studio on Windows, this will be addressed.

Testing has been done against a local installation of [Oracle 11g Express Edition][9].
In order to run the crate tests then a local database needs to be
available on `localhost:1521/xe` with a user `oci_rs` and password `test`.

Note that users of Debian based systems will face a lot of bother using Oracle databases locally. It does not install easily due to lack of official packages and the use of Alien will not help much. There are lots of complicated instructions available on the internet for how to get it to work, however the easiest is to run it in a [Docker container][14]. I have switched to [Ubuntu][13] and have had to resort to using Docker.

In order to use `oci_rs` add this to your `Cargo.toml`:

```toml
[dependencies]
oci_rs = "0.6.0"
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

[1]: http://www.oracle.com/technetwork/database/features/oci/index-090945.html
[2]: https://github.com/oracle/odpi
[3]: https://crates.io/crates/postgres
[4]: connection/struct.Connection.html
[5]: statement/struct.Statement.html
[6]: types/enum.SqlValue.html
[7]: http://www.oracle.com/technetwork/database/features/instant-client/index-097480.html
[8]: https://www.opensuse.org/
[9]: http://www.oracle.com/technetwork/database/database-technologies/express-edition/overview/index.html
[10]: http://docs.oracle.com/database/122/LNOCI/toc.htm
[11]: https://docs.oracle.com/database/122/ERRMG/toc.htm
[12]: https://docs.rs/oci_rs/0.3.1/oci_rs/
[13]: https://www.ubuntu.com/
[14]: https://github.com/wnameless/docker-oracle-xe-11g