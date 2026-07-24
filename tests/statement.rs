use sql_peas::{Connection, State, Type, Value};

mod common;

use common::{setup_english, setup_users};

macro_rules! ok(($result:expr) => ($result.unwrap()));

#[test]
fn bind_with_index() {
    let mut connection = setup_users(":memory:");
    let query = "INSERT INTO users VALUES (?, ?, ?, ?, ?)";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    ok!(statement.reset());
    ok!(statement.bind(&[(1, 2i64)][..]));
    ok!(statement.bind((2, "Bob")));
    ok!(statement.bind((3, 69.42)));
    ok!(statement.bind((4, &[0x69u8, 0x42u8][..])));
    ok!(statement.bind((5, ())));
    assert_eq!(ok!(statement.next()), State::Done);

    ok!(statement.reset());
    ok!(statement.bind((1, Some(2i64))));
    ok!(statement.bind((2, Some("Bob"))));
    ok!(statement.bind((3, Some(69.42))));
    ok!(statement.bind((4, Some(&[0x69u8, 0x42u8][..]))));
    ok!(statement.bind((5, None::<&str>)));
    assert_eq!(ok!(statement.next()), State::Done);

    ok!(statement.reset());
    ok!(statement.bind(
        &[
            Value::Integer(2),
            Value::String("Bob".into()),
            Value::Float(69.42),
            Value::Binary([0x69u8, 0x42u8].to_vec()),
            Value::Null,
        ][..]
    ));
    assert_eq!(ok!(statement.next()), State::Done);

    ok!(statement.reset());
    ok!(statement.bind(
        &[
            Some(Value::Integer(2)),
            Some(Value::String("Bob".into())),
            Some(Value::Float(69.42)),
            Some(Value::Binary([0x69u8, 0x42u8].to_vec())),
            Some(Value::Null),
        ][..]
    ));
    assert_eq!(ok!(statement.next()), State::Done);

    ok!(statement.reset());
    ok!(statement.bind(
        &[
            (1, Value::Integer(2)),
            (2, Value::String("Bob".into())),
            (3, Value::Float(69.42)),
            (4, Value::Binary([0x69u8, 0x42u8].to_vec())),
            (5, Value::Null),
        ][..]
    ));
    assert_eq!(ok!(statement.next()), State::Done);

    // Dropping the `Connection` will also drop every `Statement` it owns, but to free up resources you can drop them manually.
    ok!(connection.drop_statement(handle));
}

#[test]
fn bind_with_name() {
    let mut connection = setup_users(":memory:");
    let query = "INSERT INTO users VALUES (:id, :name, :age, :photo, :email)";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    ok!(statement.reset());
    ok!(statement.bind(&[(":id", 2i64)][..]));
    ok!(statement.bind((":name", "Bob")));
    ok!(statement.bind((":age", 69.42)));
    ok!(statement.bind((":photo", &[0x69u8, 0x42u8][..])));
    ok!(statement.bind((":email", ())));
    assert_eq!(ok!(statement.next()), State::Done);

    ok!(statement.reset());
    assert!(statement.bind((":missing", 404)).is_err());

    ok!(statement.reset());
    ok!(statement.bind(
        &[
            (":id", Value::Integer(2)),
            (":name", Value::String("Bob".into())),
            (":age", Value::Float(69.42)),
            (":photo", Value::Binary([0x69u8, 0x42u8].to_vec())),
            (":email", Value::Null),
        ][..]
    ));
    assert_eq!(ok!(statement.next()), State::Done);
}

#[test]
fn count() {
    let mut connection = setup_english(":memory:");

    let query = "SELECT value FROM english WHERE value LIKE ?";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();
    ok!(statement.bind((1, "%type")));
    let mut count = 0;
    while let State::Row = ok!(statement.next()) {
        count += 1;
    }
    assert_eq!(count, 6);

    let query = "SELECT value FROM english WHERE value LIKE '%type'";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();
    let mut count = 0;
    while let State::Row = ok!(statement.next()) {
        count += 1;
    }
    assert_eq!(count, 6);
}

#[test]
fn read_with_index() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT * FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    assert_eq!(ok!(statement.next()), State::Row);
    assert_eq!(ok!(statement.read::<i64, _>(0)), 1);
    assert_eq!(ok!(statement.read::<String, _>(1)), String::from("Alice"));
    assert_eq!(ok!(statement.read::<f64, _>(2)), 42.69);
    assert_eq!(ok!(statement.read::<Vec<u8>, _>(3)), vec![0x42, 0x69]);
    assert_eq!(ok!(statement.read::<Value, _>(4)), Value::Null);
    assert_eq!(ok!(statement.next()), State::Done);
}

#[test]
fn read_with_index_and_option() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT * FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    assert_eq!(ok!(statement.next()), State::Row);
    assert_eq!(ok!(statement.read::<Option<i64>, _>(0)), Some(1));
    assert_eq!(
        ok!(statement.read::<Option<String>, _>(1)),
        Some(String::from("Alice"))
    );
    assert_eq!(ok!(statement.read::<Option<f64>, _>(2)), Some(42.69));
    assert_eq!(
        ok!(statement.read::<Option<Vec<u8>>, _>(3)),
        Some(vec![0x42, 0x69])
    );
    assert_eq!(ok!(statement.read::<Option<String>, _>(4)), None);
    assert_eq!(ok!(statement.next()), State::Done);
}

