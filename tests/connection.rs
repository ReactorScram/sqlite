use sql_peas::{Connection, OpenFlags, State};

mod common;

use common::setup_users;

macro_rules! ok(($result:expr) => ($result.unwrap()));

#[test]
fn open_with_flags() {
    use temporary::Folder;

    let path = ok!(Folder::new("sqlite"));
    let path = path.path().join("database.sqlite3");
    setup_users(&path);

    let flags = OpenFlags::new().with_read_only();
    let connection = ok!(Connection::open_with_flags(path, flags));
    match connection.execute("INSERT INTO users VALUES (2, 'Bob', NULL, NULL)") {
        Err(_) => {}
        _ => unreachable!(),
    }
}

/*
// FIXME - Send / Sync
#[tokio::test]
async fn open_thread_safe_async() {
    use std::sync::Arc;

    use tokio::task::spawn_blocking as spawn;

    let connection = Arc::new(ok!(ok!(
        spawn(|| Connection::open_thread_safe(":memory:")).await
    )));

    {
        let connection = connection.clone();
        ok!(ok!(spawn(move || connection.execute("SELECT 1")).await));
    }

    {
        let connection = connection.clone();
        ok!(ok!(spawn(move || connection.execute("SELECT 1")).await));
    }
}
*/

/*
// FIXME - Send / Sync
#[test]
fn open_thread_safe_sync() {
    use std::sync::Arc;
    use std::thread::spawn;

    let connection = Arc::new(ok!(Connection::open_thread_safe(":memory:")));

    let mut threads = Vec::new();
    for _ in 0..5 {
        let connection = connection.clone();
        threads.push(spawn(move || {
            ok!(connection.execute("SELECT 1"));
        }));
    }
    for thread in threads {
        ok!(thread.join());
    }
}
*/

#[test]
fn execute() {
    let connection = setup_users(":memory:");
    match connection.execute(":)") {
        Err(error) => assert_eq!(
            error.message,
            Some(String::from(r#"unrecognized token: ":""#))
        ),
        _ => unreachable!(),
    }
}

#[test]
fn iterate() {
    macro_rules! pair(
        ($one:expr, $two:expr) => (($one, Some($two)));
    );

    let connection = setup_users(":memory:");

    let mut done = false;
    let query = "SELECT * FROM users";
    ok!(connection.iterate(query, |pairs| {
        assert_eq!(pairs.len(), 5);
        assert_eq!(pairs[0], pair!("id", "1"));
        assert_eq!(pairs[1], pair!("name", "Alice"));
        assert_eq!(pairs[2], pair!("age", "42.69"));
        assert_eq!(pairs[3], pair!("photo", "\x42\x69"));
        assert_eq!(pairs[4], ("email", None));
        done = true;
        true
    }));
    assert!(done);
}

#[test]
fn set_busy_handler() {
    use std::thread::spawn;
    use temporary::Folder;

    let path = ok!(Folder::new("sqlite"));
    let path = path.path().join("database.sqlite3");
    setup_users(&path);

    let guards = (0..10)
        .map(|_| {
            let path = path.to_path_buf();
            spawn(move || {
                let mut connection = ok!(sql_peas::open(&path));
                ok!(connection.set_busy_handler(|_| true));
                let query = "INSERT INTO users VALUES (?, ?, ?, ?, ?)";
                let handle = ok!(connection.prepare(query));
                let statement = connection.borrow_statement(handle).unwrap();
                ok!(statement.bind((1, 2i64)));
                ok!(statement.bind((2, "Bob")));
                ok!(statement.bind((3, 69.42)));
                ok!(statement.bind((4, &[0x69u8, 0x42u8][..])));
                ok!(statement.bind((5, ())));
                assert_eq!(ok!(statement.next()), State::Done);
                true
            })
        })
        .collect::<Vec<_>>();

    for guard in guards {
        assert!(ok!(guard.join()));
    }
}

#[cfg(feature = "extension")]
#[test]
fn enable_extension() {
    let connection = ok!(Connection::open(":memory:"));
    ok!(connection.enable_extension());
}

#[cfg(feature = "extension")]
#[test]
fn disable_extension() {
    let connection = ok!(Connection::open(":memory:"));
    ok!(connection.disable_extension());
}

#[cfg(feature = "extension")]
#[test]
fn load_extension() {
    let connection = ok!(Connection::open(":memory:"));
    ok!(connection.enable_extension());
    assert!(connection.load_extension("libsqlitefunctions").is_err());
}

#[test]
fn change_count() {
    let connection = setup_users(":memory:");
    assert_eq!(connection.change_count(), 1);
    assert_eq!(connection.total_change_count(), 1);

    ok!(connection.execute("INSERT INTO users VALUES (2, 'Bob', NULL, NULL, NULL)"));
    assert_eq!(connection.change_count(), 1);
    assert_eq!(connection.total_change_count(), 2);

    ok!(connection.execute("UPDATE users SET name = 'Bob' WHERE id = 1"));
    assert_eq!(connection.change_count(), 1);
    assert_eq!(connection.total_change_count(), 3);

    ok!(connection.execute("DELETE FROM users"));
    assert_eq!(connection.change_count(), 2);
    assert_eq!(connection.total_change_count(), 5);
}

#[test]
fn transactions() {
    let mut connection = setup_users(":memory:");

    let handle = ok!(connection.prepare("SELECT name FROM users WHERE id = 1"));
    let get_stmt = ok!(connection.borrow_statement(handle));

    ok!(get_stmt.reset());
    assert_eq!(ok!(get_stmt.next()), sql_peas::State::Row);
    assert_eq!(ok!(get_stmt.read::<String, _>(0)), "Alice");

    // Dropping a transaction rolls it back
    {
        let _tx = connection.begin_immediate_transaction().unwrap();
        connection
            .execute("UPDATE users SET name = 'Bob' where id = 1")
            .unwrap();
    }

    ok!(get_stmt.reset());
    assert_eq!(ok!(get_stmt.next()), sql_peas::State::Row);
    assert_eq!(ok!(get_stmt.read::<String, _>(0)), "Alice");

    // Committing a transaction commits it
    {
        let tx = connection.begin_immediate_transaction().unwrap();
        connection
            .execute("UPDATE users SET name = 'Bob' where id = 1")
            .unwrap();
        tx.commit().unwrap();
    }

    // Congrats on your transaction, Bob
    ok!(get_stmt.reset());
    assert_eq!(ok!(get_stmt.next()), sql_peas::State::Row);
    assert_eq!(ok!(get_stmt.read::<String, _>(0)), "Bob");
}
