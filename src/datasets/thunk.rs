use std::prelude::v1::*;

use crate::error::PondError;

pub type Thunk<T> = Box<dyn FnOnce() -> Result<T, PondError> + Send>;

pub trait IntoThunk<T> {
    fn into_thunk(self) -> Thunk<T>;
}

impl<T: Send + 'static> IntoThunk<T> for T {
    fn into_thunk(self) -> Thunk<T> {
        Box::new(move || Ok(self))
    }
}

impl<T: Send + 'static, E: Into<PondError> + Send + 'static> IntoThunk<T>
    for Box<dyn FnOnce() -> Result<T, E> + Send>
{
    fn into_thunk(self) -> Thunk<T> {
        Box::new(move || self().map_err(Into::into))
    }
}

pub trait FromThunk<T>: Sized {
    fn from_thunk(thunk: Thunk<T>) -> Result<Self, PondError>;
}

impl<T: Send + 'static> FromThunk<T> for T {
    fn from_thunk(thunk: Thunk<T>) -> Result<Self, PondError> {
        thunk()
    }
}

impl<T: Send + 'static, E: From<PondError> + Send + 'static> FromThunk<T>
    for Box<dyn FnOnce() -> Result<T, E> + Send>
{
    fn from_thunk(thunk: Thunk<T>) -> Result<Self, PondError> {
        Ok(Box::new(move || thunk().map_err(Into::into)))
    }
}
