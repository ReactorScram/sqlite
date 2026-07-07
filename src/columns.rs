use std::ffi::{c_double, c_int};

/// Bindable and readable types
use crate::{ColumnIndex, ParameterIndex, Result, Statement, Type, Value};

// https://sqlite.org/c3ref/c_static.html
macro_rules! transient(
    () => (
        std::mem::transmute::<
        *const std::ffi::c_void,
        std::option::Option<unsafe extern "C" fn(*mut std::ffi::c_void)>
        >(!0 as *const core::ffi::c_void)
    );
);

/// A type suitable for binding to a prepared statement.
pub trait Bindable {
    /// Bind to a parameter.
    fn bind(self, _: &mut Statement) -> Result<()>;
}

/// A type suitable for binding to a prepared statement given a parameter index.
pub trait BindableWithIndex {
    /// Bind to a parameter.
    ///
    /// In case of integer indices, the first parameter has index 1.
    fn bind<T: ParameterIndex>(self, _: &mut Statement, _: T) -> Result<()>;
}

/// A type suitable for reading from a prepared statement given a column index.
pub trait ReadableWithIndex: Sized {
    /// Read from a column.
    ///
    /// In case of integer indices, the first column has index 0.
    fn read<T: ColumnIndex>(_: &Statement, _: T) -> Result<Self>;
}

impl<T, U> Bindable for (T, U)
where
    T: ParameterIndex,
    U: BindableWithIndex,
{
    #[inline]
    fn bind(self, statement: &mut Statement) -> Result<()> {
        self.1.bind(statement, self.0)
    }
}

impl<T> Bindable for &[T]
where
    T: BindableWithIndex + Clone,
{
    fn bind(self, statement: &mut Statement) -> Result<()> {
        for (index, value) in self.iter().enumerate() {
            value.clone().bind(statement, index + 1)?;
        }
        Ok(())
    }
}

impl<T, U> Bindable for &[(T, U)]
where
    T: ParameterIndex,
    U: BindableWithIndex + Clone,
{
    fn bind(self, statement: &mut Statement) -> Result<()> {
        for (index, value) in self.iter() {
            value.clone().bind(statement, *index)?;
        }
        Ok(())
    }
}

impl BindableWithIndex for &[u8] {
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        unsafe {
            ok!(
                statement.raw.1,
                ffi::sqlite3_bind_blob(
                    statement.raw.0,
                    index.index(statement)? as c_int,
                    self.as_ptr() as *const _,
                    self.len() as c_int,
                    transient!(),
                )
            );
        }
        Ok(())
    }
}

impl BindableWithIndex for f64 {
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        unsafe {
            ok!(
                statement.raw.1,
                ffi::sqlite3_bind_double(
                    statement.raw.0,
                    index.index(statement)? as c_int,
                    self as c_double
                )
            );
        }
        Ok(())
    }
}

impl BindableWithIndex for i64 {
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        unsafe {
            ok!(
                statement.raw.1,
                ffi::sqlite3_bind_int64(
                    statement.raw.0,
                    index.index(statement)? as c_int,
                    self as ffi::sqlite3_int64
                )
            );
        }
        Ok(())
    }
}

impl BindableWithIndex for &str {
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        unsafe {
            ok!(
                statement.raw.1,
                ffi::sqlite3_bind_text(
                    statement.raw.0,
                    index.index(statement)? as c_int,
                    self.as_ptr() as *const _,
                    self.len() as c_int,
                    transient!(),
                )
            );
        }
        Ok(())
    }
}

impl BindableWithIndex for () {
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        unsafe {
            ok!(
                statement.raw.1,
                ffi::sqlite3_bind_null(statement.raw.0, index.index(statement)? as c_int)
            );
        }
        Ok(())
    }
}

impl BindableWithIndex for Value {
    #[inline]
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        (index, &self).bind(statement)
    }
}

impl BindableWithIndex for &Value {
    fn bind<T: ParameterIndex>(self, statement: &mut Statement, index: T) -> Result<()> {
        match self {
            Value::Binary(ref value) => (value as &[u8]).bind(statement, index),
            Value::Float(value) => value.bind(statement, index),
            Value::Integer(value) => value.bind(statement, index),
            Value::String(ref value) => (value as &str).bind(statement, index),
            Value::Null => ().bind(statement, index),
        }
    }
}

impl<T> BindableWithIndex for Option<T>
where
    T: BindableWithIndex,
{
    #[inline]
    fn bind<U: ParameterIndex>(self, statement: &mut Statement, index: U) -> Result<()> {
        match self {
            Some(value) => value.bind(statement, index),
            None => ().bind(statement, index),
        }
    }
}

impl<T> BindableWithIndex for &Option<T>
where
    T: BindableWithIndex + Clone,
{
    #[inline]
    fn bind<U: ParameterIndex>(self, statement: &mut Statement, index: U) -> Result<()> {
        match self {
            Some(value) => value.clone().bind(statement, index),
            None => ().bind(statement, index),
        }
    }
}

impl ReadableWithIndex for Vec<u8> {
    fn read<T: ColumnIndex>(statement: &Statement, index: T) -> Result<Self> {
        use std::ptr::copy_nonoverlapping as copy;
        unsafe {
            let pointer =
                ffi::sqlite3_column_blob(statement.raw.0, index.index(statement)? as c_int);
            if pointer.is_null() {
                return Ok(vec![]);
            }
            let count = ffi::sqlite3_column_bytes(statement.raw.0, index.index(statement)? as c_int)
                as usize;
            let mut buffer = Vec::with_capacity(count);
            copy(pointer as *const u8, buffer.as_mut_ptr(), count);
            buffer.set_len(count);
            Ok(buffer)
        }
    }
}

impl ReadableWithIndex for f64 {
    #[allow(clippy::unnecessary_cast)]
    fn read<T: ColumnIndex>(statement: &Statement, index: T) -> Result<Self> {
        Ok(unsafe {
            ffi::sqlite3_column_double(statement.raw.0, index.index(statement)? as c_int) as f64
        })
    }
}

impl ReadableWithIndex for i64 {
    #[allow(clippy::unnecessary_cast)]
    fn read<T: ColumnIndex>(statement: &Statement, index: T) -> Result<Self> {
        Ok(unsafe {
            ffi::sqlite3_column_int64(statement.as_raw(), index.index(statement)? as c_int) as i64
        })
    }
}

impl ReadableWithIndex for String {
    fn read<T: ColumnIndex>(statement: &Statement, index: T) -> Result<Self> {
        unsafe {
            let pointer =
                ffi::sqlite3_column_text(statement.as_raw(), index.index(statement)? as c_int);
            if pointer.is_null() {
                raise!("cannot read a text column");
            }
            Ok(c_str_to_string!(pointer))
        }
    }
}

impl ReadableWithIndex for Value {
    fn read<T: ColumnIndex>(statement: &Statement, index: T) -> Result<Self> {
        Ok(match statement.column_type(index)? {
            Type::Binary => Value::Binary(ReadableWithIndex::read(statement, index)?),
            Type::Float => Value::Float(ReadableWithIndex::read(statement, index)?),
            Type::Integer => Value::Integer(ReadableWithIndex::read(statement, index)?),
            Type::String => Value::String(ReadableWithIndex::read(statement, index)?),
            Type::Null => Value::Null,
        })
    }
}

impl<T: ReadableWithIndex> ReadableWithIndex for Option<T> {
    fn read<U: ColumnIndex>(statement: &Statement, index: U) -> Result<Self> {
        if statement.column_type(index)? == Type::Null {
            Ok(None)
        } else {
            T::read(statement, index).map(Some)
        }
    }
}