#[test]
fn read_with_name_and_option() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT * FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    assert_eq!(ok!(statement.next()), State::Row);
    assert_eq!(ok!(statement.read::<Option<i64>, _>("id")), Some(1));
    assert_eq!(
        ok!(statement.read::<Option<String>, _>("name")),
        Some(String::from("Alice"))
    );
    assert_eq!(ok!(statement.read::<Option<f64>, _>("age")), Some(42.69));
    assert_eq!(
        ok!(statement.read::<Option<Vec<u8>>, _>("photo")),
        Some(vec![0x42, 0x69])
    );
    assert_eq!(ok!(statement.read::<Option<String>, _>("email")), None);
    assert_eq!(ok!(statement.next()), State::Done);
}

#[test]
fn read_with_name() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT * FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    assert_eq!(ok!(statement.next()), State::Row);
    assert_eq!(ok!(statement.read::<i64, _>("id")), 1);
    assert_eq!(
        ok!(statement.read::<String, _>("name")),
        String::from("Alice")
    );
    assert_eq!(ok!(statement.read::<f64, _>("age")), 42.69);
    assert_eq!(ok!(statement.read::<Vec<u8>, _>("photo")), vec![0x42, 0x69]);
    assert_eq!(ok!(statement.read::<Value, _>("email")), Value::Null);
    assert_eq!(ok!(statement.next()), State::Done);
}

#[test]
fn column_count() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT * FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    assert_eq!(ok!(statement.next()), State::Row);
    assert_eq!(statement.column_count(), 5);
}

#[test]
fn column_name() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT id, name, age, photo AS user_photo FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    let names = statement.column_names();
    assert_eq!(names, vec!["id", "name", "age", "user_photo"]);
    assert_eq!("user_photo", ok!(statement.column_name(3)));
}

#[test]
fn column_type() {
    let mut connection = setup_users(":memory:");
    let query = "SELECT * FROM users";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();

    assert_eq!(ok!(statement.column_type(0)), Type::Null);
    assert_eq!(ok!(statement.column_type(1)), Type::Null);
    assert_eq!(ok!(statement.column_type(2)), Type::Null);
    assert_eq!(ok!(statement.column_type(3)), Type::Null);

    assert_eq!(ok!(statement.next()), State::Row);

    assert_eq!(ok!(statement.column_type(0)), Type::Integer);
    assert_eq!(ok!(statement.column_type(1)), Type::String);
    assert_eq!(ok!(statement.column_type(2)), Type::Float);
    assert_eq!(ok!(statement.column_type(3)), Type::Binary);
}

#[test]
fn parameter_index() {
    let mut connection = setup_users(":memory:");
    let query = "INSERT INTO users VALUES (:id, :name, :age, :photo, :email)";
    let handle = ok!(connection.prepare(query));
    let statement = connection.borrow_statement(handle).unwrap();
    ok!(statement.bind((":id", 2i64)));
    ok!(statement.bind((":name", "Bob")));
    ok!(statement.bind((":age", 69.42)));
    ok!(statement.bind((":photo", &[0x69u8, 0x42u8][..])));
    ok!(statement.bind((":email", ())));
    assert_eq!(ok!(statement.parameter_index(":missing")), None);
    assert_eq!(ok!(statement.next()), State::Done);
    ok!(connection.drop_statement(handle));
}

#[test]
fn workflow_1() {
    struct Database {
        #[allow(dead_code)]
        connection: Connection,
        handle: sql_peas::StatementHandle,
    }

    impl Database {
        fn run_once(&mut self) -> sql_peas::Result<()> {
            let statement = self.connection.borrow_statement(self.handle)?;
            statement.reset()?;
            statement.bind((":age", 40))?;
            assert_eq!(ok!(statement.next()), State::Row);
            Ok(())
        }
    }

    let mut connection = setup_users(":memory:");
    let query = "SELECT name FROM users WHERE age > :age";
    let handle = ok!(connection.prepare(query));

    let mut database = Database { connection, handle };

    for _ in 0..5 {
        assert!(database.run_once().is_ok());
    }
}

#[test]
fn workflow_2() {
    let mut connection = ok!(Connection::open(":memory:"));
    ok!(connection.execute("CREATE TABLE users (name TEXT, age INTEGER, PRIMARY KEY (name))"));

    let handle = ok!(connection.prepare(
        "INSERT INTO users (name, age) VALUES ('jean', 49) ON CONFLICT DO UPDATE SET age = 49"
    ));
    let statement = connection.borrow_statement(handle).unwrap();
    ok!(statement.next());
    ok!(connection.drop_statement(handle));

    let handle = ok!(connection.prepare(
        "INSERT INTO users (name, age) VALUES ('jean', 50) ON CONFLICT DO UPDATE SET age = 50"
    ));
    let statement = connection.borrow_statement(handle).unwrap();
    ok!(statement.next());
    ok!(connection.drop_statement(handle));

    let handle = ok!(connection.prepare("SELECT * FROM users WHERE name = 'jean'"));
    let statement = connection.borrow_statement(handle).unwrap();
    ok!(statement.next());

    let age = ok!(statement.read::<i64, _>("age"));
    assert_eq!(age, 50);
    ok!(connection.drop_statement(handle));
}
