use std::borrow::Cow;

use crate::{request::Request, response::Response};

pub enum PathParameter {
    Exact(String),
    Begin(String),
    End(String),
    Contains(String),
    Wildcard,
}

pub enum MiddlewareType {
    PreRequest(PathParameter),
    PostRequest(PathParameter),
    ErrorHandler(PathParameter),
}

pub enum MiddlewareHandler<'a> {
    PreRequest(fn(&mut Request) -> MiddlewareResult),
    PostRequest(fn(&Request, &'a mut Response) -> MiddlewareResult<'a>),
    ErrorHandler(fn (&Request, &'a mut Response) -> MiddlewareResult<'a>),
}

pub enum MiddlewareResult<'a> {
    NextMiddleware,
    SkipMiddlewares,
    SendResponseAndStopProcessing(Cow<'a, Response>)
}

pub struct MiddlewareEntry<'a> {
    pub middleware_type: MiddlewareType,
    pub handler: MiddlewareHandler<'a>,
}


pub trait HttpMiddleware<'a> {
    
    fn add_middleware(&mut self, middleware_type: MiddlewareType, handler: MiddlewareHandler<'a>);
    
    fn pre_request(&mut self, path: PathParameter, handler: fn(&mut Request) -> MiddlewareResult<'_>) {
        self.add_middleware(MiddlewareType::PreRequest(path), MiddlewareHandler::PreRequest(handler));
    }
    
    fn post_request(&mut self, path: PathParameter, handler: fn(request: &Request, response: &'a mut Response) -> MiddlewareResult<'a>) {
        self.add_middleware(MiddlewareType::PostRequest(path), MiddlewareHandler::PostRequest(handler));
    }
    
    fn error_handler(&mut self, path: PathParameter, handler: fn(request: &Request, error: &'a mut Response) -> MiddlewareResult<'a>) {
        self.add_middleware(MiddlewareType::ErrorHandler(path), MiddlewareHandler::ErrorHandler(handler));
    }
}
