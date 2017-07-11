extern crate libc;
extern crate byteorder;
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
