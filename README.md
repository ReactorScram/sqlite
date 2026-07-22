# sqlite-peas

The package provides an interface to [SQLite].

(This is a fork of https://github.com/stainless-steel/sqlite with different API ergonomics.)

## Example

Open a connection, create a table, and insert a few rows:

```rust
let connection = sqlite::open(":memory:").unwrap();

let query = "
    CREATE TABLE users (name TEXT, age INTEGER);
    INSERT INTO users VALUES ('Alice', 42);
    INSERT INTO users VALUES ('Bob', 69);
";
connection.execute(query).unwrap();
```

Select some rows and process them one by one as plain text, which is generally
not efficient:

```rust
let query = "SELECT * FROM users WHERE age > 50";

connection
    .iterate(query, |pairs| {
        for &(name, value) in pairs.iter() {
            println!("{} = {}", name, value.unwrap());
        }
        true
    })
    .unwrap();
```

Run the same query but using a prepared statement, which is much more efficient
than the previous technique:

```rust
use sqlite::State;

let query = "SELECT * FROM users WHERE age > ?";
let mut statement = connection.prepare(query).unwrap();
statement.bind((1, 50)).unwrap();

while let Ok(State::Row) = statement.next() {
    println!("name = {}", statement.read::<String, _>("name").unwrap());
    println!("age = {}", statement.read::<i64, _>("age").unwrap());
}
```

Run the same query but using a cursor, which is iterable:

```rust
let query = "SELECT * FROM users WHERE age > ?";

for row in connection
    .prepare(query)
    .unwrap()
    .into_iter()
    .bind((1, 50))
    .unwrap()
    .map(|row| row.unwrap())
{
    println!("name = {}", row.read::<&str, _>("name"));
    println!("age = {}", row.read::<i64, _>("age"));
}
```

[SQLite]: https://www.sqlite.org
