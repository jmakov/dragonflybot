use std::fmt;

use error_stack::Context;


#[derive(Debug)]
pub struct Error;
#[derive(Debug)]
pub struct ClientError;
#[derive(Debug)]
pub struct ListenerError;
#[derive(Debug)]
pub struct ListenerAggregatorError;
#[derive(Debug)]
pub struct SubscriberError;

impl Context for Error {}
impl Context for ClientError {}
impl Context for ListenerError {}
impl Context for ListenerAggregatorError {}
impl Context for SubscriberError {}

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
impl fmt::Display for ListenerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MainError")
    }
}
impl fmt::Display for ListenerAggregatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ListenerAggregatorError")
    }
}
impl fmt::Display for SubscriberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SubscriberError")
    }
}