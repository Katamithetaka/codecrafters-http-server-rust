use std::sync::Arc;

use crate::{http_method::HttpMethod, map::Map};


pub struct HttpListener<Request, Response> {
    pub(crate) path: String,
    pub(crate) method: HttpMethod,
    pub(crate) callback: Arc<dyn Fn(Request) -> Response + Send + Sync>,
}

impl<R1, R2> Clone for HttpListener<R1, R2> {
    fn clone(&self) -> Self {
        Self { path: self.path.clone(), method: self.method.clone(), callback: self.callback.clone() }
    }
}

pub trait HttpCallbacks {
    
    type Request;
    type Response;
    
    fn add_callback(&mut self, callback: HttpListener<Self::Request, Self::Response>);
    
    fn get<T: Into<String>>(
        &mut self,
        path: T,
        callback: impl Fn(Self::Request) -> Self::Response + Send + Sync + 'static,
    ) {
        self.add_callback(HttpListener {
            path: path.into(),
            method: HttpMethod::GET,
            callback: Arc::new(callback),
        });
    }

    fn all<T: Into<String>>(
        &mut self,
        path: T,
        callback: impl Fn(Self::Request) -> Self::Response + Send + Sync + 'static,
    ) {
        self.add_callback(HttpListener {
            path: path.into(),
            method: HttpMethod::ALL,
            callback: Arc::new(callback),
        });
    }

    fn post<T: Into<String>>(
        &mut self,
        path: T,
        callback: impl Fn(Self::Request) -> Self::Response + Send + Sync + 'static,
    ) {
        self.add_callback(HttpListener {
            path: path.into(),
            method: HttpMethod::POST,
            callback: Arc::new(callback),
        });
    }

    fn patch<T: Into<String>>(
        &mut self,
        path: T,
        callback: impl Fn(Self::Request) -> Self::Response + Send + Sync + 'static,
    ) {
        self.add_callback(HttpListener {
            path: path.into(),
            method: HttpMethod::PATCH,
            callback: Arc::new(callback),
        });
    }

    fn delete<T: Into<String>>(
        &mut self,
        path: T,
        callback: impl Fn(Self::Request) -> Self::Response + Send + Sync + 'static,
    ) {
        self.add_callback(HttpListener {
            path: path.into(),
            method: HttpMethod::DELETE,
            callback: Arc::new(callback),
        });
    }

    fn put<T: Into<String>>(
        &mut self,
        path: T,
        callback: impl Fn(Self::Request) -> Self::Response + Send + Sync + 'static,
    ) {
        self.add_callback(HttpListener {
            path: path.into(),
            method: HttpMethod::PUT,
            callback: Arc::new(callback),
        });
    }
}

pub(crate) fn get_path_params<Request, Response>(listener: &HttpListener<Request, Response>, path: &str) -> Map<String> {
    let mut params: Map<String> = Map::default();
    let registered_parts: Vec<&str> = listener.path.split('/').collect();
    let path_parts: Vec<&str> = path.split('/').collect();

    for (reg_part, path_part) in registered_parts.iter().zip(path_parts.iter()) {
        if reg_part.starts_with(":") {
            let key = reg_part.trim_start_matches(":").to_string();
            let value = path_part.to_string();
            params.add(&key, value);
        }
    }
    params
}

pub(crate) fn method_matches<Request, Response>(listener: &HttpListener<Request, Response>, method: &HttpMethod) -> bool {
    &listener.method == method || &listener.method == &HttpMethod::ALL
}

pub(crate) fn path_matches<Request, Response>(listener: &HttpListener<Request, Response>, path: &str) -> bool {
    let registered_path = &listener.path;
    if registered_path.contains(":") {
        let registered_parts: Vec<&str> = registered_path.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();
        if registered_parts.len() != path_parts.len() {
            return false;
        }
        for (reg_part, path_part) in registered_parts.iter().zip(path_parts.iter()) {
            if reg_part.starts_with(":") {
                continue;
            }
            if reg_part != path_part {
                return false;
            }
        }
        true
    } else {
        registered_path == path
    }
}
