use std::fmt;

use error_stack::Context;


#[derive(Debug)]
pub struct Error;
#[derive(Debug)]
pub struct ClientError;
#[derive(Debug)]
pub struct TaskError;

impl Context for Error {}
impl Context for ClientError {}
impl Context for TaskError {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MainError")
    }
}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ClientError")
    }
}
impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TaskError")
    }
}
